/// Quernc 启动器

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "QuerncLauncher")]
#[command(about = "A launcher for Quernc and Qvm scripts with AOT support")]

pub struct QuernArgs {
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
    web_list: bool,

    /// Qlm list local modules
    #[arg(long)]
    local_list: bool,

    /// Qlm install all modules
    #[arg(long)]
    module_install_all: bool,

    // === AOT Build 参数 ===

    /// Enable AOT build mode (--AOTBuild)
    #[arg(long)]
    pub aot_build: bool,

    /// Disable optimizations (--NotO)
    #[arg(long)]
    pub not_o: bool,

    /// Fast optimization (--OFast)
    #[arg(long)]
    pub ofast: bool,

    /// Size optimization (--OSize)
    #[arg(long)]
    pub osize: bool,

    /// Size and speed balanced optimization (--OSizeBest)
    #[arg(long)]
    pub osize_bast: bool,

    /// Input Q file for AOT build (position 2)
    #[arg(index = 2)]
    pub input_q: Option<String>,

    /// AOT C verbose output (--aot-c-verbose)
    #[arg(long)]
    pub aot_c_verbose: bool,

    /// AOT force warnings (--aot-force-warn)
    #[arg(long)]
    pub aot_force_warn: bool,

    /// AOT disable warnings (--aot-no-warn)
    #[arg(long)]
    pub aot_no_warn: bool,
}
