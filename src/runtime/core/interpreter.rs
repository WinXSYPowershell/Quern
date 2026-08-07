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