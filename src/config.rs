//! Runtime configuration.
//!
//! Builder-pattern configuration with env-var overrides and TOML file support.

use std::path::Path;
use std::str::FromStr;

use serde::Deserialize;

use crate::error::DapzError;

/// Output format for the proxy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    /// Standard JSON (opt-in via `--output json` for IDE pipelines).
    Json,
    /// TOON — token-efficient line protocol (default for proxy, MCP, and SDK).
    Toon,
    /// Passthrough — no transformation in output.
    Passthrough,
}

impl std::str::FromStr for OutputFormat {
    type Err = DapzError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "json" => Ok(Self::Json),
            "toon" => Ok(Self::Toon),
            "passthrough" => Ok(Self::Passthrough),
            _ => Err(DapzError::Config(format!("unknown output format: {s}"))),
        }
    }
}

/// Per-type capping limits for DAP server responses.
#[derive(Debug, Clone, Deserialize)]
pub struct CappingConfig {
    /// Maximum number of stack frames to keep (0 = unlimited).
    pub max_frames: usize,
    /// Maximum number of variables per scope (0 = unlimited).
    pub max_variables: usize,
    /// Maximum output event text length in chars (0 = unlimited).
    pub max_output_length: usize,
    /// Maximum evaluate result string length in chars (0 = unlimited).
    #[serde(default = "default_max_evaluate_length")]
    pub max_evaluate_length: usize,
    /// Maximum variable value string length in chars (0 = unlimited).
    #[serde(default = "default_max_value_length")]
    pub max_value_length: usize,
}

fn default_max_evaluate_length() -> usize {
    500
}

fn default_max_value_length() -> usize {
    120
}

impl Default for CappingConfig {
    fn default() -> Self {
        Self {
            max_frames: 0,
            max_variables: 0,
            max_output_length: 0,
            max_evaluate_length: default_max_evaluate_length(),
            max_value_length: default_max_value_length(),
        }
    }
}

impl CappingConfig {
    /// Returns `true` if any capping limit is set.
    pub fn any_enabled(&self) -> bool {
        self.max_frames > 0 || self.max_variables > 0 || self.max_output_length > 0
    }
}

/// Configuration for the dapz proxy.
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    /// Command used to launch the backend DAP server.
    pub backend_cmd: String,
    /// Per-type response capping limits.
    #[serde(default)]
    pub capping: CappingConfig,
    /// Whether to enable output event compression.
    #[serde(default = "default_true")]
    pub enable_output_compress: bool,
    /// Whether to enable variables response compression.
    #[serde(default = "default_true")]
    pub enable_variables_compress: bool,
    /// Whether to enable stackTrace response compression.
    #[serde(default = "default_true")]
    pub enable_stacktrace_compress: bool,
    /// Whether to enable evaluate response compression.
    #[serde(default = "default_true")]
    pub enable_evaluate_compress: bool,
    /// Whether to enable scopes response compression.
    #[serde(default = "default_true")]
    pub enable_scopes_compress: bool,
    /// Output format.
    #[serde(default = "default_output_format")]
    pub output_format: OutputFormat,
    /// Log level.
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

fn default_true() -> bool {
    true
}

fn default_output_format() -> OutputFormat {
    OutputFormat::Toon
}

fn default_log_level() -> String {
    "info".into()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            backend_cmd: String::new(),
            capping: CappingConfig::default(),
            enable_output_compress: true,
            enable_variables_compress: true,
            enable_stacktrace_compress: true,
            enable_evaluate_compress: true,
            enable_scopes_compress: true,
            output_format: OutputFormat::Toon,
            log_level: "info".into(),
        }
    }
}

impl Config {
    /// Create a new [`ConfigBuilder`].
    pub fn builder() -> ConfigBuilder {
        ConfigBuilder::default()
    }

    /// Load config from a TOML file.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, DapzError> {
        let content = std::fs::read_to_string(path.as_ref())
            .map_err(|e| DapzError::Config(format!("cannot read config file: {e}")))?;
        toml::from_str(&content).map_err(|e| DapzError::Config(format!("invalid config file: {e}")))
    }

    /// Returns `true` if the named interceptor is enabled.
    pub fn is_interceptor_enabled(&self, name: &str) -> bool {
        match name {
            "capping" => self.capping.any_enabled(),
            "output_compressor" => self.enable_output_compress,
            "variables_compressor" => self.enable_variables_compress,
            "stacktrace_compressor" => self.enable_stacktrace_compress,
            "evaluate_compressor" => self.enable_evaluate_compress,
            "scopes_compressor" => self.enable_scopes_compress,
            _ => true,
        }
    }
}

