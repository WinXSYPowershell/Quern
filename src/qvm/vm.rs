use std::collections::HashMap;
use crate::instructions::{Instruction, Program, ArithmeticOp, ComparisonOp};

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
