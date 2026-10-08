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
            
            // New Feature: 'out @var@' -> output the value referenced by a stack variable
            if (is_variable(peek())) {
                std::string var_tok = next_arg();
                return {"out_var", "", var_tok, "", "", ComparisonOp::Equal};
            }

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
            // New Feature 2: arithmetic push -> psh <stack> <op> "<operand>"
            // where <op> is one of + - * /
            if (has_next()) {
                std::string maybe_op = peek();
                if (maybe_op == "+" || maybe_op == "-" || maybe_op == "*" || maybe_op == "/") {
                    consume(maybe_op);
                    std::string operand = next_arg();
                    return {"psh_op", stack, maybe_op, operand, "", ComparisonOp::Equal};
                }
            }
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
                std::string remaining = word;
                while (!remaining.empty()) {
                    size_t brace_pos = remaining.find_first_of("{}");
                    if (brace_pos == std::string::npos) {
                        tokens.push_back({remaining, line_idx, col});
                        col += remaining.size();
                        remaining.clear();
                    } else {
                        if (brace_pos > 0) {
                            tokens.push_back({remaining.substr(0, brace_pos), line_idx, col});
                            col += brace_pos;
                        }
                        tokens.push_back({std::string(1, remaining[brace_pos]), line_idx, col});
                        col++;
                        remaining = remaining.substr(brace_pos + 1);
                    }
                }
            }
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
