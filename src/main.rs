//! CLI entry point for dapz — DAP compression proxy, MCP server, and daemon.
//!
//! ```bash
//! dapz proxy --backend "python3 -m debugpy.adapter"
//! dapz mcp    # requires --features mcp
//! dapz daemon # requires --features mcp — long-lived DAP session manager
//! ```

use std::process::ExitCode;
use std::str::FromStr;
use std::sync::Arc;

use clap::{Parser, Subcommand};
use dapz::interceptors::Interceptor;
use dapz::interceptors::InterceptorChain;
use dapz::interceptors::capping::CappingInterceptor;
use dapz::interceptors::evaluate::EvaluateCompressor;
use dapz::interceptors::output::OutputCompressor;
use dapz::interceptors::scopes::ScopesCompressor;
use dapz::interceptors::stacktrace::StackTraceCompressor;
use dapz::interceptors::variables::VariablesCompressor;
use dapz::metrics::{MeteredInterceptor, metrics_enabled_from_env};
use dapz::{CappingConfig, Config, OutputFormat, Proxy, StdioTransport, TcpTransport, Transport};
use tokio::sync::RwLock;
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(version, about = "AI-friendly DAP compression proxy")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Run as a transparent DAP compression proxy (default mode).
    #[command(name = "proxy", alias = "p")]
    Proxy(ProxyArgs),
    /// Run as an MCP server exposing debug tools (requires `--features mcp`).
    #[command(name = "mcp")]
    Mcp(McpArgs),
    /// Run a long-lived DAP session daemon (requires `--features mcp`).
    #[command(name = "daemon", alias = "d")]
    Daemon(DaemonArgs),

    /// Initialize Claude Code integration (MCP registration + context injection)
    #[command(name = "init")]
    Init {
        /// Add to global config (`~/.claude.json`) instead of project-local
        #[arg(short, long)]
        global: bool,

        /// Auto-patch settings without prompting (accepted for lspz CLI parity)
        #[arg(long = "auto-patch")]
        auto_patch: bool,

        /// Skip MCP config patching (print manual instructions)
        #[arg(long = "no-patch")]
        no_patch: bool,

        /// Show current dapz Claude Code configuration
        #[arg(long)]
        show: bool,

        /// Remove dapz artifacts from Claude Code settings
        #[arg(long)]
        uninstall: bool,

        /// Preview changes without writing any files
        #[arg(long = "dry-run")]
        dry_run: bool,

        /// Force overwrite even if files are already up to date
        #[arg(short, long)]
        force: bool,
    },
}

/// Arguments for `dapz daemon`.
#[derive(Parser, Debug)]
struct DaemonArgs {
    /// Project cwd used to derive the Unix socket path (DAP, not LSP roots).
    #[arg(long, env = "DAPZ_DAEMON_CWD", default_value = ".")]
    cwd: String,

    /// Explicit socket path (overrides cwd-derived path).
    #[arg(long, env = "DAPZ_DAEMON_SOCKET")]
    socket: Option<String>,

    /// Log level
    #[arg(short, long, env = "DAPZ_LOG_LEVEL", default_value = "info")]
    log_level: String,

    #[command(subcommand)]
    command: Option<DaemonCommand>,
}

/// Subcommands for `dapz daemon`.
#[derive(Subcommand, Debug)]
enum DaemonCommand {
    /// Print daemon status JSON for a cwd/socket.
    List {
        /// Emit TOON instead of JSON.
        #[arg(long)]
        toon: bool,
    },
}

/// Arguments for `dapz proxy`.
#[derive(Parser, Debug)]
struct ProxyArgs {
    /// Backend DAP server command (e.g. "python3 -m debugpy.adapter")
    #[arg(short, long, env = "DAPZ_BACKEND_CMD")]
    backend: String,

    /// Arguments to pass through to the backend DAP server
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    backend_args: Vec<String>,

    /// Log level (trace, debug, info, warn, error)
    #[arg(short, long, env = "DAPZ_LOG_LEVEL", default_value = "info")]
    log_level: String,

