// --- Code Generator ---

class CodeGenerator {
    std::set<std::string> stack_names;

    void collect_stack_names(const std::vector<Instruction>& instrs) {
        // Normalize a stack-name token before registering it: "@name@" (variable)
        // refers to the stack called "name", so store the base name only.
        auto reg = [this](const std::string& a) {
            stack_names.insert(is_variable(a) ? var_base_name(a) : a);
        };
        for (const auto& instr : instrs) {
            if (instr.type == "crt" || instr.type == "pop" || instr.type == "del") {
                reg(instr.arg1);
            } else if (instr.type == "psh") {
                reg(instr.arg1);                              // target stack (plain name)
                if (is_variable(instr.arg2)) reg(instr.arg2); // pushed value is a @var@
            } else if (instr.type == "psh_op") {
                reg(instr.arg1);                              // target stack (plain name or @var@)
            } else if (instr.type == "out_var") {
                reg(instr.arg2);                              // referenced stack variable
            } else if (instr.type == "jmp") {
                reg(instr.arg1);                              // operand: plain stack name or @var@
                reg(instr.arg2);
            }
        }
    }

    // Check if the program actually uses any stack operations or comparisons
    bool needs_stack_support(const Program& prog) const {
        auto check_instrs = [](const std::vector<Instruction>& instrs) -> bool {
            for (const auto& instr : instrs) {
                if (instr.type == "crt" || instr.type == "psh" || instr.type == "psh_op" ||
                    instr.type == "pop" || instr.type == "del" || instr.type == "jmp" ||
                    instr.type == "out_var") {
                    return true;
                }
            }
            return false;
        };

        if (check_instrs(prog.instructions)) return true;
        for (const auto& pair : prog.functions) {
            if (check_instrs(pair.second)) return true;
        }
        return false;
    }

