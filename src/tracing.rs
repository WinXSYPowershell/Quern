// ============================================================================
// 2. 结构化日志与追踪 (Tracing)
// ============================================================================

#[derive(Debug, Clone)]
pub struct TraceContext {
    pub trace_id: String,
    pub timestamp: String,
}

impl TraceContext {
    pub fn new() -> Self {
        TraceContext {
            trace_id: Uuid::new_v4().to_string(),
            timestamp: Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string(),
        }
    }
}

pub struct Logger {
    config: LoggingConfig,
}

impl Logger {
    pub fn new(config: LoggingConfig) -> Self {
        Logger { config }
    }

    fn format_log(&self, level: &str, msg: &str, trace: &TraceContext) -> String {
        if self.config.output_trace_id {
            format!("[{}] [{}] [Trace: {}] {}", trace.timestamp, level, trace.trace_id, msg)
        } else {
            format!("[{}] [{}] {}", trace.timestamp, level, msg)
        }
    }

    pub fn info(&self, msg: &str, trace: &TraceContext) {
        info!("\x1b[34m{}\x1b[0m", self.format_log("INFO", msg, trace));
    }

    pub fn warn(&self, msg: &str, trace: &TraceContext) {
        error!("\x1b[33m{}\x1b[0m", self.format_log("WARN", msg, trace));
    }

    pub fn error(&self, msg: &str, trace: &TraceContext) {
        error!("\x1b[31m{}\x1b[0m", self.format_log("ERROR", msg, trace));
    }
}