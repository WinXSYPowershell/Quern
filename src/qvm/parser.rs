use std::collections::HashMap;
use crate::errors::SyntaxError;
use crate::instructions::{Instruction, Program, ArithmeticOp, ComparisonOp};

// --- Parser Implementation ---


pub struct Parser {
    tokens: Vec<String>,
    pos: usize,
    line_map: Vec<usize>, // Maps token index to line number
    source_lines: Vec<String>, // Stores original lines for error reporting
}

impl Parser {
    pub fn is_variable(s: &str) -> bool {
        s.starts_with('@') && s.ends_with('@') && s.len() > 2
    }

    pub fn new(input: &str) -> Self {
        let mut tokens = Vec::new();
        let mut line_map = Vec::new();
        let source_lines: Vec<String> = input.lines().map(|l| l.to_string()).collect();

        for (line_idx, line) in input.lines().enumerate() {
            let line_num = line_idx + 1;
            let mut current_token = String::new();
            let mut in_quotes = false;

            for c in line.chars() {
                match c {
                    '"' => {
                        in_quotes = !in_quotes;
                    }
                    ' ' if !in_quotes => {
                        if !current_token.is_empty() {
                            tokens.push(current_token.clone());
                            line_map.push(line_num);
                            current_token.clear();
                        }
                    }
                    '{' if !in_quotes => {
                        if !current_token.is_empty() {
                            tokens.push(current_token.clone());
                            line_map.push(line_num);
                            current_token.clear();
                        }
                        tokens.push("{".to_string());
                        line_map.push(line_num);
                    }
                    '}' if !in_quotes => {
                        if !current_token.is_empty() {
                            tokens.push(current_token.clone());
                            line_map.push(line_num);
                            current_token.clear();
                        }
                        tokens.push("}".to_string());
                        line_map.push(line_num);
                    }
                    _ => {
                        current_token.push(c);
                    }
                }
            }
            if !current_token.is_empty() {
                tokens.push(current_token);
                line_map.push(line_num);
            }
        }

        Parser {
            tokens,
            pos: 0,
            line_map,
            source_lines,
        }
    }

    fn get_current_line_info(&self) -> (usize, String) {
        if self.pos < self.line_map.len() {
            let line_num = self.line_map[self.pos];
            let line_content = if line_num > 0 && line_num <= self.source_lines.len() {
                self.source_lines[line_num - 1].clone()
            } else {
                "Unknown Line".to_string()
            };
            (line_num, line_content)
        } else {
            (self.source_lines.len(), "EOF".to_string())
        }
    }

    fn create_error(&self, name: &str, code: u32, token: &str) -> SyntaxError {
        let (line_num, source_line) = self.get_current_line_info();
        SyntaxError {
            error_name: name.to_string(),
            error_code: code,
            line_number: line_num,
            source_line,
            token_content: token.to_string(),
        }
    }

    /// 判断一个token是否是指令关键字
    fn is_keyword(token: &str) -> bool {
        matches!(token, "crt" | "psh" | "pop" | "out" | "otn" | "del" | "fnc" | "cal" | "jmp" | "host_call")
    }

