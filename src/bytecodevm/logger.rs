use clap::Parser;
use chrono::Local;
use std::fs;
use std::path::Path;
use std::process::Command;
use tracing::{error, info};
use tracing_subscriber::fmt::time::ChronoLocal;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::Layer; 
use uuid::Uuid;

/// Quernc 启动器
#[derive(Parser, Debug)]
#[command(name = "QuerncLauncher")]
#[command(about = "A launcher for Quernc and Qvm scripts with AOT support")]
struct Args {
    /// 运行脚本: Quernc --run <ScriptName.q>
    #[arg(long)]
    run: Option<String>,

    /// 详细虚拟机模式: 运行 Quernc 和 Qvm (Verbose)
    #[arg(long)]
    vm_verbose: Option<String>,

    /// 检查虚拟机模式: 运行 Quernc 和 Qvm (Check)
    #[arg(long)]
    vm_check: Option<String>,

    /// 运行 QB 脚本: Qvm --run <VMScriptName.qb>
    #[arg(long)]
    qvm_run: Option<String>,

    /// 启用日志记录
    #[arg(long, default_value_t = false)]
    logs: bool,

    /// 不打印 Trace ID 到控制台
    #[arg(long, default_value_t = false)]
    no_print_trace: bool,

    // --- AOT Build Parameters ---

    /// Translate to Clang -Os
    #[arg(long, default_value_t = false)]
    aot_clang_o_size: bool,

    /// Translate to Clang -Oz
    #[arg(long, default_value_t = false)]
    aot_clang_o_size_best: bool,

    /// Translate to Clang -Og
    #[arg(long, default_value_t = false)]
    aot_clang_o_debug: bool,

    /// Translate to Clang -Ofast
    #[arg(long, default_value_t = false)]
    aot_clang_ofast: bool,

    /// Translate to Clang -O0
    #[arg(long, default_value_t = false)]
    aot_not_o: bool,

    /// C code verbose compilation
    #[arg(long, default_value_t = false)]
    aot_c_verbose: bool,

    /// Treat warnings as errors
    #[arg(long, default_value_t = false)]
    aot_force_warn: bool,

    /// Suppress warnings
    #[arg(long, default_value_t = false)]
    aot_no_warn: bool,
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
    } else if let Some(script) = &args.qvm_run {
        Some(("QvmRun", script.clone()))
    } else {
        None
    };

    if operation.is_none() {
        eprintln!("Error: No operation specified. Use --run, --vm-verbose, --vm-check, or --qvm-run.");
        std::process::exit(1);
    }

    let (mode, script_name) = operation.unwrap();
    let trace_id = Uuid::new_v4().to_string();
    
    // 设置日志系统
    setup_tracing(args.logs, mode, &trace_id, args.no_print_trace);

    info!("Starting Launcher with Trace ID: {}", trace_id);
    info!("Mode: {}, Script: {}", mode, script_name);

    // 执行命令
    // 注意：只有涉及编译的模式（Run, VMVerbose, VMCheck）才需要传递 AOT 参数
    // QvmRun 通常直接运行字节码，不需要编译参数，但根据架构可能需要调整
    let result = match mode {
        "Run" => execute_run(&script_name, &trace_id, &args),
        "VMVerbose" => execute_vm_verbose(&script_name, &trace_id, &args),
        "VMCheck" => execute_vm_check(&script_name, &trace_id, &args),
        "QvmRun" => execute_qvm_run(&script_name, &trace_id),
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

/// 执行 Quernc 编译，支持 AOT 参数
fn execute_quernc(script: &str, trace_id: &str, args: &Args) -> Result<(), String> {
    info!(trace_id = trace_id, "Executing Quernc with AOT options");
    
    let mut cmd = Command::new("Quernc");
    cmd.arg("--run").arg(script);

    // 添加 AOT 优化等级参数
    if args.aot_clang_o_size {
        cmd.arg("--clang-o-size");
    }
    if args.aot_clang_o_size_best {
        cmd.arg("--clang-o-size-best");
    }
    if args.aot_clang_o_debug {
        cmd.arg("--clang-o-debug");
    }
    if args.aot_clang_ofast {
        cmd.arg("--clang-ofast");
    }
    if args.aot_not_o {
        cmd.arg("--not-o");
    }

    // 添加其他 AOT 标志
    if args.aot_c_verbose {
        cmd.arg("--c-verbose");
    }
    if args.aot_force_warn {
        cmd.arg("--force-warn");
    }
    if args.aot_no_warn {
        cmd.arg("--no-warn");
    }

    let output = cmd.output()
        .map_err(|e| format!("Failed to execute Quernc: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!(trace_id = trace_id, "Quernc failed: {}", stderr);
        return Err(format!("Quernc exited with error: {}", stderr));
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    info!(trace_id = trace_id, "Quernc output: {}", stdout);
    Ok(())
}

fn execute_qvm(script: &str, extra_args: &[&str], trace_id: &str) -> Result<(), String> {
    info!(trace_id = trace_id, "Executing Qvm with args: {:?}", extra_args);
    
    let mut cmd = Command::new("Qvm");
    // 始终添加 --run 参数
    cmd.arg("--run");
    // 添加额外参数 (如 --verbose, --check)
    for arg in extra_args {
        cmd.arg(arg);
    }
    // 最后添加脚本名称
    cmd.arg(script);

    let output = cmd.output().map_err(|e| format!("Failed to execute Qvm: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!(trace_id = trace_id, "Qvm failed: {}", stderr);
        return Err(format!("Qvm exited with error: {}", stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    info!(trace_id = trace_id, "Qvm output: {}", stdout);
    Ok(())
}

fn execute_run(script: &str, trace_id: &str, args: &Args) -> Result<(), String> {
    execute_quernc(script, trace_id, args)?;
    Ok(())
}

fn execute_vm_verbose(script: &str, trace_id: &str, args: &Args) -> Result<(), String> {
    execute_quernc(script, trace_id, args)?;
    execute_qvm(script, &["--verbose"], trace_id)?;
    Ok(())
}

fn execute_vm_check(script: &str, trace_id: &str, args: &Args) -> Result<(), String> {
    execute_quernc(script, trace_id, args)?;
    execute_qvm(script, &["--check"], trace_id)?;
    Ok(())
}

fn execute_qvm_run(script: &str, trace_id: &str) -> Result<(), String> {
    // 仅运行 Qvm，不带额外参数，除非未来需要扩展
    // 根据需求：Qvm --run <VMScriptName.qb>
    execute_qvm(script, &[], trace_id)?;
    Ok(())
}
