use std::env;
use std::collections::HashMap;
use std::fs;
use std::sync::Mutex;
use crate::instructions::{Instruction, Program, ArithmeticOp, ComparisonOp, DebugConfig};
use std::io::{self, Write};
use std::net::{TcpStream, Shutdown};

// ============================================================
// Host API 基础设施
// ============================================================

/// 宿主函数回调trait：C++侧通过实现此trait注册自定义函数
pub trait HostFunction: Send + Sync {
    /// 执行宿主函数，接收参数列表，返回结果或错误
    fn call(&self, args: &[String]) -> Result<String, String>;
}

/// 实现一个空的HostFunction实现，方便测试
impl<F> HostFunction for F
where
    F: Fn(&[String]) -> Result<String, String> + Send + Sync,
{
    fn call(&self, args: &[String]) -> Result<String, String> {
        self(args)
    }
}

/// 宿主API注册表：存储C++侧注册的函数和变量
pub struct HostRegistry {
    /// 注册的宿主函数: module -> func -> HostFunction
    functions: HashMap<String, HashMap<String, Box<dyn HostFunction>>>,
    /// 注册的宿主变量: name -> value
    variables: HashMap<String, String>,
}

impl HostRegistry {
    pub fn new() -> Self {
        HostRegistry {
            functions: HashMap::new(),
            variables: HashMap::new(),
        }
    }

    /// 注册一个宿主函数: host_register_function("Math", "add", ...)
    pub fn register_function<F>(&mut self, module: &str, func: &str, handler: F)
    where
        F: HostFunction + 'static,
    {
        self.functions
            .entry(module.to_string())
            .or_insert_with(HashMap::new)
            .insert(func.to_string(), Box::new(handler));
    }

    /// 注册一个宿主变量
    pub fn register_variable(&mut self, name: &str, value: &str) {
        self.variables.insert(name.to_string(), value.to_string());
    }

    /// 获取已注册的宿主变量
    pub fn get_variable(&self, name: &str) -> Option<&str> {
        self.variables.get(name).map(|s| s.as_str())
    }

    /// 调用已注册的宿主函数
    pub fn call_function(&self, module: &str, func: &str, args: &[String]) -> Result<String, String> {
        if let Some(module_funcs) = self.functions.get(module) {
            if let Some(handler) = module_funcs.get(func) {
                return handler.call(args);
            }
        }
        Err(format!("Host function not registered: {}.{}", module, func))
    }

    /// 列出所有已注册的模块
    pub fn list_modules(&self) -> Vec<String> {
        self.functions.keys().cloned().collect()
    }
}


// ============================================================
// 内存数据库实现
// ============================================================

/// 内存数据库表
pub struct MemoryTable {
    /// 表名
    pub name: String,
    /// 列名
    pub columns: Vec<String>,
    /// 数据行: 每行是 column_name -> value 的映射
    pub rows: Vec<HashMap<String, String>>,
}

impl MemoryTable {
    pub fn new(name: String, columns: Vec<String>) -> Self {
        MemoryTable {
            name,
            columns,
            rows: Vec::new(),
        }
    }

    /// 插入一行数据
    pub fn insert(&mut self, values: HashMap<String, String>) -> Result<(), String> {
        // 检查所有列是否存在
        for col in &self.columns {
            if !values.contains_key(col) {
                return Err(format!("Missing column: {}", col));
            }
        }
        // 检查是否有额外的列
        for val_col in values.keys() {
            if !self.columns.contains(val_col) {
                return Err(format!("Unknown column: {}", val_col));
            }
        }
        self.rows.push(values);
        Ok(())
    }

    /// 查询数据，支持简单条件过滤
    pub fn query(&self, select_cols: &[String], condition: Option<&str>) -> Result<Vec<HashMap<String, String>>, String> {
        let mut results = Vec::new();
        
        for row in &self.rows {
            // 如果有条件，检查是否匹配
            if let Some(cond) = condition {
                if !self.evaluate_condition(row, cond)? {
                    continue;
                }
            }
            
            // 选择列
            let mut result_row = HashMap::new();
            if select_cols.is_empty() {
                // 选择所有列
                for col in &self.columns {
                    if let Some(val) = row.get(col) {
                        result_row.insert(col.clone(), val.clone());
                    }
                }
            } else {
                for col in select_cols {
                    if let Some(val) = row.get(col) {
                        result_row.insert(col.clone(), val.clone());
                    }
                }
            }
            results.push(result_row);
        }
        
        Ok(results)
    }

    /// 更新数据，支持条件
    pub fn update(&mut self, updates: HashMap<String, String>, condition: Option<&str>) -> Result<usize, String> {
        // 检查所有要更新的列是否存在
        for col in updates.keys() {
            if !self.columns.contains(col) {
                return Err(format!("Unknown column: {}", col));
            }
        }
        
        // 如果有条件，先评估哪些行需要更新（避免借用冲突）
        let mut matching_indices = Vec::new();
        if let Some(cond) = condition {
            for (i, row) in self.rows.iter().enumerate() {
                if self.evaluate_condition(row, cond)? {
                    matching_indices.push(i);
                }
            }
        } else {
            for i in 0..self.rows.len() {
                matching_indices.push(i);
            }
        }
        
        let updated_count = matching_indices.len();
        for i in matching_indices {
            for (col, val) in &updates {
                self.rows[i].insert(col.clone(), val.clone());
            }
        }
        
        Ok(updated_count)
    }