    /// Maximum number of stack frames to keep (0 = unlimited)
    #[arg(long, env = "DAPZ_MAX_FRAMES", default_value_t = 0)]
    max_frames: usize,

    /// Maximum number of variables per scope (0 = unlimited)
    #[arg(long, env = "DAPZ_MAX_VARIABLES", default_value_t = 0)]
    max_variables: usize,

    /// Maximum output event text length in chars (0 = unlimited)
    #[arg(long, env = "DAPZ_MAX_OUTPUT_LENGTH", default_value_t = 0)]
    max_output_length: usize,

    /// Enable output event compression (default: true)
    #[arg(
        long = "compress-output",
        env = "DAPZ_ENABLE_OUTPUT_COMPRESS",
        default_value_t = true
    )]
    compress_output: bool,

    /// Enable variables response compression (default: true)
    #[arg(
        long = "compress-variables",
        env = "DAPZ_ENABLE_VARIABLES_COMPRESS",
        default_value_t = true
    )]
    compress_variables: bool,

    /// Enable stackTrace response compression (default: true)
    #[arg(
        long = "compress-stacktrace",
        env = "DAPZ_ENABLE_STACKTRACE_COMPRESS",
        default_value_t = true
    )]
    compress_stacktrace: bool,

    /// Enable evaluate response compression (default: true)
    #[arg(
        long = "compress-evaluate",
        env = "DAPZ_ENABLE_EVALUATE_COMPRESS",
        default_value_t = true
    )]
    compress_evaluate: bool,

    /// Enable scopes response compression (default: true)
    #[arg(
        long = "compress-scopes",
        env = "DAPZ_ENABLE_SCOPES_COMPRESS",
        default_value_t = true
    )]
    compress_scopes: bool,

    /// Maximum evaluate result string length in chars (0 = unlimited)
    #[arg(long, env = "DAPZ_MAX_EVALUATE_LENGTH", default_value_t = 500)]
    max_evaluate_length: usize,

    /// Maximum variable value string length in chars (0 = unlimited)
    #[arg(long, env = "DAPZ_MAX_VALUE_LENGTH", default_value_t = 120)]
    max_value_length: usize,

    /// Output format: toon (default), json, or passthrough
    #[arg(short, long, env = "DAPZ_OUTPUT_FORMAT", default_value = "toon")]
    output: String,

    /// Enable runtime metrics on the interceptor chain (`DAPZ_METRICS` also works)
    #[arg(long, default_value_t = false)]
    metrics: bool,

    /// Backend transport: `stdio` (default) or `tcp://host:port`
    #[arg(long, default_value = "stdio")]
    transport: String,
}

/// Arguments for `dapz mcp`.
#[derive(Parser, Debug)]
struct McpArgs {
    /// Log level
    #[arg(short, long, env = "DAPZ_LOG_LEVEL", default_value = "info")]
    log_level: String,

    /// Use in-process adapters instead of auto-connecting to `dapz daemon`.
    #[arg(long)]
    no_daemon: bool,

    /// Project cwd for daemon socket identity (default: current directory).
    #[arg(long, env = "DAPZ_DAEMON_CWD")]
    cwd: Option<String>,
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Commands::Proxy(args) => run_proxy(args).await,
        Commands::Mcp(args) => run_mcp(args).await,
        Commands::Daemon(args) => run_daemon(args).await,
        Commands::Init {
            global,
            auto_patch,
            no_patch,
            show,
            uninstall,
            dry_run,
            force,
        } => dapz::init::run(
            global, auto_patch, no_patch, show, uninstall, dry_run, force,
        ),
    }
}