    /// 解析函数体块，返回 (指令列表, 行号列表)
    fn parse_block_with_lines(
        &mut self,
        functions: &mut HashMap<String, Vec<Instruction>>,
        function_line_numbers: &mut HashMap<String, Vec<usize>>,
    ) -> Result<(Vec<Instruction>, Vec<usize>), String> {
        let mut block_instrs = Vec::new();
        let mut block_line_numbers = Vec::new();

        while self.pos < self.tokens.len() {
            let cmd = self.tokens[self.pos].clone();
            let current_line = self.line_map[self.pos];

            if cmd == "}" {
                self.pos += 1;
                return Ok((block_instrs, block_line_numbers));
            }

            match cmd.as_str() {
                "crt" => {
                    self.consume("crt")?;
                    let name = self.next_arg()?;
                    block_instrs.push(Instruction::CreateStack(name));
                    block_line_numbers.push(current_line);
                }
                "psh" => {
                    self.consume("psh")?;
                    let stack_ref = self.next_arg()?;
                    let next_token = if self.pos < self.tokens.len() {
                        self.tokens[self.pos].clone()
                    } else {
                        String::new()
                    };

                    if next_token == "+" || next_token == "-" || next_token == "*" || next_token == "/" {
                        let op_str = self.next_arg()?;
                        let val_token = self.next_arg()?;
                        let op = ArithmeticOp::from_str(&op_str).map_err(|_| {
                            self.create_error("InvalidOperator", 1006, &op_str).to_string()
                        })?;
                        let stack_name = if Self::is_variable(&stack_ref) {
                            stack_ref[1..stack_ref.len()-1].to_string()
                        } else {
                            stack_ref
                        };
                        block_instrs.push(Instruction::Arithmetic { stack: stack_name, operator: op, value: val_token });
                        block_line_numbers.push(current_line);
                    } else {
                        let val_token = self.next_arg()?;
                        let is_var = Self::is_variable(&val_token);
                        let val_content = if is_var { val_token[1..val_token.len()-1].to_string() } else { val_token };
                        block_instrs.push(Instruction::Push(stack_ref, (val_content, is_var)));
                        block_line_numbers.push(current_line);
                    }
                }
                "pop" => {
                    self.consume("pop")?;
                    let stack = self.next_arg()?;
                    block_instrs.push(Instruction::Pop(stack));
                    block_line_numbers.push(current_line);
                }
                "out" => {
                    self.consume("out")?;
                    let id_token = self.next_arg()?;
                    let is_var = Self::is_variable(&id_token);
                    let id_content = if is_var { id_token[1..id_token.len()-1].to_string() } else { id_token };
                    block_instrs.push(Instruction::Out((id_content, is_var)));
                    block_line_numbers.push(current_line);
                }
                "otn" => {
                    self.consume("otn")?;
                    block_instrs.push(Instruction::PrintNewLine);
                    block_line_numbers.push(current_line);
                }
                "del" => {
                    self.consume("del")?;
                    let name = self.next_arg()?;
                    block_instrs.push(Instruction::DeleteStack(name));
                    block_line_numbers.push(current_line);
                }
                "fnc" => {
                    self.consume("fnc")?;
                    let func_name = self.next_arg()?;
                    self.expect("{")?;
                    let (body, body_lines) = self.parse_block_with_lines(functions, function_line_numbers)?;
                    functions.insert(func_name.clone(), body);
                    function_line_numbers.insert(func_name, body_lines);
                }
                "cal" => {
                    self.consume("cal")?;
                    let func_name = self.next_arg()?;
                    block_instrs.push(Instruction::CallFunction(func_name));
                    block_line_numbers.push(current_line);
                }
                "jmp" => {
                    self.consume("jmp").map_err(|_| self.create_error("MissingArgument", 1001, &cmd).to_string())?;

                    let left_token = self.next_arg().map_err(|_| self.create_error("UnexpectedEnd", 1002, &cmd).to_string())?;
                    let left_is_var = Self::is_variable(&left_token);
                    let left_content = if left_is_var {
                        left_token[1..left_token.len()-1].to_string()
                    } else {
                        left_token
                    };

                    let right_token = self.next_arg().map_err(|_| self.create_error("UnexpectedEnd", 1002, &cmd).to_string())?;
                    let right_is_var = Self::is_variable(&right_token);
                    let right_content = if right_is_var {
                        right_token[1..right_token.len()-1].to_string()
                    } else {
                        right_token
                    };

                    let op_str = self.next_arg().map_err(|_| self.create_error("UnexpectedEnd", 1002, &cmd).to_string())?;
                    let op = ComparisonOp::from_str(&op_str).map_err(|_| {
                        self.create_error("InvalidOperator", 1006, &op_str).to_string()
                    })?;

                    self.consume("cal").map_err(|_| self.create_error("MissingCalKeyword", 1007, &cmd).to_string())?;
                    let target_func = self.next_arg().map_err(|_| self.create_error("UnexpectedEnd", 1002, &cmd).to_string())?;

                    block_instrs.push(Instruction::ConditionalJump {
                        left_stack: (left_content, left_is_var),
                        right_stack: (right_content, right_is_var),
                        op,
                        target_func,
                    });
                    block_line_numbers.push(current_line);
                }
                "host_call" => {
                    self.consume("host_call").map_err(|_| self.create_error("MissingArgument", 1001, &cmd).to_string())?;
                    let module_func = self.next_arg().map_err(|_| self.create_error("UnexpectedEnd", 1002, &cmd).to_string())?;

                    let parts: Vec<&str> = module_func.splitn(2, '.').collect();
                    if parts.len() != 2 {
                        return Err(format!("Invalid host_call syntax: {}", module_func));
                    }
                    let module = parts[0].to_string();
                    let func = parts[1].to_string();

                    let mut args = Vec::new();
                    while self.pos < self.tokens.len() {
                        let next = &self.tokens[self.pos];
                        if Self::is_keyword(next) || next == "}" {
                            break;
                        }
                        args.push(self.next_arg().map_err(|_| self.create_error("UnexpectedEnd", 1002, &cmd).to_string())?);
                    }

                    block_instrs.push(Instruction::HostCall { module, func, args });
                    block_line_numbers.push(current_line);
                }
                _ => {
                    return Err(format!("Unknown command in block: {}", cmd));
                }
            }
        }

        Err("Unmatched braces: expected '}'".to_string())
    }

