use std::collections::HashMap;
use std::fs;
use std::sync::Mutex;
use crate::instructions::{Instruction, Program, ArithmeticOp, ComparisonOp, DebugConfig};
use std::io::{self, BufRead};

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
// QVM 虚拟机实现
// ============================================================

pub struct QVM {
    stacks: HashMap<String, Vec<String>>,
    host_registry: HostRegistry,
    last_error: Option<String>,
    /// 调试器配置
    debug_config: DebugConfig,
    /// 当前执行的指令索引
    current_instruction_idx: usize,
    /// 当前执行的函数名
    current_function: Option<String>,
}

impl QVM {
    pub fn new() -> Self {
        QVM {
            stacks: HashMap::new(),
            host_registry: HostRegistry::new(),
            last_error: None,
            debug_config: DebugConfig::new(),
            current_instruction_idx: 0,
            current_function: None,
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
                // HostCall 指令执行：调用C++侧注册的宿主函数
                // ============================================================
                Instruction::HostCall { module, func, args } => {
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