    /// 删除数据，支持条件
    pub fn delete(&mut self, condition: Option<&str>) -> Result<usize, String> {
        let initial_len = self.rows.len();
        
        if let Some(cond) = condition {
            // 先收集要删除的索引（避免借用冲突）
            let indices_to_remove: Vec<usize> = self.rows.iter().enumerate()
                .filter(|(_, row)| self.evaluate_condition(row, cond).unwrap_or(false))
                .map(|(i, _)| i)
                .collect();
            
            // 从后往前删除，避免索引偏移
            for i in indices_to_remove.into_iter().rev() {
                self.rows.remove(i);
            }
        } else {
            // 无条件删除所有行
            self.rows.clear();
        }
        
        Ok(initial_len - self.rows.len())
    }

    /// 评估条件表达式
    fn evaluate_condition(&self, row: &HashMap<String, String>, condition: &str) -> Result<bool, String> {
        // 支持的操作符: = != < > =< >=
        let operators = ["!=", ">=", "<=", "=", ">", "<"];
        
        for op in &operators {
            if let Some(pos) = condition.find(op) {
                let left = condition[..pos].trim().to_string();
                let right = condition[pos + op.len()..].trim().to_string();
                
                // 获取左值
                let left_val = match row.get(&left) {
                    Some(v) => v.clone(),
                    None => return Err(format!("Column not found: {}", left)),
                };
                
                // 获取右值（可能是列名或字面量）
                let right_val = match row.get(&right) {
                    Some(v) => v.clone(),
                    None => right.clone(), // 当作字面量
                };
                
                // 尝试数值比较
                if let (Ok(l), Ok(r)) = (left_val.parse::<f64>(), right_val.parse::<f64>()) {
                    return Ok(MemoryTable::compare_values(&l, &r, op));
                }
                
                // 字符串比较
                return Ok(MemoryTable::compare_values(&left_val, &right_val, op));
            }
        }
        
        Err(format!("Invalid condition: {}", condition))
    }

    fn compare_values<T: PartialOrd>(left: &T, right: &T, op: &str) -> bool {
        match op {
            "=" => left == right,
            "!=" => left != right,
            "<" => left < right,
            ">" => left > right,
            "<=" => left <= right,
            ">=" => left >= right,
            _ => false,
        }
    }

    /// 评估条件行(供外部调用)
    pub fn eval_cond_row(&self, row: &HashMap<String, String>, condition: &str) -> bool {
        self.evaluate_condition(row, condition).unwrap_or(false)
    }
}

/// 内存数据库
pub struct MemoryDatabase {
    /// 已打开的数据库名
    pub current_db: Option<String>,
    /// 表集合
    pub tables: HashMap<String, MemoryTable>,
}

impl MemoryDatabase {
    pub fn new() -> Self {
        MemoryDatabase {
            current_db: None,
            tables: HashMap::new(),
        }
    }

    /// 连接数据库
    pub fn connect(&mut self, name: &str) -> String {
        self.current_db = Some(name.to_string());
        format!("Connected to database: {}", name)
    }

    /// 创建表
    pub fn create_table(&mut self, name: &str, columns_str: &str) -> Result<String, String> {
        if self.current_db.is_none() {
            return Err("No database connected. Use Database.connect first.".to_string());
        }
        
        let columns: Vec<String> = columns_str.split(',').map(|s| s.trim().to_string()).collect();
        
        if columns.is_empty() {
            return Err("Columns cannot be empty".to_string());
        }
        
        if self.tables.contains_key(name) {
            return Err(format!("Table '{}' already exists", name));
        }
        
        self.tables.insert(name.to_string(), MemoryTable::new(name.to_string(), columns));
        Ok(format!("Table '{}' created successfully", name))
    }

    /// 插入数据
    pub fn insert(&mut self, table_name: &str, values_str: &str) -> Result<String, String> {
        if self.current_db.is_none() {
            return Err("No database connected. Use Database.connect first.".to_string());
        }
        
        let table = self.tables.get_mut(table_name);
        match table {
            Some(t) => {
                // 解析 key=value 或 key:value 对
                let mut values = HashMap::new();
                for pair in values_str.split(',') {
                    let parts: Vec<&str> = if pair.contains('=') {
                        pair.splitn(2, '=').collect()
                    } else if pair.contains(':') {
                        pair.splitn(2, ':').collect()
                    } else {
                        return Err(format!("Invalid value pair: {}", pair));
                    };
                    if parts.len() != 2 {
                        return Err(format!("Invalid value pair: {}", pair));
                    }
                    values.insert(parts[0].trim().to_string(), parts[1].trim().to_string());
                }
                
                t.insert(values)?;
                Ok(format!("1 row inserted into '{}'", table_name))
            }
            None => Err(format!("Table '{}' not found", table_name)),
        }
    }

