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

// --- Error Handling Structures ---

struct SyntaxError {
    std::string error_name;
    uint32_t error_code;
    size_t line_number;
    std::string source_line;
    std::string token_content;

    std::string format_normal(const std::string& filename) const {
        std::stringstream ss;
        ss << "[Error!]In " << filename << " Detected " << error_name 
           << ",Code " << error_code << " at line " << line_number 
           << ".Source:" << source_line << "<-[HERE!]" << token_content;
        return ss.str();
    }

    std::string format_verbose(const std::string& filename) const {
        std::stringstream ss;
        ss << "Error:" << error_name << " Code:" << error_code << "\n";
        ss << "Source:~~~~~\n";
        
        size_t pos = source_line.find(token_content);
        if (pos == std::string::npos) pos = 0;
        
        std::string left = source_line.substr(0, pos);
        std::string right = source_line.substr(pos);
        
        ss << left << "<-[HERE]" << right << "\n";
        
        std::string indent(pos, ' ');
        ss << indent << "^\n";
        ss << "~~~~~~~~~~\n";
        
        return ss.str();
    }
};

// --- Instruction Definitions ---

enum class ComparisonOp {
    Equal, NotEqual, LessThan, GreaterThan, LessEqual, GreaterEqual
};

ComparisonOp parse_comparison_op(const std::string& s) {
    if (s == "=") return ComparisonOp::Equal;
    if (s == "!=") return ComparisonOp::NotEqual;
    if (s == "<") return ComparisonOp::LessThan;
    if (s == ">") return ComparisonOp::GreaterThan;
    if (s == "=<") return ComparisonOp::LessEqual;
    if (s == ">=") return ComparisonOp::GreaterEqual;
    throw std::runtime_error("Unknown op");
}

std::string comparison_op_to_str(ComparisonOp op) {
    switch (op) {
        case ComparisonOp::Equal: return "=";
        case ComparisonOp::NotEqual: return "!=";
        case ComparisonOp::LessThan: return "<";
        case ComparisonOp::GreaterThan: return ">";
        case ComparisonOp::LessEqual: return "=<";
        case ComparisonOp::GreaterEqual: return ">=";
    }
    return "";
}

struct Instruction {
    std::string type;
    std::string arg1, arg2, arg3, arg4;
    ComparisonOp op;
    
    // Helper to check if it's a dummy instruction (from function definition parsing)
    bool is_dummy() const { return type.empty(); }
};

struct Program {
    std::vector<Instruction> instructions;
    std::map<std::string, std::vector<Instruction>> functions;
};

// --- Token & Line Structure ---

struct TokenInfo {
    std::string value;
    size_t line_number;
    size_t col_start; // Approximate column for error reporting
};

// --- Parser Implementation ---

class Parser {
    std::vector<TokenInfo> tokens;
    std::map<size_t, std::string> source_lines_map; // line_num -> content
    size_t pos = 0;

    // Helper to escape C strings
    std::string escape_c_string(const std::string& s) {
        std::string result;
        for (char c : s) {
            if (c == '"') result += "\\\"";
            else if (c == '\\') result += "\\\\";
            else if (c == '\n') result += "\\n";
            else if (c == '\t') result += "\\t";
            else result += c;
        }
        return result;
    }

    std::string get_source_line(size_t line_num) const {
        auto it = source_lines_map.find(line_num);
        if (it != source_lines_map.end()) return it->second;
        return "Unknown Line";
    }

    SyntaxError create_error(const std::string& name, uint32_t code, const std::string& token) const {
        if (pos >= tokens.size()) {
             return {name, code, tokens.back().line_number, get_source_line(tokens.back().line_number), token};
        }
        return {name, code, tokens[pos].line_number, get_source_line(tokens[pos].line_number), token};
    }

    void consume(const std::string& expected) {
        if (pos >= tokens.size() || tokens[pos].value != expected) {
            throw create_error("SyntaxError", 1001, expected);
        }
        pos++;
    }

    std::string next_arg() {
        if (pos >= tokens.size()) {
            throw create_error("UnexpectedEnd", 1002, "EOF");
        }
        return tokens[pos++].value;
    }
    
    // Peek at current token without consuming
    const std::string& peek() const {
        if (pos >= tokens.size()) throw std::runtime_error("EOF peek");
        return tokens[pos].value;
    }
    
    bool has_next() const {
        return pos < tokens.size();
    }
    
    size_t current_line() const {
        if (pos >= tokens.size()) return 0;
        return tokens[pos].line_number;
    }

