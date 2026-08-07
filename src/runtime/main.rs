use std::collections::HashMap;
use std::fs;
// use std::io;
use std::path::Path;
use std::sync::Arc;
use uuid::Uuid;
use chrono::Local;
use serde::{Deserialize, Serialize};
use regex::Regex;
use tracing::{error, info, warn, trace, debug};

include!("configs.rs");
include!("traceing.rs");
include!("core/interpreter/data.rs");
include!("core/interpreter/interpreter.rs");
include!("core/runtime.rs");

// ============================================================================
// 6. 主程序入口
// ============================================================================

fn main() {
    // 加载配置
    let config_path = "Config.yaml";
    let config = if Path::new(config_path).exists() {
        let content = fs::read_to_string(config_path).expect("Failed to read Config.yaml");
        serde_yaml::from_str(&content).expect("Failed to parse Config.yaml")
    } else {
        // 创建默认配置
        let default_config = QuernConfig::default();
        let yaml = serde_yaml::to_string(&default_config).unwrap();
        fs::write(config_path, yaml).unwrap();
        default_config
    };

    let arc_config = Arc::new(config);

    // 解析命令行参数
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 || args[1] != "--Run" {
        info!("Usage: quern --Run <script_path.q>");
        std::process::exit(1);
    }

    let script_path = &args[2];

    let mut interpreter = Interpreter::new(arc_config);

    match interpreter.run(script_path) {
        Ok(_) => {
            std::process::exit(0);
        }
        Err(e) => {
            eprintln!("\x1b[31mFATAL ERROR: {}\x1b[0m", e);
            std::process::exit(1);
        }
    }
}
