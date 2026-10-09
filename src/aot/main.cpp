#include <iostream>
#include <fstream>
#include <sstream>
#include <string>
#include <vector>
#include <map>
#include <set>
#include <memory>
#include <cstdlib>
#include <algorithm>
#include <cctype>


#ifdef _WIN32
#include <windows.h>
#endif

// include else code files for build
#include "errors.cpp"
#include "instruction.cpp"
#include "parser.cpp"
#include "ganerator.cpp"
#include "run_complier.cpp"


int main(int argc, char* argv[]) {
    if (argc < 2) {
        std::cerr << "Usage: " << argv[0] << " [options] <input_file.qb>\n"
                  << "Options:\n"
                  << "  --ClangOSize      : Translate to Clang -Os\n"
                  << "  --ClangOSizeBest  : Translate to Clang -Oz\n"
                  << "  --ClangODebug     : Translate to Clang -Og\n"
                  << "  --ClangOFAST      : Translate to Clang -Ofast\n"
                  << "  --NotO            : Translate to Clang -O0\n"
                  << "  --CVerbose        : C code verbose compilation\n"
                  << "  --ForceWarn       : Treat warnings as errors\n"
                  << "  --NoWarn          : Suppress warnings\n"
                  << "  --VMVerbose       : VM detailed error reporting\n";
        return 1;
    }

    CompilerOptions opts;
    std::string input_file;

    for (int i = 1; i < argc; ++i) {
        std::string arg = argv[i];
        if (arg == "--ClangOSize") opts.optimization = "-Os";
        else if (arg == "--ClangOSizeBest") opts.optimization = "-Oz";
        else if (arg == "--ClangODebug") opts.optimization = "-Og";
        else if (arg == "--ClangOFAST") opts.optimization = "-Ofast";
        else if (arg == "--NotO") opts.optimization = "-O0";
        else if (arg == "--CVerbose") opts.c_verbose = true;
        else if (arg == "--ForceWarn") opts.force_warn = true;
        else if (arg == "--NoWarn") opts.no_warn = true;
        else if (arg == "--VMVerbose") opts.vm_verbose = true;
        else if (arg[0] != '-') input_file = arg;
    }

    if (input_file.empty()) {
        std::cerr << "Error: Missing input file." << std::endl;
        return 1;
    }

    // Check and install toolchain
    if (!check_clang()) {
        download_toolchain();
        if (!check_clang()) {
            std::cerr << "Clang is still not found after installation attempt. Please install manually and ensure it's in PATH." << std::endl;
            return 1;
        }
    }

    // Read input file
    std::ifstream infile(input_file);
    if (!infile.is_open()) {
        std::cerr << "Failed to open file: " << input_file << std::endl;
        return 1;
    }
    std::stringstream buffer;
    buffer << infile.rdbuf();
    std::string content = buffer.str();
    infile.close();

    // Parse
    Parser parser(content);
    Program prog;
    try {
        prog = parser.parse();
    } catch (const SyntaxError& err) {
        if (opts.vm_verbose) {
            std::cerr << err.format_verbose(input_file);
        } else {
            std::cerr << err.format_normal(input_file) << std::endl;
        }
        return 1;
    }

    // Generate C code
    CodeGenerator generator;
    std::string c_code = generator.generate(prog);

    std::string c_file = input_file + ".c";
    std::string out_file = input_file + ".exe";
#ifdef _WIN32
    // keep .exe
#else
    out_file = input_file; // remove .exe for linux/mac
#endif

    std::ofstream outfile(c_file);
    outfile << c_code;
    outfile.close();

    // Compile
    std::string cmd = build_clang_cmd(c_file, out_file, opts);
    std::cout << "Compiling: " << cmd << std::endl;
    
    if (run_command(cmd) != 0) {
        std::cerr << "Compilation failed." << std::endl;
        return 1;
    }

    std::cout << "Successfully compiled to " << out_file << std::endl;
    return 0;
}