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