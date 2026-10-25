// ============================================================
// qvm.rs - FFI 改进版（支持 C++ 回调函数指针）
// 替换原有的 qvm.rs 文件
// ============================================================

use std::collections::HashMap;
use std::fs;
use std::sync::Mutex;
use crate::instructions::{Instruction, Program, ArithmeticOp, ComparisonOp};

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

    /// 注册一个宿主函数
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
}

impl QVM {
    pub fn new() -> Self {
        QVM {
            stacks: HashMap::new(),
            host_registry: HostRegistry::new(),
            last_error: None,
        }
    }

    /// 获取宿主注册表引用（供Rust侧注册函数/变量）
    pub fn host_registry(&mut self) -> &mut HostRegistry {
        &mut self.host_registry
    }

    /// 执行完整程序
    pub fn execute(&mut self, program: &Program) {
        self.last_error = None;
        if let Err(e) = self.execute_instructions(&program.instructions, &program.functions) {
            self.last_error = Some(e);
            eprintln!("Runtime Error: {}", e);
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

    fn execute_instructions(&mut self, instructions: &[Instruction], functions: &HashMap<String, Vec<Instruction>>) -> Result<(), String> {
        for instr in instructions {
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
                        self.execute_instructions(body, functions)?;
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
                                self.execute_instructions(body, functions)?;
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
// C FFI 接口 - 供 C++ 宿主调用
// ============================================================

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::Arc;

/// 使用 std::ffi::c_void 作为 opaque 类型
use std::os::raw::c_void;

/// 创建新的QVM实例
/// 返回指向QVM的Box指针，C++侧用 qvm_free 释放
#[no_mangle]
pub extern "C" fn qvm_new() -> *mut c_void {
    let vm = Box::new(QVM::new());
    Box::into_raw(vm) as *mut c_void
}

/// 释放QVM实例
#[no_mangle]
pub extern "C" fn qvm_free(handle: *mut c_void) {
    if !handle.is_null() {
        unsafe {
            let _ = Box::from_raw(handle as *mut QVM);
        }
    }
}

/// 注册宿主函数（带C++回调）
/// handle: QVM句柄
/// module: 模块名(C字符串)
/// func:   函数名(C字符串)
/// callback: C++回调函数指针
/// 返回: 0成功，-1失败
///
/// 注意：此版本接受一个函数指针作为回调，C++侧可以传入自定义逻辑
#[no_mangle]
pub extern "C" fn qvm_register_function_callback(
    handle: *mut c_void,
    module: *const c_char,
    func: *const c_char,
    callback: Option<unsafe extern "C" fn(*const *const c_char, i32) -> *const c_char>,
) -> i32 {
    if handle.is_null() || module.is_null() || func.is_null() || callback.is_none() {
        return -1;
    }

    let module = unsafe { CStr::from_ptr(module) }.to_string_lossy().into_owned();
    let func = unsafe { CStr::from_ptr(func) }.to_string_lossy().into_owned();

    // 安全地获取Mutex锁
    let vm_raw = handle as *mut QVM;
    let vm = unsafe { &mut *vm_raw };

    // 包装C回调为Rust HostFunction
    let boxed_callback = Box::new(callback.unwrap());

    vm.host_registry().register_function(&module, &func, move |args| {
        // 准备C字符串数组
        let c_args: Vec<*const c_char> = args
            .iter()
            .map(|s| CString::new(s.as_str()).unwrap().into_raw())
            .collect();

        // 调用C++回调
        let result_ptr = boxed_callback.call(c_args.as_ptr(), args.len() as i32);

        // 释放临时分配的C字符串
        for ptr in c_args {
            if !ptr.is_null() {
                unsafe { drop(CString::from_raw(ptr as *mut c_char)); }
            }
        }

        // 转换回调返回的C字符串为Rust String
        if result_ptr.is_null() {
            Err("Host function returned NULL".to_string())
        } else {
            let result = unsafe { CStr::from_ptr(result_ptr) }.to_string_lossy().into_owned();
            // 释放回调分配的字符串内存
            unsafe { drop(CString::from_raw(result_ptr as *mut c_char)); }
            Ok(result)
        }
    });

    0
}

/// 注册一个宿主函数（简化版，仅注册名称，使用内置 echo 实现）
#[no_mangle]
pub extern "C" fn qvm_register_function(
    handle: *mut c_void,
    module: *const c_char,
    func: *const c_char,
) -> i32 {
    if handle.is_null() || module.is_null() || func.is_null() {
        return -1;
    }

    let module = unsafe { CStr::from_ptr(module) }.to_string_lossy().into_owned();
    let func = unsafe { CStr::from_ptr(func) }.to_string_lossy().into_owned();

    let vm_raw = handle as *mut QVM;
    let vm = unsafe { &mut *vm_raw };

    vm.host_registry().register_function(&module, &func, |_args| {
        Ok(format!("[{}] registered", func))
    });
    0
}

/// 注册宿主变量
#[no_mangle]
pub extern "C" fn qvm_register_variable(
    handle: *mut c_void,
    name: *const c_char,
    value: *const c_char,
) -> i32 {
    if handle.is_null() || name.is_null() || value.is_null() {
        return -1;
    }

    let name = unsafe { CStr::from_ptr(name) }.to_string_lossy().into_owned();
    let value = unsafe { CStr::from_ptr(value) }.to_string_lossy().into_owned();

    let vm_raw = handle as *mut QVM;
    let vm = unsafe { &mut *vm_raw };

    vm.host_registry().register_variable(&name, &value);
    0
}

/// 执行单条命令
#[no_mangle]
pub extern "C" fn qvm_run_command(
    handle: *mut c_void,
    command: *const c_char,
) -> i32 {
    if handle.is_null() || command.is_null() {
        return -1;
    }

    let command = unsafe { CStr::from_ptr(command) }.to_string_lossy().into_owned();

    let vm_raw = handle as *mut QVM;
    let vm = unsafe { &mut *vm_raw };

    match vm.run_command(&command) {
        Ok(_) => 0,
        Err(e) => {
            eprintln!("qvm_run_command error: {}", e);
            -1
        }
    }
}

/// 执行脚本文件
#[no_mangle]
pub extern "C" fn qvm_run_script(
    handle: *mut c_void,
    path: *const c_char,
) -> i32 {
    if handle.is_null() || path.is_null() {
        return -1;
    }

    let path = unsafe { CStr::from_ptr(path) }.to_string_lossy().into_owned();

    let vm_raw = handle as *mut QVM;
    let vm = unsafe { &mut *vm_raw };

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
pub extern "C" fn qvm_get_last_error(handle: *mut c_void) -> *const c_char {
    if handle.is_null() {
        return std::ptr::null();
    }

    let vm_raw = handle as *mut QVM;
    let vm = unsafe { &*vm_raw };

    if let Some(err) = vm.last_error() {
        // 使用线程局部存储避免悬垂指针
        thread_local! {
            static LAST_ERROR: std::cell::RefCell<Option<String>> =
                std::cell::RefCell::new(None);
        }
        LAST_ERROR.with(|cell| {
            *cell.borrow_mut() = Some(err.to_string());
            cell.borrow().as_ref().unwrap().as_ptr() as *const c_char
        })
    } else {
        std::ptr::null()
    }
}

/// 列出已注册的宿主模块
#[no_mangle]
pub extern "C" fn qvm_list_modules(handle: *mut c_void) -> *mut *mut c_char {
    if handle.is_null() {
        return std::ptr::null_mut();
    }

    let vm_raw = handle as *mut QVM;
    let vm = unsafe { &*vm_raw };

    let modules = vm.host_registry().list_modules();
    // 返回NULL结尾的字符串数组
    let mut result: Vec<*mut c_char> = modules
        .into_iter()
        .map(|m| CString::new(m).unwrap().into_raw())
        .collect();
    result.push(std::ptr::null_mut());
    Box::into_raw(result.into_boxed_slice()) as *mut *mut c_char
}

/// 释放 qvm_list_modules 返回的数组
#[no_mangle]
pub extern "C" fn qvm_free_modules(modules: *mut *mut c_char) {
    if !modules.is_null() {
        unsafe {
            let vec = Box::from_raw(modules as *mut Vec<*mut c_char>);
            for ptr in vec.iter() {
                if !ptr.is_null() {
                    drop(CString::from_raw(*ptr));
                }
            }
        }
    }
}

// 导出QVM结构体供rust代码内部使用
pub use QVM;
