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

include!("args.rs");

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
    } else if let Some(script) = &args.module_install {
        Some(("ModuleInstall", script.clone()))
    } else if let Some(script) = &args.module_delete {
        Some(("ModuleDelete", script.clone()))
    } else if let Some(script) = &args.module_disable {
        Some(("ModuleDisable", script.clone()))
    } else if let Some(script) = &args.module_enable {
        Some(("ModuleEnable", script.clone()))
    } else if let Some(script) = &args.web_search {
        Some(("WebSearch", script.clone()))
    } else if args.web_list { // 直接判断布尔值
        Some(("WebList", "".to_string()))
    } else if args.module_install_all { // 直接判断布尔值
        Some(("ModuleInstall", "".to_string()))
    } else if args.local_list { // 直接判断布尔值
        Some(("LocalList", "".to_string()))
    } else {
        None
    };

    if operation.is_none() {
        eprintln!("Error: No operation specified. Use --run, --vm-verbose, --vm-check, --module-install, --module-delete, --module-disable, --module-enable, --web-list ,--web-search, --local-list, --module-install, --help, or --qvm-run.");
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
        "ModuleInstall" => execute_module_install(&script_name, &trace_id),
        "ModuleDelete" => execute_module_delete(&script_name, &trace_id),
        "ModuleDisable" => execute_module_disable(&script_name, &trace_id),
        "ModuleEnable" => execute_module_enable(&script_name, &trace_id),
        "WebList" => execute_web_list(&script_name, &trace_id),
        "LocalList" => execute_local_list(&script_name, &trace_id),
        "WebSearch" => execute_web_search(&script_name, &trace_id),
        "ModuleInstallAll" => execute_module_install_all(&script_name, &trace_id),
        "AOTBuild" => execute_aot_build(&script_name, &trace_id),
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

include!("traceing.rs");
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

fn execute_module_install(module_name: &str, trace_id: &str) -> Result<(), String> {
    info!(trace_id = trace_id, "Executing Qlm.exe to install module: {}", module_name);
    
    let mut cmd = Command::new("Qlm.exe");
    cmd.arg("--install").arg(module_name);

    let output = cmd.output()
        .map_err(|e| format!("Failed to execute Qlm.exe: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!(trace_id = trace_id, "Qlm.exe failed: {}", stderr);
        return Err(format!("Qlm.exe exited with error: {}", stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    info!(trace_id = trace_id, "Qlm.exe output: {}", stdout);
    Ok(())
}

fn execute_module_delete(module_name: &str, trace_id: &str) -> Result<(), String> {
    info!(trace_id = trace_id, "Executing Qlm.exe to delete module: {}", module_name);
    
    let mut cmd = Command::new("Qlm.exe");
    cmd.arg("--delete").arg(module_name);

    let output = cmd.output()
        .map_err(|e| format!("Failed to execute Qlm.exe: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!(trace_id = trace_id, "Qlm.exe failed: {}", stderr);
        return Err(format!("Qlm.exe exited with error: {}", stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    info!(trace_id = trace_id, "Qlm.exe output: {}", stdout);
    Ok(())
}

fn execute_module_disable(module_name: &str, trace_id: &str) -> Result<(), String> {
    info!(trace_id = trace_id, "Executing Qlm.exe to disable module: {}", module_name);
    
    let mut cmd = Command::new("Qlm.exe");
    cmd.arg("--disable").arg(module_name);

    let output = cmd.output()
        .map_err(|e| format!("Failed to execute Qlm.exe: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!(trace_id = trace_id, "Qlm.exe failed: {}", stderr);
        return Err(format!("Qlm.exe exited with error: {}", stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    info!(trace_id = trace_id, "Qlm.exe output: {}", stdout);
    Ok(())
}

fn execute_module_enable(module_name: &str, trace_id: &str) -> Result<(), String> {
    info!(trace_id = trace_id, "Executing Qlm.exe to enable module: {}", module_name);
    
    let mut cmd = Command::new("Qlm.exe");
    cmd.arg("--enable").arg(module_name);

    let output = cmd.output()
        .map_err(|e| format!("Failed to execute Qlm.exe: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!(trace_id = trace_id, "Qlm.exe failed: {}", stderr);
        return Err(format!("Qlm.exe exited with error: {}", stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    info!(trace_id = trace_id, "Qlm.exe output: {}", stdout);
    Ok(())
}

fn execute_web_list(_script_name: &str, trace_id: &str) -> Result<(), String> {
    info!(trace_id = trace_id, "Executing Qlm.exe to list cloud modules");
    
    let mut cmd = Command::new("Qlm.exe");
    cmd.arg("--web-list");

    let output = cmd.output()
        .map_err(|e| format!("Failed to execute Qlm.exe: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!(trace_id = trace_id, "Qlm.exe failed: {}", stderr);
        return Err(format!("Qlm.exe exited with error: {}", stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    info!(trace_id = trace_id, "Qlm.exe output: {}", stdout);
    Ok(())
}

fn execute_local_list(_script_name: &str, trace_id: &str) -> Result<(), String> {
    info!(trace_id = trace_id, "Executing Qlm.exe to list local modules");
    
    let mut cmd = Command::new("Qlm.exe");
    cmd.arg("--mods-list");

    let output = cmd.output()
        .map_err(|e| format!("Failed to execute Qlm.exe: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!(trace_id = trace_id, "Qlm.exe failed: {}", stderr);
        return Err(format!("Qlm.exe exited with error: {}", stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    info!(trace_id = trace_id, "Qlm.exe output: {}", stdout);
    Ok(())
}

fn execute_web_search(search_query: &str, trace_id: &str) -> Result<(), String> {
    info!(trace_id = trace_id, "Executing Qlm.exe to search web modules with query: {}", search_query);
    
    let mut cmd = Command::new("Qlm.exe");
    cmd.arg("--web-search").arg(search_query);

    let output = cmd.output()
        .map_err(|e| format!("Failed to execute Qlm.exe: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!(trace_id = trace_id, "Qlm.exe failed: {}", stderr);
        return Err(format!("Qlm.exe exited with error: {}", stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    info!(trace_id = trace_id, "Qlm.exe output: {}", stdout);
    Ok(())
}

fn execute_module_install_all(_script_name: &str, trace_id: &str) -> Result<(), String> {
    info!(trace_id = trace_id, "Executing Qlm.exe to install all modules");
    
    let mut cmd = Command::new("Qlm.exe");
    cmd.arg("--InstallPackage");

    let output = cmd.output()
        .map_err(|e| format!("Failed to execute Qlm.exe: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!(trace_id = trace_id, "Qlm.exe failed: {}", stderr);
        return Err(format!("Qlm.exe exited with error: {}", stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    info!(trace_id = trace_id, "Qlm.exe output: {}", stdout);
    Ok(())
}