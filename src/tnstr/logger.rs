include!("logger/modules.rs");
include!("logger/args.rs");
include!("logger/arg_logics.rs");

fn main() {
    // 收集命令行参数
    let raw_args: Vec<OsString> = std::env::args_os().collect();
    
    // 创建带 logo 的命令 —— before_help 会在每次打印帮助前自动输出 logo
    let app = Args::command().before_help(LOGO.to_string());
    
    // 尝试解析参数
    let args = Args::try_parse_from(raw_args.clone()).unwrap_or_else(|e| {
        match e.kind() {
            // 用户请求了帮助（--help），用带 logo 的命令打印
            clap::error::ErrorKind::DisplayHelp |
            clap::error::ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand => {
                let _ = app.clone().print_help();
                println!();
            }
            // 其他错误正常打印
            _ => {
                let _ = e.print();
            }
        }
        std::process::exit(1);
    });

    // 处理 --logo 参数：直接打印 logo 并退出
    if args.logo {
        print!("{}", LOGO);
        return;
    }

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
    } else if args.aot_build {
        Some(("AOTBuild", args.script_name.clone())) 
    } else {
        // 如果没有指定任何操作标志，默认以 Run 模式执行 script_name
        Some(("Run", args.script_name.clone()))
    };

    if operation.is_none() {
        eprintln!("Error: No operation specified. Use --help for usage information.");
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
        "AOTBuild" => execute_aot_build(&script_name, &trace_id, &args),
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

include!("logger/traceing.rs");
