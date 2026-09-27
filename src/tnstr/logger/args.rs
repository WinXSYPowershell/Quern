/// Quernc 启动器
#[derive(Parser, Debug)]
#[command(name = "QuerncLauncher")]
#[command(about = "A launcher for Quernc and Qvm scripts with AOT support")]
struct Args {
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