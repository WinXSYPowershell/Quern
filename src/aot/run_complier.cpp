// --- Compiler & Toolchain ---

struct CompilerOptions {
    std::string optimization = "-O2";
    bool c_verbose = false;
    bool force_warn = false;
    bool no_warn = false;
    bool vm_verbose = false;
    std::string input_file;
};

int run_command(const std::string& cmd) {
    return system(cmd.c_str());
}

bool check_clang() {
#ifdef _WIN32
    return run_command("clang --version >nul 2>&1") == 0;
#else
    return run_command("command -v clang >/dev/null 2>&1") == 0;
#endif
}

void download_toolchain() {
    std::cout << "Clang not found. Attempting to download/install..." << std::endl;
#ifdef _WIN32
    std::string cmd = 
        "powershell -Command \""
        "$url = 'https://github.com/llvm/llvm-project/releases/download/llvmorg-17.0.6/LLVM-17.0.6-win64.exe'; "
        "$out = 'LLVM-installer.exe'; "
        "try { "
        "Invoke-WebRequest -Uri $url -OutFile $out -UseBasicParsing; "
        "Start-Process -FilePath .\\$out -ArgumentList '/S' -Wait; "
        "Remove-Item $out; "
        "} catch { "
        "Write-Host 'Download or installation failed. Please install LLVM manually.'; "
        "exit 1 "
        "}\"";
    if (run_command(cmd) != 0) {
        std::cerr << "Failed to download/install Clang. Please install it manually." << std::endl;
        exit(1);
    }
    std::cout << "Installation complete. You may need to restart your terminal for PATH to update." << std::endl;
#else
    #ifdef __APPLE__
    if (run_command("xcode-select --install") != 0) {
        run_command("brew install llvm");
    }
    #else
    if (run_command("command -v apt-get >/dev/null 2>&1") == 0) {
        run_command("sudo apt-get update && sudo apt-get install -y clang");
    } else if (run_command("command -v dnf >/dev/null 2>&1") == 0) {
        run_command("sudo dnf install -y clang");
    } else if (run_command("command -v pacman >/dev/null 2>&1") == 0) {
        run_command("sudo pacman -S --noconfirm clang");
    } else {
        std::cerr << "Package manager not found. Please install Clang manually." << std::endl;
        exit(1);
    }
    #endif
#endif
}

std::string build_clang_cmd(const std::string& c_file, const std::string& out_file, const CompilerOptions& opts) {
    std::stringstream cmd;
    cmd << "clang " << c_file << " -o " << out_file;
    cmd << " " << opts.optimization;
    
    if (opts.force_warn) cmd << " -Werror";
    if (opts.no_warn) cmd << " -w";
    if (opts.c_verbose) cmd << " -v";
    
    return cmd.str();
}