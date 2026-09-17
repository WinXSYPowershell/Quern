use std::collections::HashMap;
use std::env;
use std::fs;
use std::process;
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
enum ArithmeticOp {
    Add,      // +
    Subtract, // -
    Multiply, // *
    Divide,   // /
}

impl ArithmeticOp {
    fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "+" => Ok(ArithmeticOp::Add),
            "-" => Ok(ArithmeticOp::Subtract),
            "*" => Ok(ArithmeticOp::Multiply),
            "/" => Ok(ArithmeticOp::Divide),
            _ => Err(format!("Unknown arithmetic operator: {}", s)),
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
    Arithmetic {
        stack: String,
        operator: ArithmeticOp,
        value: String,
    },
    ConditionalJump { 
        left_stack: (String, bool),   // (名称, 是否为变量)
        right_stack: (String, bool),  // (名称, 是否为变量)
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
                    // --- 修正点：处理 Out 的标识符 ---
                    if *is_var {
                        // 如果是变量，先获取变量的值，再把这个值当作栈名去输出
                        // 修正点：传入 &identifier 而不是 identifier.clone()
                        if let Some(var_value) = self.get_stack_top(identifier) {
                            // 变量的值是另一个栈的名字，输出那个栈的顶部元素
                            if let Some(target_stack) = self.stacks.get(&var_value) {
                                if let Some(top) = target_stack.last() {
                                    print!("{} ", top);
                                } else {
                                    print!("(empty) ");
                                }
                            } else {
                                // 如果变量的值不是一个已知的栈名，直接输出变量的值
                                print!("{} ", var_value);
                            }
                        } else {
                            // 如果变量本身不存在，输出空
                            print!("(undefined) ");
                        }
                    } else {
                        // 非变量模式：先进行字符串插值(@var@ -> 实际值)，再尝试栈查找
                        let resolved = self.resolve_string(identifier);
                        if let Some(stack) = self.stacks.get(&resolved) {
                            if let Some(top) = stack.last() {
                                print!("{} ", top);
                            } else {
                                print!("(empty) ");
                            }
                        } else {
                            print!("{} ", resolved);
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
                Instruction::ConditionalJump { left_stack, right_stack, op, target_func } => {
                    // 左侧操作数：变量模式从栈取值，直接模式用字面值
                    let left_val = if left_stack.1 {
                        // 变量模式：从栈中获取值
                        self.get_stack_top(&left_stack.0)
                    } else {
                        // 直接模式：使用字符串作为字面值
                        Some(left_stack.0.clone())
                    };

                    let right_val = if right_stack.1 {
                        self.get_stack_top(&right_stack.0)
                    } else {
                        Some(right_stack.0.clone())
                    };

                    // 修复类型错误：get_stack_top 返回的是 Option<String>
                    if let (Some(l), Some(r)) = (left_val, right_val) {
                        if self.compare(&l, &r, op.clone()) {
                            if let Some(body) = functions.get(target_func) {
                                self.execute_instructions(body, functions);
                            } else {
                                eprintln!("Runtime Error: Jump target function '{}' not defined", target_func);
                            }
                            return;
                        }
                    } else {
                        eprintln!("Runtime Error: Could not retrieve values for jump condition");
                    }
                }
                Instruction::Arithmetic { stack, operator, value } => {
                    if let Some(stack_data) = self.stacks.get_mut(stack) {
                        if let Some(top) = stack_data.pop() {
                            if let (Ok(left), Ok(right)) = (top.parse::<f64>(), value.parse::<f64>()) {
                                let result = match operator {
                                    ArithmeticOp::Add => left + right,
                                    ArithmeticOp::Subtract => left - right,
                                    ArithmeticOp::Multiply => left * right,
                                    ArithmeticOp::Divide => {
                                        if right == 0.0 {
                                            eprintln!("Runtime Error: Division by zero");
                                            0.0
                                        } else {
                                            left / right
                                        }
                                    }
                                };
                                // 格式化结果：整数不显示小数点
                                let result_str = if result.fract() == 0.0 && result.abs() < f64::MAX.exp() {
                                    format!("{}", result as i64)
                                } else {
                                    format!("{}", result)
                                };
                                stack_data.push(result_str);
                            } else {
                                eprintln!("Runtime Error: Cannot perform arithmetic on non-numeric values");
                            }
                        } else {
                            eprintln!("Runtime Error: Stack '{}' is empty", stack);
                        }
                    } else {
                        eprintln!("Runtime Error: Stack '{}' not found", stack);
                    }
                }
            }
        }
    }

    fn get_stack_top(&self, stack_name: &str) -> Option<String> {
        self.stacks.get(stack_name).and_then(|s| s.last().cloned())
    }

    /// 解析字符串中的 @var@ 模式，替换为对应栈的顶部值
    fn resolve_string(&self, s: &str) -> String {
        let mut result = s.to_string();
        // 循环替换所有 @var@ 模式
        while let Some(start) = result.find('@') {
            if let Some(end) = result[start+1..].find('@') {
                let var_name = &result[start+1..start+1+end];
                let var_value = self.get_stack_top(var_name).unwrap_or_default();
                result = result.replace(&format!("@{}@", var_name), &var_value);
            } else {
                break;
            }
        }
        result
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
