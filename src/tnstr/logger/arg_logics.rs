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