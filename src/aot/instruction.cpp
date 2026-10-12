// --- Instruction Definitions ---

// --- Variable (@name@) helpers: stack-pop variable feature ---
static bool is_variable(const std::string& s) {
    return s.size() > 2 && s.front() == '@' && s.back() == '@';
}
static std::string var_base_name(const std::string& s) {
    return s.substr(1, s.size() - 2);
}

enum class ComparisonOp {
    Equal, NotEqual, LessThan, GreaterThan, LessEqual, GreaterEqual
};

ComparisonOp parse_comparison_op(const std::string& s) {
    if (s == "=") return ComparisonOp::Equal;
    if (s == "!=") return ComparisonOp::NotEqual;
    if (s == "<") return ComparisonOp::LessThan;
    if (s == ">") return ComparisonOp::GreaterThan;
    if (s == "<=") return ComparisonOp::LessEqual;
    if (s == ">=") return ComparisonOp::GreaterEqual;
    throw std::runtime_error("Unknown op");
}

std::string comparison_op_to_str(ComparisonOp op) {
    switch (op) {
        case ComparisonOp::Equal: return "=";
        case ComparisonOp::NotEqual: return "!=";
        case ComparisonOp::LessThan: return "<";
        case ComparisonOp::GreaterThan: return ">";
        case ComparisonOp::LessEqual: return "<=";
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
