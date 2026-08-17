use clap::Parser;
use chrono::Local;
use std::fs;
use std::path::Path;
use std::process::Command;
use tracing::{error, info};
use tracing_subscriber::fmt::time::ChronoLocal;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::Layer; // 必须导入这个 Trait 才能使用 .boxed()
use uuid::Uuid;

/// Quernc 启动器
#[derive(Parser, Debug)]
#[command(name = "QuerncLauncher")]
#[command(about = "A launcher for Quernc and Qvm scripts")]
struct Args {
    /// 运行脚本: Quernc.exe --Run <ScriptName.q>
    #[arg(long)]
    run: Option<String>,

    /// 详细虚拟机模式: 运行 Quernc 和 Qvm (Verbose)
    #[arg(long)]
    vm_verbose: Option<String>,

    /// 检查虚拟机模式: 运行 Quernc 和 Qvm (Check)
    #[arg(long)]
    vm_check: Option<String>,

    /// 启用日志记录
    #[arg(long, default_value_t = false)]
    logs: bool,

    /// 不打印 Trace ID 到控制台
    #[arg(long, default_value_t = false)]
    no_print_trace: bool,
}

fn main() {
    let args = Args::parse();

    // 确定操作模式和脚本名称
    let operation = if let Some(script) = &args.run {
        Some(("Run", script.clone()))
    } else if let Some(script) = &args.vm_verbose {
        Some(("VMVerbose", script.clone()))
    } else if let Some(script) = &args.vm_check {
        Some(("VMCheck", script.clone()))
    } else {
        None
    };

    if operation.is_none() {
        eprintln!("Error: No operation specified. Use --Run, --VMVerbose, or --VMCheck.");
        std::process::exit(1);
    }

    let (mode, script_name) = operation.unwrap();
    let trace_id = Uuid::new_v4().to_string();
    
    // 设置日志系统
    setup_tracing(args.logs, mode, &trace_id, args.no_print_trace);

    info!("Starting Launcher with Trace ID: {}", trace_id);
    info!("Mode: {}, Script: {}", mode, script_name);

    // 执行命令
    let result = match mode {
        "Run" => execute_run(&script_name, &trace_id),
        "VMVerbose" => execute_vm_verbose(&script_name, &trace_id),
        "VMCheck" => execute_vm_check(&script_name, &trace_id),
        _ => Err(format!("Unknown mode: {}", mode)),
    };

    match result {
        Ok(_) => {
            info!("All commands executed successfully.");
        }
        Err(e) => {
            error!("Execution failed: {}", e);
            std::process::exit(1);
        }
    }
}

fn setup_tracing(enable_logs: bool, mode: &str, trace_id: &str, no_print_trace: bool) {
    let mut layers = vec![];

    // 1. 控制台层
    let console_layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stdout)
        .with_target(false)
        .with_thread_ids(false)
        .with_level(true)
        .compact();
    
    // 如果不禁止打印 Trace ID，则在开始时打印
    if !no_print_trace {
        println!("[TRACE ID] {}", trace_id);
    }

    // 使用 .boxed() 需要导入 tracing_subscriber::Layer
    layers.push(console_layer.boxed());

    // 2. 文件日志层 (如果启用)
    if enable_logs {
        let now = Local::now();
        let year = now.format("%Y").to_string();
        let month = now.format("%m").to_string();
        let day = now.format("%d").to_string();
        let time_str = now.format("%H%M%S").to_string();
        
        let log_dir = Path::new("./Logs")
            .join(&year)
            .join(&month)
            .join(&day);
        
        // 创建目录
        if let Err(e) = fs::create_dir_all(&log_dir) {
            eprintln!("Failed to create log directory: {:?}", e);
            return;
        }

        let file_name = format!("{}_{}_{}_log.log", time_str, mode, trace_id);
        let log_file_path = log_dir.join(&file_name);

        // 创建文件
        match fs::File::create(&log_file_path) {
            Ok(file) => {
                let file_layer = tracing_subscriber::fmt::layer()
                    .with_writer(file)
                    .with_timer(ChronoLocal::new("%Y-%m-%d %H:%M:%S%.3f".to_string()))
                    .with_target(true)
                    .with_thread_ids(true)
                    .with_level(true);
                
                layers.push(file_layer.boxed());
                // 注意：这里不能直接 info!，因为 subscriber 还没初始化
                // 但我们可以在控制台层已经激活的情况下，稍微变通一下，或者仅仅依赖后续日志
            }
            Err(e) => {
                eprintln!("Failed to create log file: {:?}", e);
            }
        }
    }

    let subscriber = tracing_subscriber::registry()
        .with(layers);

    subscriber.init();
}

fn execute_quernc(script: &str, trace_id: &str) -> Result<(), String> {
    info!(trace_id = trace_id, "Executing Quernc.exe");
    let output = Command::new("Quernc.exe")
        .arg("--Run")
        .arg(script)
        .output()
        .map_err(|e| format!("Failed to execute Quernc.exe: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!(trace_id = trace_id, "Quernc.exe failed: {}", stderr);
        return Err(format!("Quernc.exe exited with error: {}", stderr));
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    info!(trace_id = trace_id, "Quernc.exe output: {}", stdout);
    Ok(())
}

fn execute_qvm(script: &str, extra_args: &[&str], trace_id: &str) -> Result<(), String> {
    info!(trace_id = trace_id, "Executing Qvm.exe with args: {:?}", extra_args);
    
    let mut cmd = Command::new("Qvm.exe");
    cmd.arg("--Run");
    for arg in extra_args {
        cmd.arg(arg);
    }
    cmd.arg(script);

    let output = cmd.output().map_err(|e| format!("Failed to execute Qvm.exe: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!(trace_id = trace_id, "Qvm.exe failed: {}", stderr);
        return Err(format!("Qvm.exe exited with error: {}", stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    info!(trace_id = trace_id, "Qvm.exe output: {}", stdout);
    Ok(())
}

fn execute_run(script: &str, trace_id: &str) -> Result<(), String> {
    execute_quernc(script, trace_id)?;
    Ok(())
}

fn execute_vm_verbose(script: &str, trace_id: &str) -> Result<(), String> {
    execute_quernc(script, trace_id)?;
    execute_qvm(script, &["--Verbose"], trace_id)?;
    Ok(())
}

fn execute_vm_check(script: &str, trace_id: &str) -> Result<(), String> {
    execute_quernc(script, trace_id)?;
    execute_qvm(script, &["--Check"], trace_id)?;
    Ok(())
}