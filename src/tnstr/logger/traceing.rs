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
