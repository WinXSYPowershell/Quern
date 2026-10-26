use std::collections::{HashMap, HashSet};

// --- Instruction Definitions ---

#[derive(Debug, Clone)]
pub enum ComparisonOp {
    Equal,      // =
    NotEqual,   // !=
    LessThan,   // <
    GreaterThan,// >
    LessEqual,  // =< 
    GreaterEqual,// >=
}

impl ComparisonOp {
    pub fn from_str(s: &str) -> Result<Self, String> {
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
pub enum ArithmeticOp {
    Add,      // +
    Subtract, // -
    Multiply, // *
    Divide,   // /
}

impl ArithmeticOp {
    pub fn from_str(s: &str) -> Result<Self, String> {
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
pub enum Instruction {
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
    // --- Host API: host_call <Module>.<Func> <Args...> ---
    HostCall {
        module: String,
        func: String,
        args: Vec<String>,
    },
}

// ============================================================
// 调试器配置
// ============================================================

/// 调试器配置结构体
#[derive(Debug, Clone)]
pub struct DebugConfig {
    /// --debug 模式：每条指令执行前打印栈状态
    pub debug_mode: bool,
    /// --BreakPoint <line>：断点行号集合
    pub breakpoints: HashSet<usize>,
    /// --Step：单步执行模式
    pub single_step: bool,
    /// --PrintStack <stack_name>：要查看的栈变量名
    pub print_stack: Option<String>,
}

impl DebugConfig {
    pub fn new() -> Self {
        DebugConfig {
            debug_mode: false,
            breakpoints: HashSet::new(),
            single_step: false,
            print_stack: None,
        }
    }

    /// 启用调试模式
    pub fn enable_debug(&mut self) {
        self.debug_mode = true;
    }

    /// 添加断点行号
    pub fn add_breakpoint(&mut self, line: usize) {
        self.breakpoints.insert(line);
    }

    /// 启用单步执行
    pub fn enable_single_step(&mut self) {
        self.single_step = true;
    }

    /// 设置要查看的栈
    pub fn set_print_stack(&mut self, stack_name: String) {
        self.print_stack = Some(stack_name);
    }
}

impl Default for DebugConfig {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Program {
    pub instructions: Vec<Instruction>,
    pub functions: HashMap<String, Vec<Instruction>>,
    /// 每条顶层指令对应的源文件行号
    pub instruction_line_numbers: Vec<usize>,
    /// 每个函数体中指令对应的源文件行号
    pub function_line_numbers: HashMap<String, Vec<usize>>,
}
