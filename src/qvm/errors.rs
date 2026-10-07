use std::fmt;

// --- Error Handling Structures ---


#[derive(Debug)]
struct SyntaxError {
    error_name: String,
    error_code: u32,
    line_number: usize,
    source_line: String,
    token_content: String, // The specific token causing the error
}

impl fmt::Display for SyntaxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SyntaxError [{}]: code={}, line={}, token='{}', source='{}'",
            self.error_name, self.error_code, self.line_number, self.token_content, self.source_line
        )
    }
}

impl std::error::Error for SyntaxError {}


impl SyntaxError {
    fn format_normal(&self, filename: &str) -> String {
        format!(
            "[Error!]In {} Detected {},Code {} at line {}.Source:{}<-[HERE!]{}",
            filename,
            self.error_name,
            self.error_code,
            self.line_number,
            self.source_line.trim(),
            self.token_content
        )
    }

    fn format_verbose(&self, filename: &str) -> String {
        let indent = "   ";
        let arrow_pos = self.find_token_position_in_line();
        
        // Create the underline part
        let mut underline = String::new();
        for _ in 0..arrow_pos {
            underline.push(' ');
        }
        underline.push('^');
        
        // Create the wavy line context (simplified as tildes around the line)
        let wavy = "~".repeat(self.source_line.len() + 4);

        format!(
            "[Error!]In {} Detected {}, Code {}\nAt {}, Source:\n{}\n{}\n{}{}",
            filename,
            self.error_name,
            self.error_code,
            self.line_number,
            wavy,
            self.source_line.trim(),
            indent,
            underline
        )
    }

    // Helper to find where the token starts in the source line string
    fn find_token_position_in_line(&self) -> usize {
        if let Some(pos) = self.source_line.find(&self.token_content) {
            pos
        } else {
            0
        }
    }
}