    std::string generate_instr(const Instruction& instr) const {
        if (instr.type == "crt") return "";  // Stack init handled in main() loop
        
        // 辅助 lambda：去除首尾引号并转义
        auto process_string = [](const std::string& s) -> std::string {
            std::string content = s;
            // 去除首尾的双引号
            if (content.size() >= 2 && content.front() == '"' && content.back() == '"') {
                content = content.substr(1, content.size() - 2);
            }
            // 转义剩余的特殊字符
            std::string result;
            for (char c : content) {
                if (c == '"') result += "\\\"";
                else if (c == '\\') result += "\\\\";
                else if (c == '\n') result += "\\n";
                else if (c == '\t') result += "\\t";
                else result += c;
            }
            return result;
        };

        if (instr.type == "psh") {
            // 目标栈基名（允许 @var@ 形式）
            std::string base = is_variable(instr.arg1) ? var_base_name(instr.arg1) : instr.arg1;
            std::string target = is_variable(instr.arg1)
                ? ("find_stack(\"" + base + "\")")
                : ("&" + sanitize_c_id(instr.arg1));
            std::string valExpr;
            if (is_variable(instr.arg2)) {
                valExpr = "get_top_by_name(\"" + var_base_name(instr.arg2) + "\")";
            } else {
                valExpr = "\""+ process_string(instr.arg2) + "\"";
            }
            // push 到物理栈，同时把值登记进变量寄存器（供算术/间接读取）
            return "push_stack(" + target + ", " + valExpr + "); var_set(\"" + base + "\", " + valExpr + ");";
        }

        if (instr.type == "psh_op") {
            // 目标可能是普通栈名或 @var@ 变量，统一取其基名做寄存器累加
            std::string base = is_variable(instr.arg1) ? var_base_name(instr.arg1) : instr.arg1;
            return "psh_arith(\"" + base + "\", \"" + instr.arg2 + "\", \"" + process_string(instr.arg3) + "\");";
        }
        
        if (instr.type == "pop") return "pop_stack(&" + sanitize_c_id(instr.arg1) + ");";
        
        if (instr.type == "out_var") {
            return "out_varval(\"" + var_base_name(instr.arg2) + "\");";
        }

        if (instr.type == "out") {
            if (stack_names.count(instr.arg1)) {
                return "print_top(&" + sanitize_c_id(instr.arg1) + ");";
            } else {
                // 旧逻辑 fallback
                return "printf(\"" + process_string(instr.arg1) + "\");";
            }
        }
        
        if (instr.type == "out_lit") {
            std::string content = instr.arg2;
            // Remove surrounding quotes
            if (content.size() >= 2 && content.front() == '"' && content.back() == '"') {
                content = content.substr(1, content.size() - 2);
            }
            // Scan for @var@ patterns and generate code
            std::string code;
            size_t pos = 0;
            while (pos < content.size()) {
                size_t start = content.find('@', pos);
                if (start == std::string::npos) {
                    std::string literal = content.substr(pos);
                    code += "printf(\"" + escape_c_string(literal) + "\");";
                    break;
                }
                size_t end = content.find('@', start + 1);
                if (end == std::string::npos) {
                    std::string literal = content.substr(pos);
                    code += "printf(\"" + escape_c_string(literal) + "\");";
                    break;
                }
                std::string var_name = content.substr(start + 1, end - start - 1);
                if (start > pos) {
                    std::string literal = content.substr(pos, start - pos);
                    code += "printf(\"" + escape_c_string(literal) + "\");";
                }
                code += "printf(\"%s\", get_top_by_name(\"" + var_name + "\"));";
                pos = end + 1;
            }
            return code;
        }
        
        if (instr.type == "otn") return "printf(\"\\n\");";
        if (instr.type == "del") return "free_stack(&" + sanitize_c_id(instr.arg1) + ");";
        if (instr.type == "cal") return qb_func_name(instr.arg1) + "();";
        
        if (instr.type == "jmp") {
            // Left operand: @var@ -> get_top_by_name, numeric literal -> string literal, stack name -> get_top
            std::string l;
            if (is_variable(instr.arg1)) {
                l = "get_top_by_name(\"" + var_base_name(instr.arg1) + "\")";
            } else if (std::isdigit((unsigned char)instr.arg1[0])) {
                l = "\"" + instr.arg1 + "\"";
            } else {
                l = "get_top(&" + sanitize_c_id(instr.arg1) + ")";
            }
            // Right operand: @var@ -> get_top_by_name, numeric literal -> string literal, stack name -> get_top
            std::string r;
            if (is_variable(instr.arg2)) {
                r = "get_top_by_name(\"" + var_base_name(instr.arg2) + "\")";
            } else if (std::isdigit((unsigned char)instr.arg2[0])) {
                r = "\"" + instr.arg2 + "\"";
            } else {
                r = "get_top(&" + sanitize_c_id(instr.arg2) + ")";
            }
            return "if (compare_values(" + l + ", " + r + ", \"" + instr.arg3 + "\")" + ") " + qb_func_name(instr.arg4) + "();";
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


    // Strip surrounding quotes from a token (e.g. "exit" -> exit)
    std::string unquote(const std::string& s) const {
        if (s.size() >= 2 && s.front() == '"' && s.back() == '"') {
            return s.substr(1, s.size() - 2);
        }
        return s;
    }

    // Prefix user-defined function name with qb_ to avoid C library collisions
    std::string qb_func_name(const std::string& name) const {
        return "qb_" + unquote(name);
    }

    // Sanitize a stack name for use as a C identifier
    // (e.g. "100" -> "_100")
    std::string sanitize_c_id(const std::string& name) const {
        if (name.empty()) return name;
        if (std::isdigit((unsigned char)name[0])) {
            return "_" + name;
        }
        for (char c : name) {
            if (!std::isalnum((unsigned char)c) && c != '_') {
                std::string result;
                for (char ch : name) {
                    result += (std::isalnum((unsigned char)ch) || ch == '_') ? ch : '_';
                }
                return result;
            }
        }
        return name;
    }
    std::string get_runtime_code() const {
        return R"(
#ifdef _WIN32
#define strdup _strdup
#endif
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
        if (strcmp(op, "<=") == 0) return ln <= rn;
        if (strcmp(op, ">=") == 0) return ln >= rn;
    } else {
        int cmp = strcmp(lv, rv);
        if (strcmp(op, "=") == 0) return cmp == 0;
        if (strcmp(op, "!=") == 0) return cmp != 0;
        if (strcmp(op, "<") == 0) return cmp < 0;
        if (strcmp(op, ">") == 0) return cmp > 0;
        if (strcmp(op, "<=") == 0) return cmp <= 0;
        if (strcmp(op, ">=") == 0) return cmp >= 0;
    }
    return 0;
}

// --- Variable (@name@) runtime support ---
typedef struct { const char* name; Stack* s; } StackRef;
StackRef __stack_registry[256];
int __stack_registry_len = 0;

void register_stack(const char* name, Stack* s) {
    if (__stack_registry_len < 256) {
        __stack_registry[__stack_registry_len].name = name;
        __stack_registry[__stack_registry_len].s = s;
        __stack_registry_len++;
    }
}

Stack* find_stack(const char* name) {
    for (int i = 0; i < __stack_registry_len; i++) {
        if (strcmp(__stack_registry[i].name, name) == 0) return __stack_registry[i].s;
    }
    return NULL;
}

const char* get_top(Stack* s) {
    return (s != NULL && s->size > 0) ? s->items[s->size - 1] : "";
}

const char* get_top_by_name(const char* name) {
    Stack* s = find_stack(name);
    return (s != NULL && s->size > 0) ? s->items[s->size - 1] : "";
}

void out_variable(Stack* var_stack) {
    if (var_stack == NULL || var_stack->size == 0) {
        printf("(undefined) ");
        return;
    }
    const char* val = var_stack->items[var_stack->size - 1];
    Stack* target = find_stack(val);
    if (target != NULL) {
        if (target->size > 0) printf("%s ", target->items[target->size - 1]);
        else printf("(empty) ");
    } else {
        printf("%s ", val);
    }
}

// --- Variable value register (New Feature 2 backing store) ---
// A per-name numeric register decoupled from the physical stack, so that
// arithmetic on @name@ accumulates a value that survives .
typedef struct { char* name; char* val; } VarVal;
VarVal __var_store[256];
int __var_store_len = 0;

const char* var_get(const char* name) {
    for (int i = 0; i < __var_store_len; i++)
        if (strcmp(__var_store[i].name, name) == 0) return __var_store[i].val;
    return NULL;
}

void var_set(const char* name, const char* val) {
    Stack* s = find_stack(name);
    // keep the physical top in sync when the stack has a value
    if (s != NULL && s->size > 0) {
        free(s->items[s->size - 1]);
        s->items[s->size - 1] = strdup(val);
    }
    for (int i = 0; i < __var_store_len; i++) {
        if (strcmp(__var_store[i].name, name) == 0) {
            free(__var_store[i].val);
            __var_store[i].val = strdup(val);
            return;
        }
    }
    if (__var_store_len < 256) {
        __var_store[__var_store_len].name = strdup(name);
        __var_store[__var_store_len].val = strdup(val);
        __var_store_len++;
    }
}

// out @name@: prefer the numeric register; fall back to the original
// indirect (value-is-a-stack-name) resolution when the register is unset.
void out_varval(const char* name) {
    const char* val = var_get(name);
    if (val == NULL) { out_variable(find_stack(name)); return; }
    Stack* target = find_stack(val);
    if (target != NULL) {
        if (target->size > 0) printf("%s ", target->items[target->size - 1]);
        else printf("(empty) ");
    } else {
        printf("%s ", val);
    }
}

// --- Arithmetic push (New Feature 2): psh <stack> <op> "<operand>" ---
// base = current register value of <stack> (or its stack top, or 0);
// res = base OP operand; the result is written back to the register.
void psh_arith(const char* name, const char* op, const char* operand) {
    double base = 0.0;
    const char* cur = var_get(name);
    if (cur != NULL) {
        base = strtod(cur, NULL);
    } else {
        Stack* s = find_stack(name);
        if (s != NULL && s->size > 0) base = strtod(s->items[s->size - 1], NULL);
    }
    double rhs = strtod(operand, NULL);
    double res = base;
    if (strcmp(op, "+") == 0)      res = base + rhs;
    else if (strcmp(op, "-") == 0) res = base - rhs;
    else if (strcmp(op, "*") == 0) res = base * rhs;
    else if (strcmp(op, "/") == 0) res = (rhs != 0.0) ? (base / rhs) : 0.0;

    char buf[64];
    if (res == (long long)res) snprintf(buf, sizeof(buf), "%lld", (long long)res);
    else                       snprintf(buf, sizeof(buf), "%g", res);
    var_set(name, buf);

}

int compare_values(const char* lv, const char* rv, const char* op) {
    if (lv == NULL || rv == NULL) return 0;
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
        if (strcmp(op, "<=") == 0) return ln <= rn;
        if (strcmp(op, ">=") == 0) return ln >= rn;
    } else {
        int cmp = strcmp(lv, rv);
        if (strcmp(op, "=") == 0) return cmp == 0;
        if (strcmp(op, "!=") == 0) return cmp != 0;
        if (strcmp(op, "<") == 0) return cmp < 0;
        if (strcmp(op, ">") == 0) return cmp > 0;
        if (strcmp(op, "<=") == 0) return cmp <= 0;
        if (strcmp(op, ">=") == 0) return cmp >= 0;
    }
    return 0;
}
)";
    }