    /// 查询数据
    pub fn query(&self, table_name: &str, select_cols: &str, condition: Option<&str>) -> Result<String, String> {
        if self.current_db.is_none() {
            return Err("No database connected. Use Database.connect first.".to_string());
        }
        
        let table = self.tables.get(table_name);
        match table {
            Some(t) => {
                // 解析列名: * 表示所有列, 空字符串也表示所有列
                let cols: Vec<String> = {
                    let trimmed = select_cols.trim();
                    if trimmed.is_empty() || trimmed == "*" || trimmed == "*,*" || trimmed == "*," {
                        // 选择所有列
                        t.columns.clone()
                    } else {
                        trimmed.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
                    }
                };
                
                let results = t.query(&cols, condition)?;
                
                // 格式化为可读字符串
                if results.is_empty() {
                    return Ok("Query returned 0 rows".to_string());
                }
                
                let mut output = String::new();
                for (i, row) in results.iter().enumerate() {
                    if i > 0 {
                        output.push('|');
                    }
                    let mut first = true;
                    // 按表列顺序输出
                    for col in &t.columns {
                        if let Some(val) = row.get(col) {
                            if !first {
                                output.push(',');
                            }
                            output.push_str(val);
                            first = false;
                        }
                    }
                    output.push('\n');
                }
                
                Ok(format!("Query returned {} rows:\n{}", results.len(), output))
            }
            None => Err(format!("Table '{}' not found", table_name)),
        }
    }

    /// 更新数据
    pub fn update(&mut self, table_name: &str, updates_str: &str, condition: Option<&str>) -> Result<String, String> {
        if self.current_db.is_none() {
            return Err("No database connected. Use Database.connect first.".to_string());
        }
        
        let table = self.tables.get_mut(table_name);
        match table {
            Some(t) => {
                // 解析 key=value 或 key:value 对
                let mut values = HashMap::new();
                for pair in updates_str.split(',') {
                    let parts: Vec<&str> = if pair.contains('=') {
                        pair.splitn(2, '=').collect()
                    } else if pair.contains(':') {
                        pair.splitn(2, ':').collect()
                    } else {
                        return Err(format!("Invalid update pair: {}", pair));
                    };
                    if parts.len() != 2 {
                        return Err(format!("Invalid update pair: {}", pair));
                    }
                    values.insert(parts[0].trim().to_string(), parts[1].trim().to_string());
                }
                
                // 逐行更新以支持算术操作 (如 salary:+500)
                let matching_indices: Vec<usize> = if let Some(cond) = condition {
                    t.rows.iter()
                        .enumerate()
                        .filter(|(_, row)| t.eval_cond_row(row, cond))
                        .map(|(i, _)| i)
                        .collect()
                } else {
                    (0..t.rows.len()).collect()
                };
                
                for idx in &matching_indices {
                    for (col, val) in &values {
                        if val.starts_with('+') || val.starts_with('-') || val.starts_with('*') || val.starts_with('/') {
                            let op_char = val.chars().next().unwrap();
                            let num_str = &val[1..];
                            if let Some(cur) = t.rows[*idx].get(col) {
                                if let (Ok(cv), Ok(n)) = (cur.parse::<f64>(), num_str.parse::<f64>()) {
                                    let r = match op_char {
                                        '+' => cv + n,
                                        '-' => cv - n,
                                        '*' => cv * n,
                                        '/' => if n == 0.0 { cv } else { cv / n },
                                        _ => cv,
                                    };
                                    t.rows[*idx].insert(col.clone(), format!("{}", r));
                                    continue;
                                }
                            }
                        }
                        t.rows[*idx].insert(col.clone(), val.clone());
                    }
                }
                
                Ok(format!("{} rows updated in '{}'", matching_indices.len(), table_name))
            }
            None => Err(format!("Table '{}' not found", table_name)),
        }
    }

    /// 删除数据
    pub fn delete(&mut self, table_name: &str, condition: Option<&str>) -> Result<String, String> {
        if self.current_db.is_none() {
            return Err("No database connected. Use Database.connect first.".to_string());
        }
        
        let table = self.tables.get_mut(table_name);
        match table {
            Some(t) => {
                let count = t.delete(condition)?;
                Ok(format!("{} rows deleted from '{}'", count, table_name))
            }
            None => Err(format!("Table '{}' not found", table_name)),
        }
    }

    /// 执行SQL风格的命令
    pub fn execute(&mut self, command: &str) -> Result<String, String> {
        let cmd_upper = command.trim().to_uppercase();
        
        if cmd_upper.starts_with("SELECT") {
            // 简单解析 SELECT
            // 格式: SELECT col1,col2 FROM table WHERE condition
            let cmd = command.trim();
            if cmd_upper.starts_with("SELECT *") || cmd_upper.starts_with("SELECT ") {
                // 提取列名
                let from_pos = cmd.find("FROM").unwrap_or(cmd.len());
                let select_part = cmd[6..from_pos].trim();
                
                let rest = &cmd[from_pos..];
                let table_start = rest.find("FROM").unwrap() + 4;
                let table_end = rest[table_start..].find(|c: char| c.is_whitespace() || c == 'W').unwrap_or(rest.len() - table_start);
                let table_name = rest[table_start..table_start + table_end].trim().to_string();
                
                // 检查是否有 WHERE
                let condition = if let Some(where_pos) = rest[table_start + table_end..].find("WHERE") {
                    Some(rest[table_start + table_end + where_pos + 5..].trim().to_string())
                } else {
                    None
                };
                
                return self.query(&table_name, select_part, condition.as_deref());
            }
        }
        
        Err(format!("Unsupported command: {}. Use Database.connect/query/insert/update/delete instead.", command))
    }

    /// 关闭数据库
    pub fn close(&mut self) -> String {
        self.current_db = None;
        self.tables.clear();
        "Database closed".to_string()
    }