/// Builder for [`Config`].
#[derive(Debug, Default)]
pub struct ConfigBuilder {
    backend_cmd: Option<String>,
    capping: Option<CappingConfig>,
    enable_output_compress: Option<bool>,
    enable_variables_compress: Option<bool>,
    enable_stacktrace_compress: Option<bool>,
    enable_evaluate_compress: Option<bool>,
    enable_scopes_compress: Option<bool>,
    output_format: Option<OutputFormat>,
    log_level: Option<String>,
}

impl ConfigBuilder {
    /// Set the backend DAP server command.
    pub fn backend_cmd(mut self, cmd: impl Into<String>) -> Self {
        self.backend_cmd = Some(cmd.into());
        self
    }

    /// Enable or disable output event compression.
    pub fn enable_output_compress(mut self, enable: bool) -> Self {
        self.enable_output_compress = Some(enable);
        self
    }

    /// Enable or disable variables response compression.
    pub fn enable_variables_compress(mut self, enable: bool) -> Self {
        self.enable_variables_compress = Some(enable);
        self
    }

    /// Enable or disable stackTrace response compression.
    pub fn enable_stacktrace_compress(mut self, enable: bool) -> Self {
        self.enable_stacktrace_compress = Some(enable);
        self
    }

    /// Enable or disable evaluate response compression.
    pub fn enable_evaluate_compress(mut self, enable: bool) -> Self {
        self.enable_evaluate_compress = Some(enable);
        self
    }

    /// Enable or disable scopes response compression.
    pub fn enable_scopes_compress(mut self, enable: bool) -> Self {
        self.enable_scopes_compress = Some(enable);
        self
    }

    /// Set the output format.
    pub fn output_format(mut self, fmt: OutputFormat) -> Self {
        self.output_format = Some(fmt);
        self
    }

    /// Set the log level.
    pub fn log_level(mut self, level: impl Into<String>) -> Self {
        self.log_level = Some(level.into());
        self
    }

    /// Set the response capping limits.
    pub fn capping(mut self, capping: CappingConfig) -> Self {
        self.capping = Some(capping);
        self
    }

    /// Build the [`Config`], validating required fields.
    pub fn build(self) -> Result<Config, DapzError> {
        let backend_cmd = self
            .backend_cmd
            .or_else(|| std::env::var("DAPZ_BACKEND_CMD").ok())
            .ok_or_else(|| DapzError::Config("backend_cmd is required".into()))?;

        let enable_output_compress = self
            .enable_output_compress
            .or_else(|| {
                std::env::var("DAPZ_ENABLE_OUTPUT_COMPRESS")
                    .ok()
                    .and_then(|v| v.parse().ok())
            })
            .unwrap_or(true);

        let enable_variables_compress = self
            .enable_variables_compress
            .or_else(|| {
                std::env::var("DAPZ_ENABLE_VARIABLES_COMPRESS")
                    .ok()
                    .and_then(|v| v.parse().ok())
            })
            .unwrap_or(true);

        let enable_stacktrace_compress = self
            .enable_stacktrace_compress
            .or_else(|| {
                std::env::var("DAPZ_ENABLE_STACKTRACE_COMPRESS")
                    .ok()
                    .and_then(|v| v.parse().ok())
            })
            .unwrap_or(true);

        let enable_evaluate_compress = self
            .enable_evaluate_compress
            .or_else(|| {
                std::env::var("DAPZ_ENABLE_EVALUATE_COMPRESS")
                    .ok()
                    .and_then(|v| v.parse().ok())
            })
            .unwrap_or(true);

        let enable_scopes_compress = self
            .enable_scopes_compress
            .or_else(|| {
                std::env::var("DAPZ_ENABLE_SCOPES_COMPRESS")
                    .ok()
                    .and_then(|v| v.parse().ok())
            })
            .unwrap_or(true);

        let log_level = self
            .log_level
            .or_else(|| std::env::var("DAPZ_LOG_LEVEL").ok())
            .unwrap_or_else(|| "info".into());

        let output_format = self
            .output_format
            .or_else(|| {
                std::env::var("DAPZ_OUTPUT_FORMAT")
                    .ok()
                    .and_then(|v| OutputFormat::from_str(&v).ok())
            })
            .unwrap_or(OutputFormat::Toon);

        // Read capping from env vars
        let capping = self.capping.unwrap_or_else(|| CappingConfig {
            max_frames: std::env::var("DAPZ_MAX_FRAMES")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0),
            max_variables: std::env::var("DAPZ_MAX_VARIABLES")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0),
            max_output_length: std::env::var("DAPZ_MAX_OUTPUT_LENGTH")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0),
            max_evaluate_length: std::env::var("DAPZ_MAX_EVALUATE_LENGTH")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(500),
            max_value_length: std::env::var("DAPZ_MAX_VALUE_LENGTH")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(120),
        });

        Ok(Config {
            backend_cmd,
            capping,
            enable_output_compress,
            enable_variables_compress,
            enable_stacktrace_compress,
            enable_evaluate_compress,
            enable_scopes_compress,
            output_format,
            log_level,
        })
    }
}