async fn run_proxy(args: ProxyArgs) -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::builder().parse_lossy(&args.log_level))
        .with_target(false)
        .init();

    let output_format = match OutputFormat::from_str(&args.output) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };

    let config = match build_config(&args, output_format) {
        Ok(c) => c,
        Err(code) => return code,
    };

    let shared_config = Arc::new(RwLock::new(config));

    let backend_cmd = shared_config.blocking_read().backend_cmd.clone();
    let transport: Box<dyn Transport> =
        match create_transport(&args.transport, &backend_cmd, &args.backend_args).await {
            Ok(t) => t,
            Err(code) => return code,
        };

    let interceptor_chain = build_interceptor_chain(&shared_config, args.metrics);

    let mut proxy = Proxy::new(shared_config, transport, interceptor_chain);

    if let Err(e) = proxy.start().await {
        tracing::error!(error = %e, "Proxy exited with error");
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

async fn run_mcp(args: McpArgs) -> ExitCode {
    #[cfg(feature = "mcp")]
    {
        // Keep tracing off stdio so MCP JSON-RPC is not polluted.
        let log_path = std::env::temp_dir().join("dapz-mcp.log");
        let log_file = match std::fs::File::create(&log_path) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("dapz: cannot create {}: {e}", log_path.display());
                return ExitCode::FAILURE;
            }
        };
        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::builder().parse_lossy(&args.log_level))
            .with_writer(log_file)
            .with_target(false)
            .init();

        use rmcp::ServiceExt;
        use rmcp::transport::stdio;

        if args.no_daemon {
            use dapz::mcp::McpServer;
            tracing::info!("Starting dapz MCP server (in-process, --no-daemon)");
            let server = McpServer::new();
            match server.serve(stdio()).await {
                Ok(running) => {
                    if let Err(e) = running.waiting().await {
                        tracing::error!(error = %e, "MCP server stopped with error");
                        return ExitCode::FAILURE;
                    }
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("Failed to start MCP server: {e}");
                    ExitCode::FAILURE
                }
            }
        } else {
            use dapz::mcp::DaemonMcpServer;
            let cwd = args.cwd.unwrap_or_else(|| {
                std::env::current_dir()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|_| ".".into())
            });
            tracing::info!(%cwd, "Starting dapz MCP server (daemon mode)");
            let server = DaemonMcpServer::new(cwd);
            match server.serve(stdio()).await {
                Ok(running) => {
                    if let Err(e) = running.waiting().await {
                        tracing::error!(error = %e, "MCP server stopped with error");
                        return ExitCode::FAILURE;
                    }
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("Failed to start MCP server: {e}");
                    ExitCode::FAILURE
                }
            }
        }
    }

    #[cfg(not(feature = "mcp"))]
    {
        let _ = args;
        eprintln!(
            "MCP support is not enabled.\n\
             Rebuild with `cargo build --features mcp` to enable MCP support."
        );
        ExitCode::FAILURE
    }
}

async fn run_daemon(args: DaemonArgs) -> ExitCode {
    #[cfg(feature = "mcp")]
    {
        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::builder().parse_lossy(&args.log_level))
            .with_target(false)
            .init();

        use dapz::daemon::{DaemonClient, DaemonServer, resolve_project_cwd, socket_path_for_cwd};

        let cwd = resolve_project_cwd(&args.cwd);
        let socket = args
            .socket
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| socket_path_for_cwd(&cwd));

        match args.command {
            Some(DaemonCommand::List { toon }) => {
                match DaemonClient::connect_or_start(&cwd).await {
                    Ok(mut client) => match client.status().await {
                        Ok(status) => {
                            if toon {
                                match dapz::value_to_toon(&status) {
                                    Ok(t) => println!("{t}"),
                                    Err(e) => {
                                        eprintln!("{e}");
                                        return ExitCode::FAILURE;
                                    }
                                }
                            } else {
                                println!(
                                    "{}",
                                    serde_json::to_string_pretty(&status).unwrap_or_default()
                                );
                            }
                            ExitCode::SUCCESS
                        }
                        Err(e) => {
                            eprintln!("daemon status failed: {e}");
                            ExitCode::FAILURE
                        }
                    },
                    Err(e) => {
                        eprintln!("cannot connect to daemon: {e}");
                        ExitCode::FAILURE
                    }
                }
            }
            None => {
                let server = DaemonServer::new(socket);
                match server.start().await {
                    Ok(()) => ExitCode::SUCCESS,
                    Err(e) => {
                        eprintln!("daemon failed: {e}");
                        ExitCode::FAILURE
                    }
                }
            }
        }
    }

    #[cfg(not(feature = "mcp"))]
    {
        let _ = args;
        eprintln!(
            "Daemon mode is not enabled.\n\
             Rebuild with `cargo build --features mcp` to enable daemon support."
        );
        ExitCode::FAILURE
    }
}