    pub fn parse(mut self) -> Result<Program, SyntaxError> {
        let mut instructions = Vec::new();
        let mut instruction_line_numbers = Vec::new();
        let mut functions = HashMap::new();
        let mut function_line_numbers = HashMap::new();

        while self.pos < self.tokens.len() {
            let cmd = self.tokens[self.pos].clone();
            let current_line = self.line_map[self.pos];

            match cmd.as_str() {
                "crt" => {
                    self.consume("crt").map_err(|_| self.create_error("MissingArgument", 1001, &cmd))?;
                    let name = self.next_arg().map_err(|_| self.create_error("UnexpectedEnd", 1002, &cmd))?;
                    instructions.push(Instruction::CreateStack(name));
                    instruction_line_numbers.push(current_line);
                }
                "psh" => {
                    self.consume("psh").map_err(|_| self.create_error("MissingArgument", 1001, &cmd))?;
                    let stack_ref = self.next_arg().map_err(|_| self.create_error("UnexpectedEnd", 1002, &cmd))?;
                    let next_token = if self.pos < self.tokens.len() {
                        self.tokens[self.pos].clone()
                    } else {
                        String::new()
                    };

                    if next_token == "+" || next_token == "-" || next_token == "*" || next_token == "/" {
                        let op_str = self.next_arg().map_err(|_| self.create_error("UnexpectedEnd", 1002, &cmd))?;
                        let val_token = self.next_arg().map_err(|_| self.create_error("UnexpectedEnd", 1002, &cmd))?;
                        let op = ArithmeticOp::from_str(&op_str).map_err(|_| {
                            self.create_error("InvalidOperator", 1006, &op_str)
                        })?;
                        let stack_name = if Self::is_variable(&stack_ref) {
                            stack_ref[1..stack_ref.len()-1].to_string()
                        } else {
                            stack_ref
                        };
                        instructions.push(Instruction::Arithmetic { stack: stack_name, operator: op, value: val_token });
                        instruction_line_numbers.push(current_line);
                    } else {
                        let val_token = self.next_arg().map_err(|_| self.create_error("UnexpectedEnd", 1002, &cmd))?;
                        let is_var = Self::is_variable(&val_token);
                        let val_content = if is_var { val_token[1..val_token.len()-1].to_string() } else { val_token };
                        instructions.push(Instruction::Push(stack_ref, (val_content, is_var)));
                        instruction_line_numbers.push(current_line);
                    }
                }
                "pop" => {
                    self.consume("pop").map_err(|_| self.create_error("MissingArgument", 1001, &cmd))?;
                    let stack = self.next_arg().map_err(|_| self.create_error("UnexpectedEnd", 1002, &cmd))?;
                    instructions.push(Instruction::Pop(stack));
                    instruction_line_numbers.push(current_line);
                }
                "out" => {
                    self.consume("out").map_err(|_| self.create_error("MissingArgument", 1001, &cmd))?;
                    let id_token = self.next_arg().map_err(|_| self.create_error("UnexpectedEnd", 1002, &cmd))?;
                    let is_var = Self::is_variable(&id_token);
                    let id_content = if is_var { id_token[1..id_token.len()-1].to_string() } else { id_token };
                    instructions.push(Instruction::Out((id_content, is_var)));
                    instruction_line_numbers.push(current_line);
                }
                "otn" => {
                    self.consume("otn").map_err(|_| self.create_error("ParseError", 1003, &cmd))?;
                    instructions.push(Instruction::PrintNewLine);
                    instruction_line_numbers.push(current_line);
                }
                "del" => {
                    self.consume("del").map_err(|_| self.create_error("MissingArgument", 1001, &cmd))?;
                    let name = self.next_arg().map_err(|_| self.create_error("UnexpectedEnd", 1002, &cmd))?;
                    instructions.push(Instruction::DeleteStack(name));
                    instruction_line_numbers.push(current_line);
                }
                "fnc" => {
                    self.consume("fnc").map_err(|_| self.create_error("MissingArgument", 1001, &cmd))?;
                    let func_name = self.next_arg().map_err(|_| self.create_error("UnexpectedEnd", 1002, &cmd))?;
                    self.expect("{").map_err(|_| self.create_error("MissingBrace", 1004, &cmd))?;
                    let (body, body_lines) = self.parse_block_with_lines(&mut functions, &mut function_line_numbers).map_err(|_e| self.create_error("BlockParseError", 1005, &cmd))?;
                    functions.insert(func_name.clone(), body);
                    function_line_numbers.insert(func_name, body_lines);
                }
                "cal" => {
                    self.consume("cal").map_err(|_| self.create_error("MissingArgument", 1001, &cmd))?;
                    let func_name = self.next_arg().map_err(|_| self.create_error("UnexpectedEnd", 1002, &cmd))?;
                    instructions.push(Instruction::CallFunction(func_name));
                    instruction_line_numbers.push(current_line);
                }
                "jmp" => {
                    self.consume("jmp").map_err(|_| self.create_error("MissingArgument", 1001, &cmd))?;

                    let left_token = self.next_arg().map_err(|_| self.create_error("UnexpectedEnd", 1002, &cmd))?;
                    let left_is_var = Self::is_variable(&left_token);
                    let left_content = if left_is_var {
                        left_token[1..left_token.len()-1].to_string()
                    } else {
                        left_token
                    };

                    let right_token = self.next_arg().map_err(|_| self.create_error("UnexpectedEnd", 1002, &cmd))?;
                    let right_is_var = Self::is_variable(&right_token);
                    let right_content = if right_is_var {
                        right_token[1..right_token.len()-1].to_string()
                    } else {
                        right_token
                    };

                    let op_str = self.next_arg().map_err(|_| self.create_error("UnexpectedEnd", 1002, &cmd))?;
                    let op = ComparisonOp::from_str(&op_str).map_err(|_| {
                        self.create_error("InvalidOperator", 1006, &op_str)
                    })?;

                    self.consume("cal").map_err(|_| self.create_error("MissingCalKeyword", 1007, &cmd))?;
                    let target_func = self.next_arg().map_err(|_| self.create_error("UnexpectedEnd", 1002, &cmd))?;

                    instructions.push(Instruction::ConditionalJump {
                        left_stack: (left_content, left_is_var),
                        right_stack: (right_content, right_is_var),
                        op,
                        target_func,
                    });
                    instruction_line_numbers.push(current_line);
                }
                "host_call" => {
                    self.consume("host_call").map_err(|_| self.create_error("MissingArgument", 1001, &cmd))?;
                    let module_func = self.next_arg().map_err(|_| self.create_error("UnexpectedEnd", 1002, &cmd))?;

                    let parts: Vec<&str> = module_func.splitn(2, '.').collect();
                    if parts.len() != 2 {
                        return Err(self.create_error("InvalidHostCallSyntax", 1008, &module_func));
                    }
                    let module = parts[0].to_string();
                    let func = parts[1].to_string();

                    let mut args = Vec::new();
                    while self.pos < self.tokens.len() {
                        let next = &self.tokens[self.pos];
                        if Self::is_keyword(next) || next == "}" {
                            break;
                        }
                        args.push(self.next_arg().map_err(|_| self.create_error("UnexpectedEnd", 1002, &cmd))?);
                    }

                    instructions.push(Instruction::HostCall { module, func, args });
                    instruction_line_numbers.push(current_line);
                }
                _ => {
                    return Err(self.create_error("UnknownCommand", 1000, &cmd));
                }
            }
        }

        Ok(Program {
            instructions,
            functions,
            instruction_line_numbers,
            function_line_numbers,
        })
    }

    fn consume(&mut self, expected: &str) -> Result<(), String> {
        if self.pos >= self.tokens.len() {
            return Err("Unexpected end of input".to_string());
        }
        if self.tokens[self.pos] != expected {
            return Err(format!("Expected '{}', got '{}'", expected, self.tokens[self.pos]));
        }
        self.pos += 1;
        Ok(())
    }

    fn next_arg(&mut self) -> Result<String, String> {
        if self.pos >= self.tokens.len() {
            return Err("Unexpected end of input, expected argument".to_string());
        }
        let arg = self.tokens[self.pos].clone();
        self.pos += 1;
        Ok(arg)
    }

    fn expect(&mut self, token: &str) -> Result<(), String> {
        self.consume(token)
    }
}
