use std::collections::HashMap;

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
}

pub struct Program {
    pub instructions: Vec<Instruction>,
    pub functions: HashMap<String, Vec<Instruction>>,
}
