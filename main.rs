// -----------------------------------------------------------------------
// <copyright file="main.rs" company="Quern Project">
//     Copyright 2026 WinXSYPowershell
//
//     Licensed under the Apache License, Version 2.0 (the "License");
//     you may not use this file except in compliance with the License.
//     You may obtain a copy of the License at
//
//         http://www.apache.org/licenses/LICENSE-2.0
//
//     Unless required by applicable law or agreed to in writing, software
//     distributed under the License is distributed on an "AS IS" BASIS,
//     WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//     See the License for the specific language governing permissions and
//     limitations under the License.
// </copyright>
// -----------------------------------------------------------------------
use std::collections::HashMap;
use std::fs;
// use std::io;
use std::path::Path;
use std::sync::Arc;
use uuid::Uuid;
use chrono::Local;
use serde::{Deserialize, Serialize};
use regex::Regex;

// ============================================================================
// 1. 配置管理 (Config.yaml)
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QuernConfig {
    pub security: SecurityConfig,
    pub logging: LoggingConfig,
    pub runtime: RuntimeConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SecurityConfig {
    pub enable_sandbox: bool,
    pub max_loop_iterations: u64,
    pub allowed_import_paths: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LoggingConfig {
    pub level: String, // INFO, WARN, ERROR
    pub output_trace_id: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RuntimeConfig {
    pub default_stack_size: usize,
    pub enable_jit: bool,
}

impl Default for QuernConfig {
    fn default() -> Self {
        QuernConfig {
            security: SecurityConfig {
                enable_sandbox: true,
                max_loop_iterations: 1000000,
                allowed_import_paths: vec!["./scripts".to_string()],
            },
            logging: LoggingConfig {
                level: "INFO".to_string(),
                output_trace_id: true,
            },
            runtime: RuntimeConfig {
                default_stack_size: 1024,
                enable_jit: false,
            },
        }
    }
}

// ============================================================================
// 2. 结构化日志与追踪 (Tracing)
// ============================================================================

#[derive(Debug, Clone)]
pub struct TraceContext {
    pub trace_id: String,
    pub timestamp: String,
}

impl TraceContext {
    pub fn new() -> Self {
        TraceContext {
            trace_id: Uuid::new_v4().to_string(),
            timestamp: Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string(),
        }
    }
}

pub struct Logger {
    config: LoggingConfig,
}

impl Logger {
    pub fn new(config: LoggingConfig) -> Self {
        Logger { config }
    }

    fn format_log(&self, level: &str, msg: &str, trace: &TraceContext) -> String {
        if self.config.output_trace_id {
            format!("[{}] [{}] [Trace: {}] {}", trace.timestamp, level, trace.trace_id, msg)
        } else {
            format!("[{}] [{}] {}", trace.timestamp, level, msg)
        }
    }

    pub fn info(&self, msg: &str, trace: &TraceContext) {
        println!("\x1b[34m{}\x1b[0m", self.format_log("INFO", msg, trace));
    }

    pub fn warn(&self, msg: &str, trace: &TraceContext) {
        eprintln!("\x1b[33m{}\x1b[0m", self.format_log("WARN", msg, trace));
    }

    pub fn error(&self, msg: &str, trace: &TraceContext) {
        eprintln!("\x1b[31m{}\x1b[0m", self.format_log("ERROR", msg, trace));
    }
}

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

// ============================================================================
// 4. 运行时环境 (Quern_Poll Simulation)
// ============================================================================

pub struct ExecutionContext {
    pub variables: HashMap<String, Variable>,
    pub lists: HashMap<String, Vec<QuernValue>>,
    pub dicts: HashMap<String, HashMap<String, QuernValue>>,
    pub classes: HashMap<String, ClassTemplate>,
    pub functions: HashMap<String, FunctionDef>,
    pub aliases: HashMap<String, String>,
    pub logger: Arc<Logger>,
    pub trace: TraceContext,
    pub config: Arc<QuernConfig>,
    pub call_stack: Vec<String>, // 用于溯源
}

#[derive(Debug, Clone)]
pub struct FunctionDef {
    pub name: String,
    pub params: Vec<String>,
    pub body: Vec<String>,
    pub access: AccessLevel,
}

impl ExecutionContext {
    pub fn new(logger: Arc<Logger>, config: Arc<QuernConfig>) -> Self {
        ExecutionContext {
            variables: HashMap::new(),
            lists: HashMap::new(),
            dicts: HashMap::new(),
            classes: HashMap::new(),
            functions: HashMap::new(),
            aliases: HashMap::new(),
            logger,
            trace: TraceContext::new(),
            config,
            call_stack: Vec::new(),
        }
    }

    // 安全的变量定义
    pub fn define_var(&mut self, name: &str, value: QuernValue, access: AccessLevel) -> Result<(), String> {
        if !self.config.security.enable_sandbox && name.starts_with("__") {
            return Err("Access to system variables denied".to_string());
        }

        let var = Variable {
            name: name.to_string(),
            value,
            access,
        };
        self.variables.insert(name.to_string(), var);
        Ok(())
    }

    // 获取变量，检查访问权限
    pub fn get_var(&self, name: &str, current_scope_fn: &str) -> Result<&QuernValue, String> {
        if let Some(var) = self.variables.get(name) {
            match var.access {
                AccessLevel::Public => Ok(&var.value),
                AccessLevel::Private => {
                    // 只有定义它的函数能访问
                    if self.call_stack.last().map_or(false, |fn_name| fn_name == current_scope_fn) {
                        Ok(&var.value)
                    } else {
                        Err(format!("Private variable '{}' access denied in function '{}'", name, current_scope_fn))
                    }
                }
                AccessLevel::Default => {
                     if self.call_stack.last().map_or(false, |fn_name| fn_name == current_scope_fn) {
                        Ok(&var.value)
                    } else {
                        Err(format!("Variable '{}' is local to another scope", name))
                    }
                }
            }
        } else {
            Err(format!("Variable '{}' not found", name))
        }
    }

    // 定义列表
    pub fn define_list(&mut self, name: &str, items: Vec<QuernValue>) {
        self.lists.insert(name.to_string(), items);
    }

    // 定义字典
    pub fn define_dict(&mut self, name: &str, items: HashMap<String, QuernValue>) {
        self.dicts.insert(name.to_string(), items);
    }

    // 应用类模板
    pub fn apply_template(&mut self, template_name: &str) -> Result<(), String> {
        // 1. 先克隆出需要的数据，立即结束对 self.classes 的借用
        let template = self.classes.get(template_name).cloned()
            .ok_or_else(|| format!("Class template '{}' not found", template_name))?;

        // 2. 现在可以安全地调用 &mut self 的方法了
        for (k, v) in &template.variables {
            self.define_var(k, v.clone(), AccessLevel::Default)?;
        }
        for (k, v) in &template.lists {
            self.define_list(k, v.clone());
        }
        for (k, v) in &template.dicts {
            self.define_dict(k, v.clone());
        }
        Ok(())
    }
}

// ============================================================================
// 5. 解析器与执行器 (Quern_Interpreter)
// ============================================================================

pub struct Interpreter {
    ctx: ExecutionContext,
}

impl Interpreter {
    pub fn new(config: Arc<QuernConfig>) -> Self {
        let logger = Arc::new(Logger::new(config.logging.clone()));
        let ctx = ExecutionContext::new(logger.clone(), config.clone());
        Interpreter { ctx }
    }

    // 主入口：Run
    pub fn run(&mut self, script_path: &str) -> Result<(), String> {
        self.ctx.logger.info(&format!("Starting execution of: {}", script_path), &self.ctx.trace);

        // 1. 读取文件
        let content = fs::read_to_string(script_path)
            .map_err(|e| format!("Failed to read script: {}", e))?;

        // 2. 预处理：处理 Import
        let processed_content = self.handle_imports(&content, Path::new(script_path).parent().unwrap())?;

        // 3. 解析结构：提取 Main 函数
        let main_func = self.extract_main_function(&processed_content)?;

        // 4. 注册所有顶层函数和类
        self.register_global_definitions(&processed_content)?;

        // 5. 执行 Main
        self.execute_function_body("Main", &main_func.body)
    }

    fn handle_imports(&self, content: &str, base_path: &Path) -> Result<String, String> {
        let mut result = String::new();
        let import_re = Regex::new(r#"Import\s+"([^"]+)"|"#).unwrap();

        for line in content.lines() {
            if let Some(caps) = import_re.captures(line) {
                let file_name = caps.get(1).unwrap().as_str();
                let full_path = base_path.join(file_name);

                // 安全检查：路径遍历
                if !full_path.starts_with(base_path) {
                    return Err("Directory traversal attempt detected in Import".to_string());
                }

                let imported_content = fs::read_to_string(&full_path)
                    .map_err(|e| format!("Import failed: {}", e))?;
                result.push_str(&imported_content);
                result.push('\n');
            } else {
                result.push_str(line);
                result.push('\n');
            }
        }
        Ok(result)
    }

    fn extract_main_function(&self, content: &str) -> Result<FunctionDef, String> {

        let re = Regex::new(r#"Function\s+"Main"\s*\(([^)]*)\)\s*\{([\s\S]*?)\}"#).unwrap();
        if let Some(caps) = re.captures(content) {
            let params_str = caps.get(1).unwrap().as_str();
            let body_str = caps.get(2).unwrap().as_str();

            let params = params_str.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();

            let body_lines: Vec<String> = body_str.lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .collect();

            Ok(FunctionDef {
                name: "Main".to_string(),
                params,
                body: body_lines,
                access: AccessLevel::Public,
            })
        } else {
            Err("No Main function found".to_string())
        }
    }

    fn register_global_definitions(&mut self, content: &str) -> Result<(), String> {
        // 注册 Class
        let class_re = Regex::new(r#"Class\s+"([^"]+)"\s*\{([\s\S]*?)\}"#).unwrap();
        for caps in class_re.captures_iter(content) {
            let name = caps.get(1).unwrap().as_str();
            let body = caps.get(2).unwrap().as_str();
            // 仅解析 Data.Var 和 DataStruct.List/Dict 在 Class 内部，因Class为一个构造函数
            let mut vars = HashMap::new();
            let lists = HashMap::new();
            let dicts = HashMap::new();

            for line in body.lines() {
                let line = line.trim();
                if line.starts_with("Data.Var") {
                    // 解析 Data.Var Type Name = Value
                    if let Some(val) = line.split('=').nth(1) {
                         let parts: Vec<&str> = line.split_whitespace().collect();
                         if parts.len() >= 4 {
                             let var_name = parts[2];
                             let val_str = val.trim();
                             // 类型推断
                             let q_val = self.parse_literal(val_str)?;
                             vars.insert(var_name.to_string(), q_val);
                         }
                    }
                }
                // 类似处理 List 和 Dict...
            }

            self.ctx.classes.insert(name.to_string(), ClassTemplate {
                name: name.to_string(),
                variables: vars,
                lists,
                dicts,
            });
        }

        // 注册 Function
        let func_re = Regex::new(r#"Function\s+"([^"]+)"\s*(?:\(([^)]*)\))?\s*(\{[\s\S]*?\})"#).unwrap();
        for caps in func_re.captures_iter(content) {
            let name = caps.get(1).unwrap().as_str();
            if name == "Main" { continue; } // Main 单独处理

            let params_str = caps.get(2).map_or("", |m| m.as_str());
            let body_block = caps.get(3).unwrap().as_str();

            let params = params_str.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();

            let body_lines: Vec<String> = body_block[1..body_block.len()-1].lines() // 去掉 {}
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .collect();

            // 检查访问修饰符 (默认 Public 除非标记 Private)
            let access = if body_block.contains("(Private)") || body_block.contains("Private") {
                AccessLevel::Private
            } else {
                AccessLevel::Public
            };

            self.ctx.functions.insert(name.to_string(), FunctionDef {
                name: name.to_string(),
                params,
                body: body_lines,
                access,
            });
        }
        Ok(())
    }

    fn execute_function_body(&mut self, fn_name: &str, lines: &[String]) -> Result<(), String> {
        self.ctx.call_stack.push(fn_name.to_string());

        let mut i = 0;
        while i < lines.len() {
            let line = &lines[i];

            // 尝试执行 Catch 块逻辑

            match self.execute_line(line) {
                Ok(_) => {},
                Err(e) => {
                    // 检查当前作用域是否有 Catch
                    if self.has_catch_handler(lines, i) {
                        self.ctx.logger.warn(&format!("Caught error in {}: {}", fn_name, e), &self.ctx.trace);
                        // 跳过到 Catch 块执行
                        i = self.find_catch_block_index(lines, i);
                        continue;
                    } else {
                        self.ctx.call_stack.pop();
                        return Err(format!("Panic in {} at line {}: {}", fn_name, i + 1, e));
                    }
                }
            }
            i += 1;
        }

        self.ctx.call_stack.pop();
        Ok(())
    }

    fn has_catch_handler(&self, lines: &[String], current_idx: usize) -> bool {
        // 在当前函数剩余部分查找 "Catch"
        for j in current_idx..lines.len() {
            if lines[j].starts_with("Catch") {
                return true;
            }
        }
        false
    }

    fn find_catch_block_index(&self, lines: &[String], current_idx: usize) -> usize {
        for j in current_idx..lines.len() {
            if lines[j].starts_with("Catch") {
                return j + 1; // 返回 Catch 块的第一行代码
            }
        }
        lines.len()
    }

    fn execute_line(&mut self, line: &str) -> Result<(), String> {
        let line = line.trim();
        if line.is_empty() { return Ok(()); }

        // 1. Console Output
        if line.starts_with("Console.Info") {
            let content = self.extract_paren_content(line)?;
            let resolved = self.resolve_value(&content)?;
            self.ctx.logger.info(&resolved.to_string(), &self.ctx.trace);
            return Ok(());
        }

        // 2. Variable Definition: Data.Var Type Name = Content
        if line.starts_with("Data.Var") {
            return self.handle_var_def(line);
        }

        // 3. List Definition: DataStruct.List "Name" = [...]
        if line.starts_with("DataStruct.List") {
            return self.handle_list_def(line);
        }

        // 4. Dict Definition: DataStruct.Dict "Name" = [...]
        if line.starts_with("DataStruct.Dict") {
            return self.handle_dict_def(line);
        }

        // 5. Loop: Loop (Count) { ... }
        if line.starts_with("Loop") {
            return self.handle_loop(line);
        }

        // 6. If/Else
        if line.starts_with("If") {
            return self.handle_if(line);
        }

        // 7. Entrust
        if line.starts_with("Entrust") {
            return self.handle_entrust(line);
        }

        // 8. Template
        if line.starts_with("Template") {
            let processed_line = line.replace("Template", "");
            let class_name = processed_line.trim().trim_matches('"').trim();
            return self.ctx.apply_template(class_name);
        }

        // 9. Alias
        if line.starts_with("Alias") {
             let parts: Vec<&str> = line.split('=').collect();
             if parts.len() == 2 {
                 let processed_alias = parts[0].replace("Alias", "");
                 let alias_name = processed_alias.trim().trim_matches('"').trim();

                  // 关键修复：为 parts[1] 也创建一个持久的 String 变量
                  let processed_target = parts[1].to_string();
                  let target = processed_target.trim().trim_matches('"').trim();

                  self.ctx.aliases.insert(alias_name.to_string(), target.to_string());
                  }
                  return Ok(());
        }

        // 10. Function Call (Simple)
        // 检查是否是已知函数调用
        let function_to_call: Option<(String, Vec<String>)> = self.ctx.functions
            .iter()
            .find(|(name, _def)| line.starts_with(*name))
            .map(|(name, def)| (name.clone(), def.body.clone()));

        // 2. 如果找到了匹配的函数，则执行它
        if let Some((name, body)) = function_to_call {
            return self.execute_function_body(&name, &body);
        }

        Err(format!("Unknown or unsupported statement: {}", line))
    }

    fn handle_var_def(&mut self, line: &str) -> Result<(), String> {
        // Data.Var Type Name = Value
        // 或者 Data.Var Name = Value (Update)
        let re = Regex::new(r#"Data\.Var\s+(?:(\w+)\s+)?(\w+)\s*=\s*(.+)"#).unwrap();
        if let Some(caps) = re.captures(line) {
            let _type = caps.get(1).map_or("Any", |m| m.as_str()); // 类型目前仅做记录，Rust侧统一为 QuernValue
            let name = caps.get(2).unwrap().as_str();
            let val_str = caps.get(3).unwrap().as_str();

            let value = self.parse_literal(val_str)?;

            // 检查是否已有变量以确定是定义还是更新，以及权限
            let access = if self.ctx.variables.contains_key(name) {
                // 更新时保持原有权限，默认为 Default
                AccessLevel::Default
            } else {
                AccessLevel::Default
            };

            // 检查 Private/Public 前缀
            let final_access = if line.contains("Private") {
                AccessLevel::Private
            } else if line.contains("Public") {
                AccessLevel::Public
            } else {
                access
            };

            self.ctx.define_var(name, value, final_access)
        } else {
            Err(format!("Invalid Var definition: {}", line))
        }
    }

    fn handle_list_def(&mut self, line: &str) -> Result<(), String> {
        // DataStruct.List "Name" = [Item1, Item2]
        let re = Regex::new(r#"DataStruct\.List\s+"(\w+)"\s*=\s*\[(.*)\]"#).unwrap();
        if let Some(caps) = re.captures(line) {
            let name = caps.get(1).unwrap().as_str();
            let items_str = caps.get(2).unwrap().as_str();

            let mut items = Vec::new();
            for item_str in items_str.split(',') {
                let trimmed = item_str.trim();
                if !trimmed.is_empty() {
                    items.push(self.parse_literal(trimmed)?);
                }
            }
            self.ctx.define_list(name, items);
            Ok(())
        } else {
            Err(format!("Invalid List definition: {}", line))
        }
    }

    fn handle_dict_def(&mut self, line: &str) -> Result<(), String> {
        // DataStruct.Dict "Name" = [Key = Val, Key = Val]
        let re = Regex::new(r#"DataStruct\.Dict\s+"(\w+)"\s*=\s*\[(.*)\]"#).unwrap();
        if let Some(caps) = re.captures(line) {
            let name = caps.get(1).unwrap().as_str();
            let items_str = caps.get(2).unwrap().as_str();

            let mut dict = HashMap::new();
            // 分割： Key = Value 格式
            for pair in items_str.split(',') {
                let parts: Vec<&str> = pair.split('=').collect();
                if parts.len() == 2 {
                    let key = parts[0].trim().to_string();
                    let val = self.parse_literal(parts[1].trim())?;
                    dict.insert(key, val);
                }
            }
            self.ctx.define_dict(name, dict);
            Ok(())
        } else {
            Err(format!("Invalid Dict definition: {}", line))
        }
    }

    fn handle_loop(&mut self, _line: &str) -> Result<(), String> {
        // Loop (Count) { Code }
        self.ctx.logger.warn("Loop structure requires AST block support (Simulated)", &self.ctx.trace);
        Ok(())
    }

    fn handle_if(&mut self, _line: &str) -> Result<(), String> {
         self.ctx.logger.warn("If structure requires AST block support (Simulated)", &self.ctx.trace);
         Ok(())
    }

    fn handle_entrust(&mut self, _line: &str) -> Result<(), String> {
         self.ctx.logger.warn("Entrust structure requires AST block support (Simulated)", &self.ctx.trace);
         Ok(())
    }

    fn parse_literal(&self, s: &str) -> Result<QuernValue, String> {
        let s = s.trim();
        if s == "true" { return Ok(QuernValue::Bool(true)); }
        if s == "false" { return Ok(QuernValue::Bool(false)); }
        if s == "null" { return Ok(QuernValue::Null); }

        // String
        if s.starts_with('"') && s.ends_with('"') {
            return Ok(QuernValue::Str(s[1..s.len()-1].to_string()));
        }

        // Int
        if let Ok(i) = s.parse::<i64>() {
            return Ok(QuernValue::Int(i));
        }

        // Float
        if let Ok(f) = s.parse::<f64>() {
            return Ok(QuernValue::Float(f));
        }

        // Reference Variable
        if let Ok(val) = self.ctx.get_var(s, "Parser") {
            return Ok(val.clone());
        }

        Err(format!("Cannot parse literal: {}", s))
    }

    fn resolve_value(&self, s: &str) -> Result<String, String> {
        // 去除 Console.Info(...) 的外层括号内容
        let s = s.trim();
        // 尝试解析为变量
        if let Ok(val) = self.ctx.get_var(s, "Output") {
            return Ok(val.to_string());
        }
        // 否则当作字符串
        Ok(s.trim_matches('"').to_string())
    }

    fn extract_paren_content(&self, line: &str) -> Result<String, String> {
        let start = line.find('(').ok_or("Missing (")?;
        let end = line.rfind(')').ok_or("Missing )")?;
        Ok(line[start+1..end].to_string())
    }
}

// ============================================================================
// 6. 主程序入口
// ============================================================================

fn main() {
    // 加载配置
    let config_path = "Config.yaml";
    let config = if Path::new(config_path).exists() {
        let content = fs::read_to_string(config_path).expect("Failed to read Config.yaml");
        serde_yaml::from_str(&content).expect("Failed to parse Config.yaml")
    } else {
        // 创建默认配置
        let default_config = QuernConfig::default();
        let yaml = serde_yaml::to_string(&default_config).unwrap();
        fs::write(config_path, yaml).unwrap();
        default_config
    };

    let arc_config = Arc::new(config);

    // 解析命令行参数
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 || args[1] != "--Run" {
        eprintln!("Usage: quern --Run <script_path.q>");
        std::process::exit(1);
    }

    let script_path = &args[2];

    let mut interpreter = Interpreter::new(arc_config);

    match interpreter.run(script_path) {
        Ok(_) => {
            std::process::exit(0);
        }
        Err(e) => {
            eprintln!("\x1b[31mFATAL ERROR: {}\x1b[0m", e);
            std::process::exit(1);
        }
    }
}
