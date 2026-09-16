use std::collections::HashMap;
use std::env;
use std::fs;
use std::process;

// --- Error Handling Structures ---

#[derive(Debug)]
struct SyntaxError {
    error_name: String,
    error_code: u32,
    line_number: usize,
    source_line: String,
    token_content: String, // The specific token causing the error
}

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

// --- Instruction Definitions ---

#[derive(Debug, Clone)]
enum ComparisonOp {
    Equal,      // =
    NotEqual,   // !=
    LessThan,   // <
    GreaterThan,// >
    LessEqual,  // =< 
    GreaterEqual,// >=
}

impl ComparisonOp {
    fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "=" => Ok(ComparisonOp::Equal),
            "!=" => Ok(ComparisonOp::NotEqual),
            "<" => Ok(ComparisonOp::LessThan),
            ">" => Ok(ComparisonOp::GreaterThan),
            "=<" => Ok(ComparisonOp::LessEqual),
            ">=" => Ok(ComparisonOp::GreaterEqual),
            _ => Err(format!("Unknown comparison operator: {}", s)),
        }
    }
}

#[derive(Debug, Clone)]
enum Instruction {
    CreateStack(String),
    Push(String, (String, bool)), 
    Pop(String),
    Out((String, bool)), 
    PrintNewLine,
    DeleteStack(String),
    CallFunction(String),
    ConditionalJump {
        left_stack: String,
        left_is_var: bool,
        right_stack: String,
        right_is_var: bool,
        op: ComparisonOp,
        target_func: String,
    },
}

struct Program {
    instructions: Vec<Instruction>,
    functions: HashMap<String, Vec<Instruction>>,
}

// --- VM Implementation ---

struct VM {
    stacks: HashMap<String, Vec<String>>,
}

impl VM {
    fn new() -> Self {
        VM {
            stacks: HashMap::new(),
        }
    }

    fn execute(&mut self, program: &Program) {
        self.execute_instructions(&program.instructions, &program.functions);
    }

    fn execute_instructions(&mut self, instructions: &[Instruction], functions: &HashMap<String, Vec<Instruction>>) {
        for instr in instructions {
            match instr {
                Instruction::CreateStack(name) => {
                    self.stacks.entry(name.clone()).or_insert_with(Vec::new);
                }
                Instruction::Push(stack_name, (val, is_var)) => {
                    let value_to_push = if *is_var {
                        let fetched_val = self.get_stack_top(val);
                        match fetched_val {
                            Some(v) => v,
                            None => {
                                eprintln!("Warning: Variable '{}' not found, pushing empty string.", val);
                                "".to_string()
                            }
                        }
                    } else {
                        val.clone()
                    };
                    
                    if let Some(stack) = self.stacks.get_mut(stack_name) {
                        stack.push(value_to_push);
                    } else {
                        eprintln!("Runtime Error: Stack '{}' not found", stack_name);
                    }
                }
                Instruction::Pop(stack_name) => {
                    if let Some(stack) = self.stacks.get_mut(stack_name) {
                        stack.pop();
                    } else {
                        eprintln!("Runtime Error: Stack '{}' not found", stack_name);
                    }
                }
                Instruction::Out((identifier, is_var)) => {
                    // === 新增：字符串插值支持 ===
                    // 检查字符串中是否包含 @var@ 模式
                    let has_var_pattern = identifier.contains('@') && Self::has_var_pattern(identifier);
                    
                    if has_var_pattern {
                        // 字符串插值：替换所有 @var@ 为栈值
                        let output = self.interpolate_string(identifier);
                        print!("{} ", output);
                    } else if *is_var {
                        // 纯变量模式：@var@ 完整匹配
                        if let Some(var_value) = self.get_stack_top(identifier) {
                            if let Some(target_stack) = self.stacks.get(&var_value) {
                                if let Some(top) = target_stack.last() {
                                    print!("{} ", top);
                                } else {
                                    print!("(empty) ");
                                }
                            } else {
                                print!("{} ", var_value);
                            }
                        } else {
                            print!("(undefined) ");
                        }
                    } else {
                        // 普通栈名或字面量
                        if let Some(stack) = self.stacks.get(identifier) {
                            if let Some(top) = stack.last() {
                                print!("{} ", top);
                            } else {
                                print!("(empty) ");
                            }
                        } else {
                            print!("{} ", identifier);
                        }
                    }
                }
                Instruction::PrintNewLine => {
                    println!();
                }
                Instruction::DeleteStack(name) => {
                    self.stacks.remove(name);
                }
                Instruction::CallFunction(func_name) => {
                    if let Some(body) = functions.get(func_name) {
                        self.execute_instructions(body, functions);
                    } else {
                        eprintln!("Runtime Error: Function '{}' not defined", func_name);
                    }
                }
                Instruction::ConditionalJump { left_stack, left_is_var, right_stack, right_is_var, op, target_func } => {
                    // === 新增：根据 is_var 标记解析操作数 ===
                    let left_val = if *left_is_var {
                        // 变量模式：从同名栈获取顶部值
                        self.get_stack_top(left_stack)
                    } else {
                        // 非变量模式：先尝试栈名，找不到则当作字面量
                        self.get_stack_top(left_stack).or_else(|| Some(left_stack.clone()))
                    };
                    
                    let right_val = if *right_is_var {
                        // 变量模式：从同名栈获取顶部值
                        self.get_stack_top(right_stack)
                    } else {
                        // 非变量模式：先尝试栈名，找不到则当作字面量
                        self.get_stack_top(right_stack).or_else(|| Some(right_stack.clone()))
                    };

                    if let (Some(l), Some(r)) = (left_val, right_val) {
                        if self.compare(&l, &r, op.clone()) {
                            if let Some(body) = functions.get(target_func) {
                                self.execute_instructions(body, functions);
                            } else {
                                eprintln!("Runtime Error: Jump target function '{}' not defined", target_func);
                            }
                        }
                    } else {
                        eprintln!("Runtime Error: Could not retrieve values for jump condition (left: '{}', right: '{}')", left_stack, right_stack);
                    }
                }
            }
        }
    }