    /// 列出所有表
    pub fn list_tables(&self) -> String {
        let mut tables: Vec<&str> = self.tables.keys().map(|s| s.as_str()).collect();
        tables.sort();
        if tables.is_empty() {
            "No tables".to_string()
        } else {
            format!("Tables: {}", tables.join(", "))
        }
    }
}

// ============================================================
// QVM 虚拟机实现
// ============================================================

pub struct QVM {
    stacks: HashMap<String, Vec<String>>,
    host_registry: HostRegistry,
    db: MemoryDatabase,
    last_error: Option<String>,
    /// 调试器配置
    debug_config: DebugConfig,
    /// 当前执行的指令索引
    current_instruction_idx: usize,
    /// 当前执行的函数名
    current_function: Option<String>,
    /// 活跃的 Socket 连接: handle_id -> TcpStream
    connections: HashMap<String, TcpStream>,
    /// 连接句柄计数器
    conn_counter: u64,
}

impl QVM {
    pub fn new() -> Self {
        QVM {
            stacks: HashMap::new(),
            host_registry: HostRegistry::new(),
            db: MemoryDatabase::new(),
            last_error: None,
            debug_config: DebugConfig::new(),
            current_instruction_idx: 0,
            current_function: None,
            connections: HashMap::new(),
            conn_counter: 0,
        }
    }

    /// 设置调试器配置
    pub fn set_debug_config(&mut self, config: DebugConfig) {
        self.debug_config = config;
    }

    /// 获取宿主注册表引用（供C++侧注册函数/变量）
    pub fn host_registry(&mut self) -> &mut HostRegistry {
        &mut self.host_registry
    }

    /// 执行完整程序
    pub fn execute(&mut self, program: &Program) {
        self.last_error = None;
        if let Err(e) = self.execute_instructions(&program.instructions, &program.functions, &program.instruction_line_numbers, None) {
            // [修复 E0382] 先打印再move，避免借用已移动的值
            eprintln!("Runtime Error: {}", e);
            self.last_error = Some(e);
        }
    }

    /// 执行单条命令字符串（FFI入口）
    pub fn run_command(&mut self, command: &str) -> Result<String, String> {
        use crate::parser::Parser;
        let program = Parser::new(command).parse().map_err(|e| e.to_string())?;
        self.execute(&program);
        Ok("OK".to_string())
    }

    /// 执行脚本文件（FFI入口）
    pub fn run_script(&mut self, path: &str) -> Result<String, String> {
        let content = fs::read_to_string(path).map_err(|e| format!("Failed to read script: {}", e))?;
        self.run_command(&content)
    }

    // ============================================================
    // 调试器方法
    // ============================================================

    /// 打印当前所有栈的状态（--debug 模式）
    fn print_stack_state(&self, instruction_idx: usize, line_number: usize) {
        eprintln!("\n=== Debugger State (instruction #{}, line {}) ===", instruction_idx, line_number);
        eprintln!("--- Stacks ---");
        if self.stacks.is_empty() {
            eprintln!("(no stacks)");
        } else {
            let mut sorted_stacks: Vec<_> = self.stacks.iter().collect();
            sorted_stacks.sort_by_key(|(name, _)| name.clone());
            for (name, stack) in sorted_stacks {
                if stack.is_empty() {
                    eprintln!("  {}: [empty]", name);
                } else {
                    let top = stack.last().unwrap();
                    eprintln!("  {}: [{} ... {}] (top={})", name, stack.len(), top, top);
                }
            }
        }
        eprintln!("=== End Debugger State ===\n");
    }

    /// 打印指定栈的完整内容（--PrintStack 功能）
    pub fn print_stack_contents(&self, stack_name: &str) {
        if let Some(stack) = self.stacks.get(stack_name) {
            eprintln!("\n=== Stack '{}' Contents ===", stack_name);
            if stack.is_empty() {
                eprintln!("  (empty)");
            } else {
                for (i, val) in stack.iter().enumerate() {
                    eprintln!("  [{}] {}", i, val);
                }
            }
            eprintln!("=== End Stack '{}' ===\n", stack_name);
        } else {
            eprintln!("Warning: Stack '{}' does not exist", stack_name);
        }
    }

    /// 检查是否应该暂停（断点或单步模式）
    fn check_debug_pause(&self, line_number: usize) -> bool {
        if self.debug_config.breakpoints.contains(&line_number) {
            eprintln!("\n>>> Breakpoint hit at line {} <<<", line_number);
            eprintln!("Press Enter to continue, or type 'continue' to resume:");
            let mut input = String::new();
            io::stdin().read_line(&mut input).unwrap();
            return true;
        }

        if self.debug_config.single_step {
            eprintln!("\n>>> Single step: press Enter to continue <<<");
            let mut input = String::new();
            io::stdin().read_line(&mut input).unwrap();
            return true;
        }

        false
    }

