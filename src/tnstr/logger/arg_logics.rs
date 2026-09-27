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

/// Execute AOT Build - Two-step process: Quernc -> QuernBuild
fn execute_aot_build(script: &str, trace_id: &str, args: &Args) -> Result<(), String> {
    info!(trace_id = trace_id, "Starting AOT build for script: {}", script);

    // Extract filename without extension
    let file_stem = std::path::Path::new(script)
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| "Invalid script path".to_string())?;

    // Define output paths
    let bytecode_path = format!("./cache/bytecode/{}.qb", file_stem);
    
    // Ensure cache directory exists
    std::fs::create_dir_all("./cache/bytecode")
        .map_err(|e| format!("Failed to create cache directory: {}", e))?;

    // Step 1: Run Quernc to translate .q to .qb
    info!(trace_id = trace_id, "Step 1: Running Quernc to generate bytecode...");
    
    let quernc_output = Command::new("Quernc")
        .arg("--Run")
        .arg(script)
        .output()
        .map_err(|e| format!("Failed to execute Quernc: {}", e))?;

    if !quernc_output.status.success() {
        let stderr = String::from_utf8_lossy(&quernc_output.stderr);
        error!(trace_id = trace_id, "Quernc failed: {}", stderr);
        return Err(format!("Quernc translation failed"));
    }

    info!(trace_id = trace_id, "Quernc completed successfully. Bytecode at: {}", bytecode_path);

    // Step 2: Determine optimization level for QuernBuild
    let opt_flag = if args.aot_clang_o_size {
        "--ClangOSize"
    } else if args.aot_clang_o_size_best {
        "--ClangOSizeBest"
    } else if args.aot_clang_o_debug {
        "--ClangODebug"
    } else if args.aot_clang_ofast {
        "--ClangOFAST"
    } else if args.aot_not_o {
        "--NotO"
    } else {
        "--ClangOSize" // Default to -Os
    };

    // Step 3: Run QuernBuild with the bytecode file
    info!(trace_id = trace_id, "Step 2: Running QuernBuild with {}...", opt_flag);
    
    let mut cmd = Command::new("QuernBuild");
    cmd.arg(opt_flag).arg(&bytecode_path);
    
    // Add optional flags
    if args.aot_c_verbose {
        cmd.arg("--CVerbose");
    }
    if args.aot_force_warn {
        cmd.arg("--ForceWarn");
    }
    if args.aot_no_warn {
        cmd.arg("--NoWarn");
    }

    let build_output = cmd.output()
        .map_err(|e| format!("Failed to execute QuernBuild: {}", e))?;

    if !build_output.status.success() {
        let stderr = String::from_utf8_lossy(&build_output.stderr);
        error!(trace_id = trace_id, "QuernBuild failed: {}", stderr);
        return Err(format!("QuernBuild compilation failed"));
    }

    let stdout = String::from_utf8_lossy(&build_output.stdout);
    info!(trace_id = trace_id, "AOT build successful!\n{}", stdout);
    Ok(())
}