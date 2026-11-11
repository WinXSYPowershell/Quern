// ============================================================================
// 3. 数据类型与作用域 (Var Pool, Dict, List)
// ============================================================================

#[derive(Debug, Clone)]
pub enum QuernValue {
    Null,
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    List(Vec<QuernValue>),
    Dict(HashMap<String, QuernValue>),
    Function(String), // 引用函数名
}

impl std::fmt::Display for QuernValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QuernValue::Null => write!(f, "null"),
            QuernValue::Int(v) => write!(f, "{}", v),
            QuernValue::Float(v) => write!(f, "{}", v),
            QuernValue::Str(v) => write!(f, "{}", v),
            QuernValue::Bool(v) => write!(f, "{}", v),
            QuernValue::List(_) => write!(f, "[List]"),
            QuernValue::Dict(_) => write!(f, "[Dict]"),
            QuernValue::Function(name) => write!(f, "<Fn: {}>", name),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum AccessLevel {
    Private,
    Public,
    Default, // Current Scope
}

#[derive(Debug, Clone)]
pub struct Variable {
    pub name: String,
    pub value: QuernValue,
    pub access: AccessLevel,
}

#[derive(Debug, Clone)]
pub struct ClassTemplate {
    pub name: String,
    pub variables: HashMap<String, QuernValue>,
    pub lists: HashMap<String, Vec<QuernValue>>,
    pub dicts: HashMap<String, HashMap<String, QuernValue>>,
}