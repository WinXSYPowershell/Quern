use std::collections::HashMap;
use crate::errors::SyntaxError;
use crate::instructions::{Instruction, Program, ArithmeticOp, ComparisonOp};

// --- Parser Implementation ---


struct Parser {
    tokens: Vec<String>,
    pos: usize,
    line_map: Vec<usize>, // Maps token index to line number
    source_lines: Vec<String>, // Stores original lines for error reporting
}

impl Parser {
    fn is_variable(s: &str) -> bool {
        s.starts_with('@') && s.ends_with('@') && s.len() > 2
    }
        fn new(input: &str) -> Self {
            let mut tokens = Vec::new();
            let mut line_map = Vec::new();
            let source_lines: Vec<String> = input.lines().map(|l| l.to_string()).collect();

            for (line_idx, line) in input.lines().enumerate() {
                let line_num = line_idx + 1;
                let mut current_token = String::new();
                let mut in_quotes = false; // 标记是否在引号内

                for c in line.chars() {
                    match c {
                        '"' => {
                            // 遇到引号就切换状态
                            in_quotes = !in_quotes;
                            // 引号本身不作为token的一部分
                        }
                        // 如果是空格，并且不在引号内，就认为一个token结束了
                        ' ' if !in_quotes => {
                            if !current_token.is_empty() {
                                tokens.push(current_token.clone());
                                line_map.push(line_num);
                                current_token.clear();
                            }
                        }
                        // 左花括号和右花括号也作为独立的token（不在引号内时）
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
                        // 其他字符都加到当前token里
                        _ => {
                            current_token.push(c);
                        }
                    }
                }
                // 一行结束后，如果还有没处理的token，也加进去
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

    fn parse(mut self) -> Result<Program, SyntaxError> {
        let mut instructions = Vec::new();
        let mut functions = HashMap::new();

        while self.pos < self.tokens.len() {
            let cmd = self.tokens[self.pos].clone();
            
            match cmd.as_str() {
                "crt" => {
                    self.consume("crt").map_err(|e| self.create_error("MissingArgument", 1001, &cmd))?;
                    let name = self.next_arg().map_err(|e| self.create_error("UnexpectedEnd", 1002, &cmd))?;
                    instructions.push(Instruction::CreateStack(name));
                }
                "psh" => {
                    self.consume("psh").map_err(|e| self.create_error("MissingArgument", 1001, &cmd))?;
                    let stack_ref = self.next_arg().map_err(|e| self.create_error("UnexpectedEnd", 1002, &cmd))?;
                    // 检查下一个token是否是运算符
                    let next_token = if self.pos < self.tokens.len() {
                        self.tokens[self.pos].clone()
                    } else {
                        String::new()
                    };
                    
                    if next_token == "+" || next_token == "-" || next_token == "*" || next_token == "/" {
                        // 算术运算模式: psh stack_ref operator value
                        let op_str = self.next_arg().map_err(|e| self.create_error("UnexpectedEnd", 1002, &cmd))?;
                        let val_token = self.next_arg().map_err(|e| self.create_error("UnexpectedEnd", 1002, &cmd))?;
                        let op = ArithmeticOp::from_str(&op_str).map_err(|_| {
                            self.create_error("InvalidOperator", 1006, &op_str)
                        })?;
                        // 如果是变量引用，去掉@符号得到栈名
                        let stack_name = if Self::is_variable(&stack_ref) {
                            stack_ref[1..stack_ref.len()-1].to_string()
                        } else {
                            stack_ref
                        };
                        instructions.push(Instruction::Arithmetic { stack: stack_name, operator: op, value: val_token });
                    } else {
                        // 普通push模式
                        let val_token = self.next_arg().map_err(|e| self.create_error("UnexpectedEnd", 1002, &cmd))?;
                        let is_var = Self::is_variable(&val_token);
                        // 如果是变量，去掉前后的 @ 符号；否则保持原样
                        let val_content = if is_var { val_token[1..val_token.len()-1].to_string() } else { val_token };
                        instructions.push(Instruction::Push(stack_ref, (val_content, is_var)));
                    }
                }
                "pop" => {
                    self.consume("pop").map_err(|e| self.create_error("MissingArgument", 1001, &cmd))?;
                    let stack = self.next_arg().map_err(|e| self.create_error("UnexpectedEnd", 1002, &cmd))?;
                    instructions.push(Instruction::Pop(stack));
                }
                "out" => {
                    self.consume("out").map_err(|e| self.create_error("MissingArgument", 1001, &cmd))?;
                    let id_token = self.next_arg().map_err(|e| self.create_error("UnexpectedEnd", 1002, &cmd))?;
                    
                    let is_var = Self::is_variable(&id_token);
                    let id_content = if is_var { id_token[1..id_token.len()-1].to_string() } else { id_token };
                    instructions.push(Instruction::Out((id_content, is_var)));
                }
                "otn" => {
                    self.consume("otn").map_err(|e| self.create_error("ParseError", 1003, &cmd))?;
                    instructions.push(Instruction::PrintNewLine);
                }
                "del" => {
                    self.consume("del").map_err(|e| self.create_error("MissingArgument", 1001, &cmd))?;
                    let name = self.next_arg().map_err(|e| self.create_error("UnexpectedEnd", 1002, &cmd))?;
                    instructions.push(Instruction::DeleteStack(name));
                }
                "fnc" => {
                    self.consume("fnc").map_err(|e| self.create_error("MissingArgument", 1001, &cmd))?;
                    let func_name = self.next_arg().map_err(|e| self.create_error("UnexpectedEnd", 1002, &cmd))?;
                    self.expect("{").map_err(|e| self.create_error("MissingBrace", 1004, &cmd))?;
                    
                    let body = self.parse_block(&mut functions).map_err(|e| self.create_error("BlockParseError", 1005, &cmd))?;
                    functions.insert(func_name, body);
                }
                "cal" => {
                    self.consume("cal").map_err(|e| self.create_error("MissingArgument", 1001, &cmd))?;
                    let func_name = self.next_arg().map_err(|e| self.create_error("UnexpectedEnd", 1002, &cmd))?;
                    instructions.push(Instruction::CallFunction(func_name));
                }
                "jmp" => {
                    // 1. 解析左侧操作数
                    self.consume("jmp").map_err(|e| self.create_error("MissingArgument", 1001, &cmd))?;
                    
                    let left_token = self.next_arg().map_err(|e| self.create_error("UnexpectedEnd", 1002, &cmd))?;
                    let left_is_var = Self::is_variable(&left_token);
                    let left_content = if left_is_var { 
                        left_token[1..left_token.len()-1].to_string() 
                    } else { 
                        left_token 
                    };

                    // 2. 解析右侧操作数
                    let right_token = self.next_arg().map_err(|e| self.create_error("UnexpectedEnd", 1002, &cmd))?;
                    let right_is_var = Self::is_variable(&right_token);
                    let right_content = if right_is_var { 
                        right_token[1..right_token.len()-1].to_string() 
                    } else { 
                        right_token 
                    };

                    // 3. 解析操作符
                    let op_str = self.next_arg().map_err(|e| self.create_error("UnexpectedEnd", 1002, &cmd))?;
                    let op = ComparisonOp::from_str(&op_str).map_err(|_| {
                        self.create_error("InvalidOperator", 1006, &op_str)
                    })?;

                    // 4. 解析目标函数
                    self.consume("cal").map_err(|e| self.create_error("MissingCalKeyword", 1007, &cmd))?;
                    let target_func = self.next_arg().map_err(|e| self.create_error("UnexpectedEnd", 1002, &cmd))?;

                    // 5. 生成指令
                    // 使用正确的字段名 left_stack 和 right_stack
                    instructions.push(Instruction::ConditionalJump {
                        left_stack: (left_content, left_is_var), 
                        right_stack: (right_content, right_is_var), 
                        op, 
                        target_func, 
                    });
                }
                _ => {
                    return Err(self.create_error("UnknownCommand", 1000, &cmd));
                }
            }
        }

        Ok(Program {
            instructions,
            functions,
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

    fn parse_block(&mut self, functions: &mut HashMap<String, Vec<Instruction>>) -> Result<Vec<Instruction>, String> {
        let mut block_instrs = Vec::new();
        
        while self.pos < self.tokens.len() {
            let cmd = self.tokens[self.pos].clone();
            
            if cmd == "}" {
                self.pos += 1; 
                return Ok(block_instrs);
            }

            match cmd.as_str() {
                "crt" => {
                    self.consume("crt")?;
                    let name = self.next_arg()?;
                    block_instrs.push(Instruction::CreateStack(name));
                }
"psh" => {
                    self.consume("psh")?;
                    let stack_ref = self.next_arg()?;
                    // 检查下一个token是否是运算符
                    let next_token = if self.pos < self.tokens.len() {
                        self.tokens[self.pos].clone()
                    } else {
                        String::new()
                    };
                    
                    if next_token == "+" || next_token == "-" || next_token == "*" || next_token == "/" {
                        // 算术运算模式: psh stack_ref operator value
                        let op_str = self.next_arg()?;
                        let val_token = self.next_arg()?;
                        let op = ArithmeticOp::from_str(&op_str).map_err(|_| {
                            self.create_error("InvalidOperator", 1006, &op_str).to_string()
                        })?;
                        // 如果是变量引用，去掉@符号得到栈名
                        let stack_name = if Self::is_variable(&stack_ref) {
                            stack_ref[1..stack_ref.len()-1].to_string()
                        } else {
                            stack_ref
                        };
                        block_instrs.push(Instruction::Arithmetic { stack: stack_name, operator: op, value: val_token });
                    } else {
                        // 普通push模式
                        let val_token = self.next_arg()?;
                        let is_var = Self::is_variable(&val_token);
                        let val_content = if is_var { val_token[1..val_token.len()-1].to_string() } else { val_token };
                        block_instrs.push(Instruction::Push(stack_ref, (val_content, is_var)));
                    }
                }
                "pop" => {
                    self.consume("pop")?;
                    let stack = self.next_arg()?;
                    block_instrs.push(Instruction::Pop(stack));
                }
                "out" => {
                    self.consume("out")?;
                    let id_token = self.next_arg()?;
                    let is_var = Self::is_variable(&id_token);
                    let id_content = if is_var { id_token[1..id_token.len()-1].to_string() } else { id_token };
                    block_instrs.push(Instruction::Out((id_content, is_var)));
                }
                "otn" => {
                    self.consume("otn")?;
                    block_instrs.push(Instruction::PrintNewLine);
                }
                "del" => {
                    self.consume("del")?;
                    let name = self.next_arg()?;
                    block_instrs.push(Instruction::DeleteStack(name));
                }
                "fnc" => {
                    self.consume("fnc")?;
                    let func_name = self.next_arg()?;
                    self.expect("{")?;
                    let body = self.parse_block(functions)?;
                    functions.insert(func_name, body);
                }
                "cal" => {
                    self.consume("cal")?;
                    let func_name = self.next_arg()?;
                    block_instrs.push(Instruction::CallFunction(func_name));
                }
                "jmp" => {
                    self.consume("jmp").map_err(|e| self.create_error("MissingArgument", 1001, &cmd).to_string())?;
                    
                    // 1. 解析左侧操作数
                    let left_token = self.next_arg().map_err(|e| self.create_error("UnexpectedEnd", 1002, &cmd).to_string())?;
                    let left_is_var = Self::is_variable(&left_token);
                    let left_content = if left_is_var { 
                        left_token[1..left_token.len()-1].to_string() 
                    } else { 
                        left_token 
                    };

                    // 2. 解析右侧操作数
                    let right_token = self.next_arg().map_err(|e| self.create_error("UnexpectedEnd", 1002, &cmd).to_string())?;
                    let right_is_var = Self::is_variable(&right_token);
                    let right_content = if right_is_var { 
                        right_token[1..right_token.len()-1].to_string() 
                    } else { 
                        right_token 
                    };

                    // 3. 解析操作符
                    let op_str = self.next_arg().map_err(|e| self.create_error("UnexpectedEnd", 1002, &cmd).to_string())?;
                    let op = ComparisonOp::from_str(&op_str).map_err(|_| {
                        self.create_error("InvalidOperator", 1006, &op_str).to_string()
                    })?;

                    // 4. 解析目标函数
                    self.consume("cal").map_err(|e| self.create_error("MissingCalKeyword", 1007, &cmd).to_string())?;
                    let target_func = self.next_arg().map_err(|e| self.create_error("UnexpectedEnd", 1002, &cmd).to_string())?;

                    // 5. 生成指令
                    block_instrs.push(Instruction::ConditionalJump { 
                        left_stack: (left_content, left_is_var), 
                        right_stack: (right_content, right_is_var), 
                        op, 
                        target_func, 
                    });
                }
                _ => {
                    return Err(format!("Unknown command in block: {}", cmd));
                }
            }
        }
        
        Err("Unmatched braces: expected '}'".to_string())
    }
}