fn build_config(args: &ProxyArgs, output_format: OutputFormat) -> Result<Config, ExitCode> {
    Config::builder()
        .backend_cmd(&args.backend)
        .capping(CappingConfig {
            max_frames: args.max_frames,
            max_variables: args.max_variables,
            max_output_length: args.max_output_length,
            max_evaluate_length: args.max_evaluate_length,
            max_value_length: args.max_value_length,
        })
        .enable_output_compress(args.compress_output)
        .enable_variables_compress(args.compress_variables)
        .enable_stacktrace_compress(args.compress_stacktrace)
        .enable_evaluate_compress(args.compress_evaluate)
        .enable_scopes_compress(args.compress_scopes)
        .output_format(output_format)
        .log_level(&args.log_level)
        .build()
        .map_err(|e| {
            eprintln!("Configuration error: {e}");
            ExitCode::FAILURE
        })
}

fn build_interceptor_chain(
    shared_config: &Arc<RwLock<Config>>,
    metrics_cli: bool,
) -> InterceptorChain {
    let config = shared_config.blocking_read();
    let metrics_on = metrics_cli || metrics_enabled_from_env();
    let wrap = |inner: Box<dyn Interceptor>| -> Box<dyn Interceptor> {
        if metrics_on {
            Box::new(MeteredInterceptor::new(inner).enable())
        } else {
            inner
        }
    };

    let interceptors: Vec<Box<dyn Interceptor>> = vec![
        wrap(Box::new(CappingInterceptor::new(
            config.capping.max_frames,
            config.capping.max_variables,
            config.capping.max_output_length,
        ))),
        wrap(Box::new(OutputCompressor)),
        wrap(Box::new(EvaluateCompressor::new(
            config.capping.max_evaluate_length,
        ))),
        wrap(Box::new(VariablesCompressor::new(
            config.capping.max_value_length,
        ))),
        wrap(Box::new(StackTraceCompressor)),
        wrap(Box::new(ScopesCompressor)),
        wrap(Box::new(
            dapz::interceptors::exception::ExceptionInfoCompressor::new(800),
        )),
    ];

    tracing::info!(
        metrics = metrics_on,
        "Interceptor chain built: capping, output, evaluate, variables, stacktrace, scopes, exceptionInfo"
    );

    InterceptorChain::new(interceptors, shared_config.clone())
}

async fn create_transport(
    scheme: &str,
    backend_cmd: &str,
    backend_args: &[String],
) -> Result<Box<dyn Transport>, ExitCode> {
    if scheme == "stdio" {
        return StdioTransport::spawn(backend_cmd, backend_args)
            .map(|t| Box::new(t) as Box<dyn Transport>)
            .map_err(|e| {
                eprintln!("Failed to start backend server: {e}");
                ExitCode::FAILURE
            });
    }

    if let Some(addr) = scheme.strip_prefix("tcp://") {
        return TcpTransport::connect(addr)
            .await
            .map(|t| {
                tracing::info!(addr = %addr, "Connected to TCP DAP adapter");
                Box::new(t) as Box<dyn Transport>
            })
            .map_err(|e| {
                eprintln!("Failed to connect to TCP adapter '{addr}': {e}");
                ExitCode::FAILURE
            });
    }

    eprintln!(
        "Unknown or unsupported transport scheme '{scheme}'. Use 'stdio' or 'tcp://host:port'."
    );
    Err(ExitCode::FAILURE)
}