    Instruction parse_instruction(std::map<std::string, std::vector<Instruction>>& functions) {
        if (!has_next()) throw create_error("UnexpectedEnd", 1002, "EOF");
        
        std::string cmd = peek();
        size_t cmd_line = current_line();

        // Special handling for 'out' to support multi-word strings on the same line
        if (cmd == "out") {
            consume("out");
            if (!has_next()) throw create_error("MissingArgument", 1001, "out");
            
            // Collect all remaining tokens on the SAME line as the output string
            std::string output_str;
            bool first = true;
            while (has_next() && tokens[pos].line_number == cmd_line) {
                // Stop if we hit a known command keyword that starts a new instruction
                // This allows: out Hello \n crt stack
                std::string token_val = tokens[pos].value;
                
                // Check if this token looks like a new command
                // We define commands explicitly here to avoid ambiguity
                if (!first && (token_val == "crt" || token_val == "psh" || token_val == "pop" || 
                               token_val == "del" || token_val == "fnc" || token_val == "cal" || 
                               token_val == "jmp" || token_val == "otn" || token_val == "}")) {
                    break; 
                }
                
                if (!first) output_str += " ";
                output_str += token_val;
                pos++;
                first = false;
            }
            
            if (output_str.empty()) throw create_error("MissingArgument", 1001, "out");
            
            // Store as a special literal argument
            return {"out_lit", "", output_str, "", "", ComparisonOp::Equal};
        }

        if (cmd == "crt") {
            consume("crt");
            return {"crt", next_arg(), "", "", "", ComparisonOp::Equal};
        } else if (cmd == "psh") {
            consume("psh");
            std::string stack = next_arg();
            std::string val = next_arg();
            return {"psh", stack, val, "", "", ComparisonOp::Equal};
        } else if (cmd == "pop") {
            consume("pop");
            return {"pop", next_arg(), "", "", "", ComparisonOp::Equal};
        } else if (cmd == "otn") {
            consume("otn");
            return {"otn", "", "", "", "", ComparisonOp::Equal};
        } else if (cmd == "del") {
            consume("del");
            return {"del", next_arg(), "", "", "", ComparisonOp::Equal};
        } else if (cmd == "cal") {
            consume("cal");
            return {"cal", next_arg(), "", "", "", ComparisonOp::Equal};
        } else if (cmd == "jmp") {
            consume("jmp");
            std::string left = next_arg();
            std::string right = next_arg();
            std::string op_str = next_arg();
            ComparisonOp op = parse_comparison_op(op_str);
            consume("cal");
            std::string target = next_arg();
            return {"jmp", left, right, op_str, target, op};
        } else if (cmd == "fnc") {
            consume("fnc");
            std::string func_name = next_arg();
            consume("{");
            std::vector<Instruction> body;
            while (has_next() && peek() != "}") {
                Instruction instr = parse_instruction(functions);
                if (!instr.is_dummy()) {
                    body.push_back(instr);
                }
            }
            if (!has_next()) throw create_error("MissingBrace", 1004, "}");
            consume("}");
            functions[func_name] = body;
            return {"", "", "", "", "", ComparisonOp::Equal}; // Dummy
        } else if (cmd == "}") {
            throw create_error("UnexpectedBrace", 1005, "}");
        } else {
            throw create_error("UnknownCommand", 1000, cmd);
        }
    }

public:
    Parser(const std::string& input) {
        std::istringstream stream(input);
        std::string line;
        size_t line_idx = 1;
        
        while (std::getline(stream, line)) {
            // Remove carriage return if present (Windows files)
            if (!line.empty() && line.back() == '\r') {
                line.pop_back();
            }
            
            source_lines_map[line_idx] = line;
            
            std::istringstream line_stream(line);
            std::string word;
            size_t col = 0;
            
            while (line_stream >> word) {
                // Simple tokenizer: splits by whitespace
                // Note: This doesn't handle quoted strings "Hello World" as single token yet,
                // but our 'out' handler above solves the specific problem.
                tokens.push_back({word, line_idx, col});
                col += word.length() + 1;
            }
            line_idx++;
        }
    }

    Program parse() {
        Program prog;
        while (has_next()) {
            Instruction instr = parse_instruction(prog.functions);
            if (!instr.is_dummy()) {
                prog.instructions.push_back(instr);
            }
        }
        return prog;
    }
};

// --- Code Generator ---

class CodeGenerator {
    std::set<std::string> stack_names;

    void collect_stack_names(const std::vector<Instruction>& instrs) {
        for (const auto& instr : instrs) {
            if (instr.type == "crt" || instr.type == "psh" || instr.type == "pop" || instr.type == "del") {
                stack_names.insert(instr.arg1);
            } else if (instr.type == "out") {
                 // 'out' arg1 might be a stack name or a literal identifier in old logic
                 // In new logic, literals are 'out_lit'
                 if (instr.type == "out") stack_names.insert(instr.arg1);
            } else if (instr.type == "jmp") {
                stack_names.insert(instr.arg1);
                stack_names.insert(instr.arg2);
            }
        }
    }