    // === 新增：检查字符串是否包含 @var@ 模式 ===
    fn has_var_pattern(s: &str) -> bool {
        let mut chars = s.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '@' {
                // 检查是否形成 @var@ 模式
                let mut var_content = String::new();
                let mut end_found = false;
                for c2 in &mut chars {
                    if c2 == '@' {
                        end_found = true;
                        break;
                    }
                    var_content.push(c2);
                }
                if end_found && !var_content.is_empty() {
                    return true;
                }
            }
        }
        false
    }

    // === 新增：字符串插值 - 替换 @var@ 为栈值 ===
    fn interpolate_string(&self, s: &str) -> String {
        let mut result = s.to_string();
        // 反复查找并替换 @var@ 模式
        loop {
            // 找到第一个 @ 开始的位置
            match result.find('@') {
                Some(start) => {
                    // 检查是否形成完整的 @var@ 模式
                    // 注意：需要从 remaining[1..] 中查找第二个 @，而不是从 remaining 开头查找
                    let remaining = &result[start..];
                    if let Some(end_offset) = remaining.get(1..).and_then(|s| s.find('@')) {
                        // var_name 是从第一个 @ 之后到第二个 @ 之间的内容
                        let var_name = remaining[1..1 + end_offset].to_string();
                        if !var_name.is_empty() {
                            // 找到变量名，获取其值
                            let val = self.get_stack_top(&var_name).unwrap_or_else(|| "(undefined)".to_string());
                            // 第二个 @ 在 remaining 中的绝对位置 = 1 + end_offset
                            // 对应到 result 中的位置 = start + 1 + end_offset
                            // 我们要替换的范围是 [start, start + 1 + end_offset + 1) 即 @varname@
                            let absolute_end = start + 1 + end_offset + 1;
                            result = result[..start].to_string() + &val + &result[absolute_end..];
                            continue;
                        }
                    }
                    // 不是有效的 @var@ 模式，停止插值
                    break;
                }
                None => break,
            }
        }
        result
    }

    fn get_stack_top(&self, stack_name: &str) -> Option<String> {
        self.stacks.get(stack_name).and_then(|s| s.last().cloned())
    }

    fn compare(&self, left: &str, right: &str, op: ComparisonOp) -> bool {
        let l_num = left.parse::<f64>();
        let r_num = right.parse::<f64>();

        if let (Ok(ln), Ok(rn)) = (l_num, r_num) {
            match op {
                ComparisonOp::Equal => (ln - rn).abs() < f64::EPSILON,
                ComparisonOp::NotEqual => (ln - rn).abs() >= f64::EPSILON,
                ComparisonOp::LessThan => ln < rn,
                ComparisonOp::GreaterThan => ln > rn,
                ComparisonOp::LessEqual => ln <= rn,
                ComparisonOp::GreaterEqual => ln >= rn,
            }
        } else {
            match op {
                ComparisonOp::Equal => left == right,
                ComparisonOp::NotEqual => left != right,
                ComparisonOp::LessThan => left < right,
                ComparisonOp::GreaterThan => left > right,
                ComparisonOp::LessEqual => left <= right,
                ComparisonOp::GreaterEqual => left >= right,
            }
        }
    }
}

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
                    let stack = self.next_arg().map_err(|e| self.create_error("UnexpectedEnd", 1002, &cmd))?;
                    let val_token = self.next_arg().map_err(|e| self.create_error("UnexpectedEnd", 1002, &cmd))?;
                    let is_var = Self::is_variable(&val_token);
                    // 如果是变量，去掉前后的 @ 符号；否则保持原样
                    let val_content = if is_var { val_token[1..val_token.len()-1].to_string() } else { val_token };
                    instructions.push(Instruction::Push(stack, (val_content, is_var)));
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
                    self.consume("jmp").map_err(|e| self.create_error("MissingArgument", 1001, &cmd))?;
                    
                    let left_token = self.next_arg().map_err(|e| self.create_error("UnexpectedEnd", 1002, &cmd))?;
                    let right_token = self.next_arg().map_err(|e| self.create_error("UnexpectedEnd", 1002, &cmd))?;
                    let op_str = self.next_arg().map_err(|e| self.create_error("UnexpectedEnd", 1002, &cmd))?;
                    
                    let op = ComparisonOp::from_str(&op_str).map_err(|_| {
                        self.create_error("InvalidOperator", 1006, &op_str)
                    })?;
                    
                    self.consume("cal").map_err(|e| self.create_error("MissingCalKeyword", 1007, &cmd))?;
                    
                    let target_func = self.next_arg().map_err(|e| self.create_error("UnexpectedEnd", 1002, &cmd))?;
                    
                    // === 新增：处理变量语法 @var@ ===
                    let left_is_var = Self::is_variable(&left_token);
                    let left_stack = if left_is_var {
                        left_token[1..left_token.len()-1].to_string()
                    } else {
                        left_token
                    };
                    
                    let right_is_var = Self::is_variable(&right_token);
                    let right_stack = if right_is_var {
                        right_token[1..right_token.len()-1].to_string()
                    } else {
                        right_token
                    };
                    
                    instructions.push(Instruction::ConditionalJump {
                        left_stack,
                        left_is_var,
                        right_stack,
                        right_is_var,
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
                    let stack = self.next_arg()?;
                    let val_token = self.next_arg()?;
                    let is_var = Self::is_variable(&val_token);
                    let val_content = if is_var { val_token[1..val_token.len()-1].to_string() } else { val_token };
                    block_instrs.push(Instruction::Push(stack, (val_content, is_var)));
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
                    self.consume("jmp")?;
                    let left_token = self.next_arg()?;
                    let right_token = self.next_arg()?;
                    let op_str = self.next_arg()?;
                    let op = ComparisonOp::from_str(&op_str)?;
                    self.consume("cal")?;
                    let target_func = self.next_arg()?;
                    
                    // === 新增：处理变量语法 @var@ ===
                    let left_is_var = Self::is_variable(&left_token);
                    let left_stack = if left_is_var {
                        left_token[1..left_token.len()-1].to_string()
                    } else {
                        left_token
                    };
                    
                    let right_is_var = Self::is_variable(&right_token);
                    let right_stack = if right_is_var {
                        right_token[1..right_token.len()-1].to_string()
                    } else {
                        right_token
                    };
                    
                    block_instrs.push(Instruction::ConditionalJump {
                        left_stack,
                        left_is_var,
                        right_stack,
                        right_is_var,
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

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 3 {
        eprintln!("Usage: {} [--Check] [--Verbose] --Run <filename.qb>", args[0]);
        process::exit(1);
    }

    let mut check_mode = false;
    let mut verbose_mode = false;
    let mut filename = String::new();
    let mut run_flag_found = false;

    // Simple argument parser
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--Check" => check_mode = true,
            "--Verbose" => verbose_mode = true,
            "--Run" => {
                run_flag_found = true;
                if i + 1 < args.len() {
                    filename = args[i+1].clone();
                    i += 1; // Skip next arg as it is the filename
                } else {
                    eprintln!("Error: --Run requires a filename");
                    process::exit(1);
                }
            }
            _ => {
                // Ignore unknown flags or treat as error depending on strictness
            }
        }
        i += 1;
    }

    if !run_flag_found || filename.is_empty() {
        eprintln!("Error: Missing --Run <filename>");
        process::exit(1);
    }

    let content = match fs::read_to_string(&filename) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to read file '{}': {}", filename, e);
            process::exit(1);
        }
    };

    let parser = Parser::new(&content);
    
    match parser.parse() {
        Ok(program) => {
            if check_mode {
                println!("Syntax Check Passed: No errors detected in {}", filename);
            } else {
                let mut vm = VM::new();
                vm.execute(&program);
            }
        }
        Err(err) => {
            if verbose_mode {
                eprintln!("{}", err.format_verbose(&filename));
            } else {
                eprintln!("{}", err.format_normal(&filename));
            }
            process::exit(1);
        }
    }
}