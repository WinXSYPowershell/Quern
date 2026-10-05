/// Quernc 启动器
const LOGO: &str = r#"QQQQQQQQQ
   QQ:::::::::QQ
  QQ:::::::::::::QQ
 Q:::::::QQQ:::::::Q
 Q::::::O   Q::::::Q uuuuuu    uuuuuu      eeeeeeeeeeee    rrrrr   rrrrrrrrr   nnnn  nnnnnnnn
 Q:::::O     Q:::::Q u::::u    u::::u    ee::::::::::::ee  rrrrrrrr:::::::::r  n:::nn::::::::nn
 Q:::::O     Q:::::Q u::::u    u::::u   e::::::eeeee:::::er:::::::::::::::::r n::::::::::::::nn
 Q:::::O     Q:::::Q u::::u    u::::u  e::::::e     e:::::err::::::rrrrr::::::rnn:::::::::::::::n
 Q:::::O     Q:::::Q u::::u    u::::u  e:::::::eeeee::::::e r:::::r     r:::::r  n:::::nnnn:::::n
 Q:::::O     Q:::::Q u::::u    u::::u  e:::::::::::::::::e  r:::::r     rrrrrrr  n::::n    n::::n
 Q:::::O  QQQQ:::::Q u::::u    u::::u  e::::::eeeeeeeeeee   r:::::r              n::::n    n::::n
 Q::::::O Q::::::::Q u:::::uuuu:::::u  e:::::::e            r:::::r              n::::n    n::::n
 Q:::::::QQ::::::::Q u:::::::::::::::u e::::::::e           r:::::r              n::::n    n::::n
  QQ::::::::::::::Q   u:::::::::::::::u e::::::::eeeeeeee   r:::::r              n::::n    n::::n
   QQ:::::::::::Q     uu::::::::uu:::u  ee:::::::::::::e   r:::::r              n::::n    n::::n
     QQQQQQQQ::::QQ     uuuuuuuu  uuuu    eeeeeeeeeeeeee   rrrrrrr              nnnnnn    nnnnnn
             Q:::::Q
              QQQQQQ
"#;
#[derive(Parser, Debug)]
#[command(name = "QuerncLauncher")]
#[command(about = "A launcher for Quernc and Qvm scripts with AOT support")]
struct Args {

    /// The script file to process (e.g., basic.q)
    #[arg(index = 1)]
    pub script_name: String,

    /// RunScripts: Quernc --run <ScriptName.q>
    #[arg(long)]
    run: Option<String>,

    /// Verbose VM Mode: Run Quernc and Qvm (Verbose)
    #[arg(long)]
    vm_verbose: Option<String>,

    /// Check VM Mode: Run Quernc and Qvm (Check)
    #[arg(long)]
    vm_check: Option<String>,

    /// Run Quern Bytecode Script: Qvm --run <VMScriptName.qb>
    #[arg(long)]
    qvm_run: Option<String>,

    /// Enable logging to file (saves to ./Logs)
    #[arg(long, default_value_t = false)]
    logs: bool,

    /// No print Trace ID to console
    #[arg(long, default_value_t = false)]
    no_print_trace: bool,

    /// Qlm install module
    #[arg(long)]
    module_install: Option<String>,

    /// Qlm delete module
    #[arg(long)]
    module_delete: Option<String>,

    /// Qlm disable module
    #[arg(long)]
    module_disable: Option<String>,

    /// Qlm enable module
    #[arg(long)]
    module_enable: Option<String>,

    /// Qlm search module in cloud repository
    #[arg(long)]
    web_search: Option<String>,

    /// Qlm list cloud modules
    #[arg(long)]
    web_list: bool, // 类型是 bool

    /// Qlm list local modules
    #[arg(long)]
    local_list: bool, // 类型是 bool
    
    /// Qlm install all modules
    #[arg(long)]
    module_install_all: bool, // 类型是 bool
        /// Enable AOT build mode (Quernc -> QuernBuild two-step)
    
    #[arg(long)]
    pub aot_build: bool,

    /// Disable optimizations (NoOpt)
    #[arg(long)]
    pub not_o: bool,

    /// Fast optimization (-O3)
    #[arg(long)]
    pub ofast: bool,

    /// Size optimization (-Os)
    #[arg(long)]
    pub osize: bool,

    /// Size and speed balanced optimization (-Oz)
    #[arg(long)]
    pub osize_bast: bool,

    /// Verbose C compilation output
    #[arg(long)]
    pub aot_c_verbose: bool,

    /// Force warnings
    #[arg(long)]
    pub aot_force_warn: bool,

    /// Disable warnings
    #[arg(long)]
    pub aot_no_warn: bool,

    #[arg(long)]
    pub logo: bool,
}