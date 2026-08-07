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