    fn execute_instructions(
        &mut self,
        instructions: &[Instruction],
        functions: &HashMap<String, Vec<Instruction>>,
        line_numbers: &[usize],
        function_name: Option<String>,
    ) -> Result<(), String> {
        self.current_function = function_name;
        for (idx, instr) in instructions.iter().enumerate() {
            self.current_instruction_idx = idx;

            // 获取当前指令的行号
            let line_number = if idx < line_numbers.len() {
                line_numbers[idx]
            } else {
                0
            };

            // --debug 模式：执行前打印栈状态
            if self.debug_config.debug_mode {
                self.print_stack_state(idx, line_number);
            }

            // 检查断点（在执行指令前检查）
            if self.debug_config.breakpoints.contains(&line_number) {
                eprintln!("\n>>> Breakpoint hit at line {} <<<", line_number);
                eprintln!("Press Enter to continue:");
                let mut input = String::new();
                io::stdin().read_line(&mut input).unwrap();
            }


            match instr {
                Instruction::CreateStack(name) => {
                    self.stacks.entry(name.clone()).or_insert_with(Vec::new);
                }
                Instruction::Push(stack_name, (val, is_var)) => {
                    let value_to_push: String = if *is_var {
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
                    if *is_var {
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
                        self.execute_instructions(body, functions, &[], Some(func_name.clone()))?;
                    } else {
                        eprintln!("Runtime Error: Function '{}' not defined", func_name);
                    }
                }
                Instruction::ConditionalJump { left_stack, right_stack, op, target_func } => {
                    let left_val: Option<String> = if left_stack.1 {
                        self.get_stack_top(&left_stack.0)
                    } else {
                        Some(left_stack.0.clone())
                    };

                    let right_val: Option<String> = if right_stack.1 {
                        self.get_stack_top(&right_stack.0)
                    } else {
                        Some(right_stack.0.clone())
                    };

                    if let (Some(l), Some(r)) = (left_val, right_val) {
                        if self.compare(&l, &r, op.clone()) {
                            if let Some(body) = functions.get(target_func) {
                                self.execute_instructions(body, functions, &[], Some(target_func.clone()))?;
                            } else {
                                eprintln!("Runtime Error: Jump target function '{}' not defined", target_func);
                            }
                            return Ok(());
                        }
                    } else {
                        eprintln!("Runtime Error: Could not retrieve values for jump condition");
                    }
                }
                Instruction::Arithmetic { stack, operator, value } => {
                    if let Some(stack_data) = self.stacks.get_mut(stack) {
                        if let Some(top) = stack_data.pop() {
                            if let (Ok(left), Ok(right)) = (top.parse::<f64>(), value.parse::<f64>()) {
                                let result: f64 = match operator {
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
                // ============================================================
                // HostCall 指令执行：调用C++侧注册的宿主函数 或 内置数据库操作
                // ============================================================
                Instruction::HostCall { module, func, args } => {
                    // 内置 Env 环境变量模块
                    if module == "Env" {
                        match func.as_str() {
                            "get" => {
                                if args.is_empty() {
                                    eprintln!("[HostCall Error] Env.get requires a variable name");
                                } else {
                                    match env::var(&args[0]) {
                                        Ok(val) => println!("[HostCall: Env.{}] {}", func, val),
                                        Err(e) => eprintln!("[HostCall Error] Env.get {}: {}", args[0], e),
                                    }
                                }
                            }
                            "set" => {
                                if args.len() < 2 {
                                    eprintln!("[HostCall Error] Env.set requires variable name and value");
                                } else {
                                    env::set_var(&args[0], &args[1]);
                                    println!("[HostCall: Env.{}] Set {}={}", func, args[0], args[1]);
                                }
                            }
                            "unset" => {
                                if args.is_empty() {
                                    eprintln!("[HostCall Error] Env.unset requires a variable name");
                                } else {
                                    env::remove_var(&args[0]);
                                    println!("[HostCall: Env.{}] Unset {}", func, args[0]);
                                }
                            }
                            "list" => {
                                let mut vars: Vec<_> = env::vars().collect();
                                vars.sort_by_key(|(k, _)| k.clone());
                                if vars.is_empty() {
                                    println!("[HostCall: Env.list] (no environment variables)");
                                } else {
                                    for (key, val) in vars {
                                        println!("[HostCall: Env.list] {}={}", key, val);
                                    }
                                }
                            }
                            _ => {
                                eprintln!("[HostCall Error] Unknown Env function: {}", func);
                                eprintln!("Available Env functions: get, set, unset, list");
                            }
                        }
                    } else if module == "Database" {
                    // 内置数据库操作
                        match func.as_str() {
                            "connect" => {
                                if args.is_empty() {
                                    eprintln!("[HostCall Error] Database.connect requires a database name");
                                } else {
                                    let result = self.db.connect(&args[0]);
                                    println!("[HostCall: Database.{}] {}", func, result);
                                }
                            }
                            "create_table" => {
                                if args.len() < 2 {
                                    eprintln!("[HostCall Error] Database.create_table requires table_name and columns");
                                } else {
                                    match self.db.create_table(&args[0], &args[1]) {
                                        Ok(result) => println!("[HostCall: Database.{}] {}", func, result),
                                        Err(e) => eprintln!("[HostCall Error] Database.create_table: {}", e),
                                    }
                                }
                            }
                            "insert" => {
                                if args.len() < 2 {
                                    eprintln!("[HostCall Error] Database.insert requires table_name and values");
                                } else {
                                    match self.db.insert(&args[0], &args[1]) {
                                        Ok(result) => println!("[HostCall: Database.{}] {}", func, result),
                                        Err(e) => eprintln!("[HostCall Error] Database.insert: {}", e),
                                    }
                                }
                            }
                            "query" => {
                                if args.len() < 2 {
                                    eprintln!("[HostCall Error] Database.query requires table_name and select_columns");
                                } else {
                                    let condition = if args.len() > 2 { Some(&args[2][..]) } else { None };
                                    match self.db.query(&args[0], &args[1], condition) {
                                        Ok(result) => println!("[HostCall: Database.{}] {}", func, result),
                                        Err(e) => eprintln!("[HostCall Error] Database.query: {}", e),
                                    }
                                }
                            }
                            "update" => {
                                if args.len() < 2 {
                                    eprintln!("[HostCall Error] Database.update requires table_name and updates");
                                } else {
                                    let condition = if args.len() > 2 { Some(&args[2][..]) } else { None };
                                    match self.db.update(&args[0], &args[1], condition) {
                                        Ok(result) => println!("[HostCall: Database.{}] {}", func, result),
                                        Err(e) => eprintln!("[HostCall Error] Database.update: {}", e),
                                    }
                                }
                            }
                            "delete" => {
                                if args.is_empty() {
                                    eprintln!("[HostCall Error] Database.delete requires table_name");
                                } else {
                                    let condition = if args.len() > 1 { Some(&args[1][..]) } else { None };
                                    match self.db.delete(&args[0], condition) {
                                        Ok(result) => println!("[HostCall: Database.{}] {}", func, result),
                                        Err(e) => eprintln!("[HostCall Error] Database.delete: {}", e),
                                    }
                                }
                            }
                            "list_tables" => {
                                let result = self.db.list_tables();
                                println!("[HostCall: Database.{}] {}", func, result);
                            }
                            "close" => {
                                let result = self.db.close();
                                println!("[HostCall: Database.{}] {}", func, result);
                            }
                            _ => {
                                match self.host_registry.call_function(module, func, args) {
                                    Ok(result) => {
                                        println!("[HostCall: {}.{}] {}", module, func, result);
                                    }
                                    Err(e) => {
                                        eprintln!("[HostCall Error] Database.{}: {}", func, e);
                                    }
                                }
                            }
                        }
                    } else {
                        // 普通宿主函数调用
                        match self.host_registry.call_function(module, func, args) {
                            Ok(result) => {
                                println!("[HostCall: {}.{}] {}", module, func, result);
                            }
                            Err(e) => {
                                eprintln!("[HostCall Error] {}: {}", module, e);
                            }
                        }
                    }
                }
                // ============================================================
                // Socket 网络指令执行
                // ============================================================
                Instruction::Socket { stack, protocol, address, port } => {
                    // skt @stack@ tcp 127.0.0.1 8080
                    // 创建连接，将连接句柄ID压入栈顶
                    match self.create_connection(protocol, address, *port) {
                        Ok(conn_id) => {
                            if let Some(stack_data) = self.stacks.get_mut(stack) {
                                stack_data.push(conn_id);
                            } else {
                                eprintln!("Runtime Error: Stack '{}' not found", stack);
                            }
                        }
                        Err(e) => {
                            eprintln!("Runtime Error: {}", e);
                        }
                    }
                }
                Instruction::Send { stack } => {
                    // snd @stack@
                    // 栈中顺序: [..., conn_id, data]
                    // 弹出 data，弹出 conn_id，发送数据，conn_id 压回
                    // 注意：先取出 conn_id 再释放栈借用，避免双重可变借用
                    let (conn_id, data) = {
                        if let Some(stack_data) = self.stacks.get_mut(stack) {
                            if stack_data.len() >= 2 {
                                let data = stack_data.pop().unwrap();
                                let conn_id = stack_data.pop().unwrap();
                                // 先压回 conn_id 保持连接可用
                                stack_data.push(conn_id.clone());
                                (conn_id, data)
                            } else {
                                eprintln!("Runtime Error: Stack '{}' has insufficient values for snd (need conn_id and data)", stack);
                                return Ok(());
                            }
                        } else {
                            eprintln!("Runtime Error: Stack '{}' not found", stack);
                            return Ok(());
                        }
                    }; // stack_data 借用在此释放
                    
                    // 现在可以安全地借用 self 来获取连接
                    let send_result = {
                        if let Some(stream) = self.connections.get_mut(&conn_id) {
                            stream.write_all(data.as_bytes())
                        } else {
                            eprintln!("Runtime Error: Connection '{}' not found", conn_id);
                            Err(std::io::Error::new(std::io::ErrorKind::NotFound, "connection not found"))
                        }
                    };
                    
                    if let Err(e) = send_result {
                        eprintln!("Runtime Error: Send failed: {}", e);
                    } else {
                        // 尝试刷新缓冲区
                        if let Some(stream) = self.connections.get_mut(&conn_id) {
                            let _ = stream.flush();
                        }
                    }
                }
                Instruction::Receive { stack } => {
                    // rcv @stack@
                    // 栈中顺序: [..., conn_id]
                    // 弹出 conn_id，接收数据，conn_id 和 received_data 压回
                    let conn_id = {
                        if let Some(stack_data) = self.stacks.get_mut(stack) {
                            if stack_data.len() >= 1 {
                                stack_data.pop().unwrap()
                            } else {
                                eprintln!("Runtime Error: Stack '{}' has no connection ID for rcv", stack);
                                return Ok(());
                            }
                        } else {
                            eprintln!("Runtime Error: Stack '{}' not found", stack);
                            return Ok(());
                        }
                    }; // stack_data 借用在此释放
                    
                    // 接收数据
                    let received = {
                        if let Some(stream) = self.connections.get_mut(&conn_id) {
                            use std::io::Read;
                            let mut buf = [0u8; 4096];
                            match stream.read(&mut buf) {
                                Ok(n) if n > 0 => {
                                    String::from_utf8_lossy(&buf[..n]).to_string()
                                }
                                Ok(_) => {
                                    eprintln!("Runtime Error: Connection closed by remote host");
                                    String::new()
                                }
                                Err(e) => {
                                    eprintln!("Runtime Error: Receive failed: {}", e);
                                    String::new()
                                }
                            }
                        } else {
                            eprintln!("Runtime Error: Connection '{}' not found", conn_id);
                            String::new()
                        }
                    }; // stream 借用在此释放
                    
                    // 压回 conn_id 和接收到的数据
                    if let Some(stack_data) = self.stacks.get_mut(stack) {
                        stack_data.push(conn_id);
                        stack_data.push(received);
                    }
                }
                Instruction::Close { stack } => {
                    // cls @stack@
                    // 弹出栈顶连接句柄ID，关闭连接
                    let conn_id = {
                        if let Some(stack_data) = self.stacks.get_mut(stack) {
                            if stack_data.len() >= 1 {
                                stack_data.pop().unwrap()
                            } else {
                                eprintln!("Runtime Error: Stack '{}' has no connection ID for cls", stack);
                                return Ok(());
                            }
                        } else {
                            eprintln!("Runtime Error: Stack '{}' not found", stack);
                            return Ok(());
                        }
                    }; // stack_data 借用在此释放
                    
                    // 关闭连接
                    if self.connections.remove(&conn_id).is_some() {
                        // 连接已移除并自动关闭
                    } else {
                        eprintln!("Runtime Error: Connection '{}' not found", conn_id);
                    }
                }
            }

            // 执行后检查断点/单步暂停
            if self.debug_config.breakpoints.contains(&line_number) || self.debug_config.single_step {
                self.check_debug_pause(line_number);
            }
        }
        Ok(())
    }

    fn get_stack_top(&self, stack_name: &str) -> Option<String> {
        self.stacks.get(stack_name).and_then(|s| s.last().cloned())
    }

    /// 解析字符串中的 @var@ 模式，替换为对应栈的顶部值
    fn resolve_string(&self, s: &str) -> String {
        let mut result = s.to_string();
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

    /// 获取上次错误信息
    pub fn last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }

    /// 创建新的 Socket 连接，返回连接句柄 ID
    fn create_connection(&mut self, protocol: &str, address: &str, port: u16) -> Result<String, String> {
        // 目前仅支持 TCP
        if protocol != "tcp" && protocol != "TCP" {
            return Err(format!("Unsupported protocol: {}. Only 'tcp' is supported.", protocol));
        }
        let addr = format!("{}:{}", address, port);
        let stream = TcpStream::connect(addr.as_str())
            .map_err(|e| format!("Failed to connect to {}: {}", addr, e))?;
        
        self.conn_counter += 1;
        let conn_id = format!("conn_{}", self.conn_counter);
        self.connections.insert(conn_id.clone(), stream);
        Ok(conn_id)
    }

    /// 从栈顶获取连接句柄 ID，返回连接的可变引用
    fn get_connection_mut(&mut self, _stack_name: &str, conn_id: &str) -> Result<&mut TcpStream, String> {
        if let Some(conn) = self.connections.get_mut(conn_id) {
            Ok(conn)
        } else {
            Err(format!("Socket connection '{}' not found", conn_id))
        }
    }

    /// 移除并关闭指定连接
    fn close_connection(&mut self, _stack_name: &str, conn_id: &str) -> Result<(), String> {
        if let Some(_stream) = self.connections.remove(conn_id) {
            // TcpStream drop 时会自动关闭
            Ok(())
        } else {
            Err(format!("Socket connection '{}' not found", conn_id))
        }
    }
}

impl Drop for QVM {
    fn drop(&mut self) {
        // 自动关闭所有活跃连接
        for (_id, stream) in self.connections.drain() {
            let _ = stream.shutdown(Shutdown::Both);
        }
    }
}

// ============================================================
// C FFI 接口 — 供 C++ 宿主调用
// ============================================================

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

/// opaque句柄：C++侧持有的QVM实例指针
pub struct QVMHandle {
    pub vm: Mutex<QVM>,
}

/// 创建新的QVM实例
/// 返回指向QVMHandle的指针，C++侧用 qvm_free 释放
#[no_mangle]
pub extern "C" fn qvm_new() -> *mut QVMHandle {
    let handle = Box::new(QVMHandle {
        vm: Mutex::new(QVM::new()),
    });
    Box::into_raw(handle)
}

/// 释放QVM实例
#[no_mangle]
pub extern "C" fn qvm_free(handle: *mut QVMHandle) {
    if !handle.is_null() {
        unsafe { drop(Box::from_raw(handle)); }
    }
}

/// 注册宿主函数
/// module: 模块名(C字符串)
/// func: 函数名(C字符串)
/// 返回0成功，-1失败
#[no_mangle]
pub extern "C" fn qvm_register_function(
    handle: *mut QVMHandle,
    module: *const c_char,
    func: *const c_char,
) -> i32 {
    if handle.is_null() || module.is_null() || func.is_null() {
        return -1;
    }
    let module = unsafe { CStr::from_ptr(module) }.to_string_lossy().into_owned();
    let func = unsafe { CStr::from_ptr(func) }.to_string_lossy().into_owned();

    // [修复 E0133] 裸指针 .as_ref() 需要 unsafe 块
    let mut vm = match unsafe { handle.as_ref() } {
        Some(h) => match h.vm.lock() {
            Ok(guard) => guard,
            Err(_) => return -1,
        },
        None => return -1,
    };

    // [修复 E0373] 闭包借用局部变量 func，生命周期不够
    // 使用 move 闭包 + clone 让闭包拥有 func 的所有权
    let func_owned = func.clone();
    vm.host_registry().register_function(&module, &func, move |_args: &[String]| {
        Ok(format!("[{}] registered", func_owned))
    });
    0
}

/// 注册宿主变量
/// name: 变量名, value: 变量值
#[no_mangle]
pub extern "C" fn qvm_register_variable(
    handle: *mut QVMHandle,
    name: *const c_char,
    value: *const c_char,
) -> i32 {
    if handle.is_null() || name.is_null() || value.is_null() {
        return -1;
    }
    let name = unsafe { CStr::from_ptr(name) }.to_string_lossy().into_owned();
    let value = unsafe { CStr::from_ptr(value) }.to_string_lossy().into_owned();

    // [修复 E0133] 裸指针 .as_ref() 需要 unsafe 块
    let mut vm = match unsafe { handle.as_ref() } {
        Some(h) => match h.vm.lock() {
            Ok(guard) => guard,
            Err(_) => return -1,
        },
        None => return -1,
    };

    vm.host_registry().register_variable(&name, &value);
    0
}

/// 执行单条命令
/// command: QVM脚本命令字符串
/// 返回0成功，-1失败
#[no_mangle]
pub extern "C" fn qvm_run_command(
    handle: *mut QVMHandle,
    command: *const c_char,
) -> i32 {
    if handle.is_null() || command.is_null() {
        return -1;
    }
    let command = unsafe { CStr::from_ptr(command) }.to_string_lossy().into_owned();

    // [修复 E0133] 裸指针 .as_ref() 需要 unsafe 块
    let mut vm = match unsafe { handle.as_ref() } {
        Some(h) => match h.vm.lock() {
            Ok(guard) => guard,
            Err(_) => return -1,
        },
        None => return -1,
    };

    match vm.run_command(&command) {
        Ok(_) => 0,
        Err(e) => {
            eprintln!("qvm_run_command error: {}", e);
            -1
        }
    }
}

/// 执行脚本文件
/// path: 脚本文件路径
#[no_mangle]
pub extern "C" fn qvm_run_script(
    handle: *mut QVMHandle,
    path: *const c_char,
) -> i32 {
    if handle.is_null() || path.is_null() {
        return -1;
    }
    let path = unsafe { CStr::from_ptr(path) }.to_string_lossy().into_owned();

    // [修复 E0133] 裸指针 .as_ref() 需要 unsafe 块
    let mut vm = match unsafe { handle.as_ref() } {
        Some(h) => match h.vm.lock() {
            Ok(guard) => guard,
            Err(_) => return -1,
        },
        None => return -1,
    };

    match vm.run_script(&path) {
        Ok(_) => 0,
        Err(e) => {
            eprintln!("qvm_run_script error: {}", e);
            -1
        }
    }
}

/// 获取上次错误信息
#[no_mangle]
pub extern "C" fn qvm_get_last_error(handle: *mut QVMHandle) -> *const c_char {
    if handle.is_null() {
        return std::ptr::null();
    }
    // [修复 E0133] 裸指针 .as_ref() 需要 unsafe 块
    let vm = match unsafe { handle.as_ref() } {
        Some(h) => match h.vm.lock() {
            Ok(guard) => guard,
            Err(_) => return std::ptr::null(),
        },
        None => return std::ptr::null(),
    };
    
    if let Some(err) = vm.last_error() {
        // [修复 E0599] Cell::get 要求 T: Copy，String 不实现 Copy
        // 改用 CString::into_raw() 返回字符串指针（调用方负责释放）
        let c_str = CString::new(err.as_bytes()).unwrap();
        c_str.into_raw() as *const c_char
    } else {
        std::ptr::null()
    }
}

/// 列出已注册的宿主模块
#[no_mangle]
pub extern "C" fn qvm_list_modules(handle: *mut QVMHandle) -> *mut *mut c_char {
    if handle.is_null() {
        return std::ptr::null_mut();
    }
    // [修复 E0133] 裸指针 .as_ref() 需要 unsafe 块
    // [修复 E0596] vm 需要 mut 因为 host_registry() 需要 &mut self
    let mut vm = match unsafe { handle.as_ref() } {
        Some(h) => match h.vm.lock() {
            Ok(guard) => guard,
            Err(_) => return std::ptr::null_mut(),
        },
        None => return std::ptr::null_mut(),
    };
    
    let modules = vm.host_registry().list_modules();
    // 返回NULL结尾的字符串数组
    let mut result: Vec<*mut c_char> = modules
        .into_iter()
        .map(|m| CString::new(m).unwrap().into_raw())
        .collect();
    result.push(std::ptr::null_mut());
    Box::into_raw(result.into_boxed_slice()) as *mut *mut c_char
}