    std::string generate_instr(const Instruction& instr) const {
        if (instr.type == "crt") return "init_stack(&" + instr.arg1 + ");";
        if (instr.type == "psh") return "push_stack(&" + instr.arg1 + ", \"" + escape_c_string(instr.arg2) + "\");";
        if (instr.type == "pop") return "pop_stack(&" + instr.arg1 + ");";
        if (instr.type == "out") {
            // Check if arg1 is a known stack
            if (stack_names.count(instr.arg1)) {
                return "print_top(&" + instr.arg1 + ");";
            } else {
                // Treat as literal string if not a stack (legacy behavior fallback)
                return "printf(\"%s \", \"" + escape_c_string(instr.arg1) + "\");";
            }
        }
        if (instr.type == "out_lit") {
            // New behavior: direct string literal output
            return "printf(\"%s \", \"" + escape_c_string(instr.arg2) + "\");";
        }
        if (instr.type == "otn") return "printf(\"\\n\");";
        if (instr.type == "del") return "free_stack(&" + instr.arg1 + ");";
        if (instr.type == "cal") return instr.arg1 + "();";
        if (instr.type == "jmp") {
            return "if (compare_stacks(&" + instr.arg1 + ", &" + instr.arg2 + ", \"" + instr.arg3 + "\")) " + instr.arg4 + "();";
        }
        return "";
    }
    
    std::string escape_c_string(const std::string& s) const {
        std::string result;
        for (char c : s) {
            if (c == '"') result += "\\\"";
            else if (c == '\\') result += "\\\\";
            else if (c == '\n') result += "\\n";
            else if (c == '\t') result += "\\t";
            else result += c;
        }
        return result;
    }

    std::string get_runtime_code() const {
        return R"(
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef struct {
    char** items;
    int size;
    int capacity;
} Stack;

void init_stack(Stack* s) {
    s->capacity = 16;
    s->size = 0;
    s->items = (char**)malloc(s->capacity * sizeof(char*));
}

void push_stack(Stack* s, const char* val) {
    if (s->size == s->capacity) {
        s->capacity *= 2;
        s->items = (char**)realloc(s->items, s->capacity * sizeof(char*));
    }
    s->items[s->size++] = strdup(val);
}

void pop_stack(Stack* s) {
    if (s->size > 0) {
        free(s->items[--s->size]);
    }
}

void free_stack(Stack* s) {
    for (int i = 0; i < s->size; i++) free(s->items[i]);
    free(s->items);
    s->items = NULL;
    s->size = 0;
    s->capacity = 0;
}

void print_top(Stack* s) {
    if (s->size > 0) {
        printf("%s ", s->items[s->size - 1]);
    } else {
        printf("(empty) ");
    }
}

int compare_stacks(Stack* l, Stack* r, const char* op) {
    if (l->size == 0 || r->size == 0) return 0;
    char* lv = l->items[l->size - 1];
    char* rv = r->items[r->size - 1];
    
    char* lend;
    char* rend;
    double ln = strtod(lv, &lend);
    double rn = strtod(rv, &rend);
    
    int is_num = (*lend == '\0' && *rend == '\0' && lend != lv && rend != rv);
    
    if (is_num) {
        if (strcmp(op, "=") == 0) return (ln - rn) < 1e-9 && (ln - rn) > -1e-9;
        if (strcmp(op, "!=") == 0) return (ln - rn) >= 1e-9 || (ln - rn) <= -1e-9;
        if (strcmp(op, "<") == 0) return ln < rn;
        if (strcmp(op, ">") == 0) return ln > rn;
        if (strcmp(op, "=<") == 0) return ln <= rn;
        if (strcmp(op, ">=") == 0) return ln >= rn;
    } else {
        int cmp = strcmp(lv, rv);
        if (strcmp(op, "=") == 0) return cmp == 0;
        if (strcmp(op, "!=") == 0) return cmp != 0;
        if (strcmp(op, "<") == 0) return cmp < 0;
        if (strcmp(op, ">") == 0) return cmp > 0;
        if (strcmp(op, "=<") == 0) return cmp <= 0;
        if (strcmp(op, ">=") == 0) return cmp >= 0;
    }
    return 0;
}
)";
    }

public:
    std::string generate(const Program& prog) {
        std::stringstream ss;
        
        // Collect stack names from all instructions and functions
        collect_stack_names(prog.instructions);
        for (const auto& pair : prog.functions) {
            collect_stack_names(pair.second);
        }

        ss << get_runtime_code();
        
        // Global stack variables
        for (const auto& name : stack_names) {
            ss << "Stack " << name << ";\n";
        }
        ss << "\n";

        // Forward declarations
        for (const auto& pair : prog.functions) {
            ss << "void " << pair.first << "();\n";
        }
        ss << "\n";

        // Function definitions
        for (const auto& pair : prog.functions) {
            ss << "void " << pair.first << "() {\n";
            for (const auto& instr : pair.second) {
                std::string code = generate_instr(instr);
                if (!code.empty()) ss << "    " << code << "\n";
            }
            ss << "}\n\n";
        }

        // Main function
        ss << "int main() {\n";
        for (const auto& name : stack_names) {
            ss << "    init_stack(&" << name << ");\n";
        }
        for (const auto& instr : prog.instructions) {
            std::string code = generate_instr(instr);
            if (!code.empty()) ss << "    " << code << "\n";
        }
        for (const auto& name : stack_names) {
            ss << "    free_stack(&" << name << ");\n";
        }
        ss << "    return 0;\n}\n";

        return ss.str();
    }
};

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
