//! CLI entry point for dapz — DAP compression proxy and MCP server.
//!
//! ```bash
//! dapz proxy --backend "python3 -m debugpy.adapter"
//! dapz mcp   # requires --features mcp
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
use dapz::{CappingConfig, Config, OutputFormat, Proxy, StdioTransport, Transport};
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

    /// Output format: json, toon, or passthrough
    #[arg(short, long, env = "DAPZ_OUTPUT_FORMAT", default_value = "json")]
    output: String,
}

/// Arguments for `dapz mcp`.
#[derive(Parser, Debug)]
struct McpArgs {
    /// Log level
    #[arg(short, long, env = "DAPZ_LOG_LEVEL", default_value = "info")]
    log_level: String,
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Commands::Proxy(args) => run_proxy(args).await,
        Commands::Mcp(args) => run_mcp(args).await,
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
        match StdioTransport::spawn(&backend_cmd, &args.backend_args) {
            Ok(t) => Box::new(t),
            Err(e) => {
                eprintln!("Failed to start backend server: {e}");
                return ExitCode::FAILURE;
            }
        };

    let interceptor_chain = build_interceptor_chain(&shared_config);

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

        use dapz::mcp::McpServer;
        use rmcp::ServiceExt;
        use rmcp::transport::stdio;

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

fn build_interceptor_chain(shared_config: &Arc<RwLock<Config>>) -> InterceptorChain {
    let config = shared_config.blocking_read();
    let interceptors: Vec<Box<dyn Interceptor>> = vec![
        Box::new(CappingInterceptor::new(
            config.capping.max_frames,
            config.capping.max_variables,
            config.capping.max_output_length,
        )),
        Box::new(OutputCompressor),
        Box::new(EvaluateCompressor::new(config.capping.max_evaluate_length)),
        Box::new(VariablesCompressor::new(config.capping.max_value_length)),
        Box::new(StackTraceCompressor),
        Box::new(ScopesCompressor),
    ];

    tracing::info!(
        "Interceptor chain built: capping, output, evaluate, variables, stacktrace, scopes"
    );

    InterceptorChain::new(interceptors, shared_config.clone())
}
