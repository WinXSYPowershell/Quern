// ============================================================================
// 1. 配置管理 (Config.yaml)
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QuernConfig {
    pub security: SecurityConfig,
    pub logging: LoggingConfig,
    pub runtime: RuntimeConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SecurityConfig {
    pub enable_sandbox: bool,
    pub max_loop_iterations: u64,
    pub allowed_import_paths: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LoggingConfig {
    pub level: String, // INFO, WARN, ERROR
    pub output_trace_id: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RuntimeConfig {
    pub default_stack_size: usize,
    pub enable_jit: bool,
}

impl Default for QuernConfig {
    fn default() -> Self {
        QuernConfig {
            security: SecurityConfig {
                enable_sandbox: true,
                max_loop_iterations: 1000000,
                allowed_import_paths: vec!["./scripts".to_string()],
            },
            logging: LoggingConfig {
                level: "INFO".to_string(),
                output_trace_id: true,
            },
            runtime: RuntimeConfig {
                default_stack_size: 1024,
                enable_jit: false,
            },
        }
    }
}