public:
    std::string generate(const Program& prog) {
        std::stringstream ss;
        
        bool use_stack = needs_stack_support(prog);

        // Only collect stack names if we actually need them
        if (use_stack) {
            collect_stack_names(prog.instructions);
            for (const auto& pair : prog.functions) {
                collect_stack_names(pair.second);
            }
        }

        // Always include stdio.h
        ss << "#include <stdio.h>\n";
        
        // Conditionally include runtime support
        if (use_stack) {
            ss << get_runtime_code();
            
            // Global stack variables
            for (const auto& name : stack_names) {
                ss << "Stack " << sanitize_c_id(name) << ";\n";
            }
            ss << "\n";
        }

        // Forward declarations
        for (const auto& pair : prog.functions) {
            ss << "void " << qb_func_name(pair.first) << "();\n";
        }
        ss << "\n";

        // Function definitions
        for (const auto& pair : prog.functions) {
            ss << "void " << qb_func_name(pair.first) << "() {\n";
            for (const auto& instr : pair.second) {
                std::string code = generate_instr(instr);
                if (!code.empty()) ss << "    " << code << "\n";
            }
            // If this is the exit function, add return 0; to ensure proper program termination
            // becurse the exit function is the end of the last function, if use the exit(0); the program will exit early and not run the rast of code and not free the stacks.
            if (unquote(pair.first) == "exit") {
                ss << "    return 0;\n";
            }
            ss << "}\n\n";
        }

        // Main function
        ss << "int main() {\n";
        
        // Initialize stacks only if used
        if (use_stack) {
            for (const auto& name : stack_names) {
                ss << "    register_stack(\"" << name << "\", &" << sanitize_c_id(name) << ");\n";
            }
            for (const auto& name : stack_names) {
                ss << "    init_stack(&" << sanitize_c_id(name) << ");\n";
            }
        }

        for (const auto& instr : prog.instructions) {
            std::string code = generate_instr(instr);
            if (!code.empty()) ss << "    " << code << "\n";
        }
        
        // Free stacks only if used
        if (use_stack) {
            for (const auto& name : stack_names) {
                ss << "    free_stack(&" << sanitize_c_id(name) << ");\n";
            }
        }

        // If exit function exists, call it (exit function will call exit(0))
        if (prog.functions.count("exit")) {
            ss << "    qb_exit();\n";
        }

        ss << "    return 0;\n}\n";

        return ss.str();
    }
};