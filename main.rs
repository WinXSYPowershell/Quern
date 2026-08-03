
use mlua::{Lua, Function, UserData, HookTriggers, Error as LuaError};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Instant;
use std::fmt;

// 加密与日志依赖
use ed25519_dalek::{Signature, VerifyingKey, SIGNATURE_LENGTH};
use hex;
use tracing::{info, warn, error, span, Level};
use tracing_subscriber::fmt::Subscriber;
use semver::{Version, VersionReq}; // 新增: 语义化版本支持
 // 用于生成安全的随机数
// ============================================================================
// 1. 基础契约与错误定义 (增强版)
// ============================================================================

#[derive(Debug)]
pub enum EngineError {
    IoError(std::io::Error),
    LuaError(mlua::Error),
    PreprocessorError(String),
    SecurityViolation(String),
    SignatureVerificationFailed(String),
    ResourceLimitExceeded(String),
    DependencyError(String),
    CryptoError(String),
    SerializationError(String),
}

impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EngineError::IoError(e) => write!(f, "IO Error: {}", e),
            EngineError::LuaError(e) => write!(f, "Lua Error: {}", e),
            EngineError::PreprocessorError(msg) => write!(f, "Preprocessor Error: {}", msg),
            EngineError::SecurityViolation(msg) => write!(f, "Security Violation: {}", msg),
            EngineError::SignatureVerificationFailed(msg) => write!(f, "Signature Failed: {}", msg),
            EngineError::ResourceLimitExceeded(msg) => write!(f, "Resource Limit: {}", msg),
            EngineError::DependencyError(msg) => write!(f, "Dependency Error: {}", msg),
            EngineError::CryptoError(msg) => write!(f, "Crypto Error: {}", msg),
            EngineError::SerializationError(msg) => write!(f, "Serialization Error: {}", msg),
        }
    }
}

impl From<std::io::Error> for EngineError {
    fn from(err: std::io::Error) -> Self {
        EngineError::IoError(err)
    }
}

impl From<mlua::Error> for EngineError {
    fn from(err: mlua::Error) -> Self {
        // 增强：尝试从 Lua 错误中提取更详细的堆栈信息
        let _msg = err.to_string();
        if let LuaError::RuntimeError(ref trace) = err {
             error!("Lua Runtime Error Trace:\n{}", trace);
        }
        EngineError::LuaError(err)
    }
}

// ============================================================================
// 2. 可观测性组件 (Observability - Phase 3 Core)
// ============================================================================

pub fn init_tracing() {
    let subscriber = Subscriber::builder()
        .with_max_level(Level::DEBUG)
        .with_target(true) // 开启 target 以区分模块
        .with_thread_ids(true)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
}

fn generate_trace_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    // 使用更短的 ID 格式便于阅读
    format!("{:x}", duration.as_nanos() % 0xFFFFFFFFFFFF)
}

#[derive(Clone, Debug)]
pub struct ModProfile {
    pub load_time_ms: u128,
    pub execution_count: u64,
    pub total_cpu_time_ms: u128,
    pub last_error: Option<String>,
}

#[derive(Clone)]
pub struct AuditLogger {
    log_path: PathBuf,
}

impl AuditLogger {
    pub fn new(base_path: &Path) -> Self {
        let log_path = base_path.join("security_audit.log");
        Self { log_path }
    }

    pub fn log_violation(&self, trace_id: &str, message: &str) {
        let entry = format!("[TRACE:{}] VIOLATION: {}\n", trace_id, message);
        fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)
            .and_then(|mut f| std::io::Write::write_all(&mut f, entry.as_bytes()))
            .ok();
        
        warn!(target: "security_audit", trace_id = trace_id, "{}", message);
    }
    
    pub fn log_warning(&self, trace_id: &str, message: &str) {
         let entry = format!("[TRACE:{}] WARNING: {}\n", trace_id, message);
         fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)
            .and_then(|mut f| std::io::Write::write_all(&mut f, entry.as_bytes()))
            .ok();
        warn!(target: "security_temp", trace_id = trace_id, "{}", message);
    }
}

// ============================================================================
// 3. 安全组件：Ed25519 签名验证器
// ============================================================================

const OFFICIAL_PUBLIC_KEY_HEX: &str = "0652786e8a2b51627064150aa600bd772930c18db7eacfd7cfc9859dd386c850";
const TEMP_PUBLIC_KEY_HEX: &str = "2290a60f56e441d7312068598f5fdbdbef5e8647628f886bd4a1e09828d32b8b";

pub struct SignatureVerifier {
    official_key: VerifyingKey,
    temp_key: VerifyingKey,
}

impl SignatureVerifier {
    pub fn new() -> Result<Self, EngineError> {
        let official_bytes = hex::decode(OFFICIAL_PUBLIC_KEY_HEX)
            .map_err(|e| EngineError::CryptoError(format!("Failed to decode official key hex: {}", e)))?;
        let official_array: [u8; 32] = official_bytes.try_into()
            .map_err(|_| EngineError::CryptoError("Official key length invalid".to_string()))?;
        let official_key = VerifyingKey::from_bytes(&official_array)
            .map_err(|e| EngineError::CryptoError(format!("Invalid official public key: {}", e)))?;

        let temp_bytes = hex::decode(TEMP_PUBLIC_KEY_HEX)
            .map_err(|e| EngineError::CryptoError(format!("Failed to decode temp key hex: {}", e)))?;
        let temp_array: [u8; 32] = temp_bytes.try_into()
            .map_err(|_| EngineError::CryptoError("Temp key length invalid".to_string()))?;
        let temp_key = VerifyingKey::from_bytes(&temp_array)
            .map_err(|e| EngineError::CryptoError(format!("Invalid temp public key: {}", e)))?;

        Ok(Self { official_key, temp_key })
    }

    pub fn verify_mod(&self, mod_path: &Path, signature_path: &Path, is_temp: bool, trace_id: &str) -> Result<(), EngineError> {
        if !signature_path.exists() {
            return Err(EngineError::SignatureVerificationFailed(
                format!("Missing signature file (.sig) for: {:?}", mod_path)
            ));
        }

        let mod_content = fs::read(mod_path)?;
        let sig_bytes = fs::read(signature_path)?;

        if sig_bytes.len() != SIGNATURE_LENGTH {
            return Err(EngineError::SignatureVerificationFailed(
                format!("Invalid signature length for: {:?}", mod_path)
            ));
        }

        let mut sig_array = [0u8; SIGNATURE_LENGTH];
        sig_array.copy_from_slice(&sig_bytes);
        let signature = Signature::from_bytes(&sig_array);

        let verifying_key = if is_temp { &self.temp_key } else { &self.official_key };

        match verifying_key.verify_strict(&mod_content, &signature) {
            Ok(_) => {
                if is_temp {
                    warn!(target: "security_verify", trace_id = trace_id, "Temp Mod signature verified (WARNING MODE): {:?}", mod_path);
                } else {
                    info!(target: "security_verify", trace_id = trace_id, "Official Mod signature verified: {:?}", mod_path);
                    println!("Official Mod signature verified.");
                }
                Ok(())
            }
            Err(e) => {
                error!(target: "security_verify", trace_id = trace_id, "Signature verification FAILED for {:?}: {}", mod_path, e);
                Err(EngineError::SignatureVerificationFailed(
                    format!("Cryptographic signature verification failed for mod: {:?}. Error: {}", mod_path, e)
                ))
            }
        }
    }
}

// ============================================================================
// 4. 资源配额管理器 (增强版)
// ============================================================================

#[derive(Clone, Debug)]
pub struct ResourceLimits {
    pub max_instruction_count: u64,
    pub max_execution_time_ms: u64,
    pub memory_limit_mb: usize, // 新增：内存限制
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_instruction_count: 10000,
            max_execution_time_ms: 50,
            memory_limit_mb: 64, // 默认 64MB
        }
    }
}

pub struct QuotaEnforcer {
    limits: ResourceLimits,
    start_time: Instant,
    instruction_counter: u64,
}

impl QuotaEnforcer {
    pub fn new(limits: ResourceLimits) -> Self {
        Self {
            limits,
            start_time: Instant::now(),
            instruction_counter: 0,
        }
    }

    pub fn reset(&mut self) {
        self.start_time = Instant::now();
        self.instruction_counter = 0;
    }

    pub fn check_quota(&mut self) -> Result<(), mlua::Error> {
        self.instruction_counter += 1;
        // 每 N 条指令检查一次时间，减少开销
        if self.instruction_counter % self.limits.max_instruction_count == 0 {
            let elapsed = self.start_time.elapsed();
            if elapsed.as_millis() > self.limits.max_execution_time_ms as u128 {
                return Err(mlua::Error::runtime(format!(
                    "CPU Time Limit Exceeded: {}ms > {}ms",
                    elapsed.as_millis(),
                    self.limits.max_execution_time_ms
                )));
            }
        }
        Ok(())
    }
}

// ============================================================================
// 5. 依赖管理与元数据 (Phase 3: SemVer & Namespace)
// ============================================================================

#[derive(Clone, Debug)]
pub struct ModMetadata {
    pub name: String,
    pub version: Version, // 改为 semver::Version
    pub namespace: String, // 新增：命名空间隔离
    pub dependencies: Vec<(String, VersionReq)>, // 改为 (Name, VersionReq)
}

impl UserData for ModMetadata {}

pub struct DependencyResolver {
    // Key: Namespace::Name
    loaded_mods: HashMap<String, ModMetadata>,
}

impl DependencyResolver {
    pub fn new() -> Self {
        Self { loaded_mods: HashMap::new() }
    }

    pub fn resolve_and_validate(&mut self, new_mod: &ModMetadata, loading_stack: &HashSet<String>) -> Result<(), EngineError> {
        let mod_key = format!("{}::{}", new_mod.namespace, new_mod.name);
        
        if loading_stack.contains(&mod_key) {
            return Err(EngineError::DependencyError(format!("Circular dependency detected: {}", mod_key)));
        }

        // 检查是否已加载相同版本，避免重复加载
        if let Some(existing) = self.loaded_mods.get(&mod_key) {
            if existing.version == new_mod.version {
                info!("Mod {} already loaded with same version, skipping.", mod_key);
                println!("Mod {} already loaded with same version, skipping.", mod_key);
                return Ok(());
            } else {
                // 简单策略：如果版本不同，报错或覆盖？这里选择报错以保持稳定
                return Err(EngineError::DependencyError(format!(
                    "Version conflict for {}: loaded {}, trying to load {}", 
                    mod_key, existing.version, new_mod.version
                )));
            }
        }

        self.loaded_mods.insert(mod_key.clone(), new_mod.clone());
        
        let mut next_stack = loading_stack.clone();
        next_stack.insert(mod_key.clone());

        for (dep_name, dep_ver_req) in &new_mod.dependencies {
            // 在同一个命名空间下查找依赖，或者全局查找？这里假设依赖也在同一命名空间或全局
            // 简化：假设依赖名是唯一的，或者需要指定 namespace::name
            let dep_key = if dep_name.contains("::") {
                dep_name.clone()
            } else {
                format!("{}::{}", new_mod.namespace, dep_name)
            };

            if let Some(dep_mod) = self.loaded_mods.get(&dep_key) {
                if !dep_ver_req.matches(&dep_mod.version) {
                    return Err(EngineError::DependencyError(format!(
                        "Dependency version mismatch: {} requires {}, but found {}",
                        dep_name, dep_ver_req, dep_mod.version
                    )));
                }
            } else {
                return Err(EngineError::DependencyError(format!("Missing dependency: {}", dep_name)));
            }
        }
        Ok(())
    }
}

// ============================================================================
// 6. 预处理器
// ============================================================================

pub struct QPreprocessor {
    base_path: PathBuf,
    cache_path: PathBuf,
}

impl QPreprocessor {
    pub fn new(base_path: PathBuf, cache_path: PathBuf) -> Self {
        if !cache_path.exists() {
            fs::create_dir_all(&cache_path).ok();
        }
        Self { base_path, cache_path }
    }

    pub fn process(&self, file_path: &Path) -> Result<(String, Vec<PathBuf>), EngineError> {
        let content = fs::read_to_string(file_path)?;
        let mut imports = Vec::new();
        let mut processed_lines = Vec::new();
        
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("Import") {
                if let Some(path_str) = self.extract_quoted_string(trimmed, "Import") {
                    imports.push(self.base_path.join(&path_str));
                }
            } else if trimmed.starts_with("Include") {
                 if let Some(path_str) = self.extract_quoted_string(trimmed, "Include") {
                    let include_path = self.base_path.join(&path_str);
                    let included_content = fs::read_to_string(&include_path)
                        .map_err(|e| EngineError::PreprocessorError(format!("Include failed: {}", e)))?;
                    processed_lines.push(included_content);
                }
            } else {
                processed_lines.push(line.to_string());
            }
        }
        Ok((processed_lines.join("\n"), imports))
    }

    fn extract_quoted_string(&self, line: &str, keyword: &str) -> Option<String> {
        let prefix = format!("{} ", keyword);
        if line.starts_with(&prefix) {
            let rest = &line[prefix.len()..];
            if rest.starts_with('"') && rest.ends_with('"') && rest.len() > 1 {
                return Some(rest[1..rest.len()-1].to_string());
            }
        }
        None
    }
}

// ============================================================================
// 7. 核心引擎 (Phase 3: Hot Reload & Profiling)
// ============================================================================

// 用于热重载的 Mod 句柄
struct ModHandle {
    metadata: ModMetadata,
    // 存储该 Mod 注册的全局表或函数，以便卸载时清理
    globals_snapshot: Vec<String>, 
    load_time: Instant,
}

pub struct ScriptEngine {
    lua: Lua,
    base_path: PathBuf,
    preprocessor: QPreprocessor,
    verifier: SignatureVerifier,
    quota_enforcer: Arc<Mutex<QuotaEnforcer>>,
    audit_logger: AuditLogger,
    dep_resolver: DependencyResolver,
    is_temp_mode: bool,
    
    // Phase 3: Profiling & Hot Reload State
    profiles: Arc<RwLock<HashMap<String, ModProfile>>>,
    active_mods: HashMap<String, ModHandle>, // Key: Namespace::Name
}

impl ScriptEngine {
    pub fn new(base_path: PathBuf, is_temp_mode: bool) -> Result<Self, EngineError> {
        let cache_path = base_path.join("cache");
        let audit_logger = AuditLogger::new(&base_path);
        let verifier = SignatureVerifier::new()?;
        let limits = ResourceLimits::default();
        let enforcer = QuotaEnforcer::new(limits.clone());
        let quota_enforcer = Arc::new(Mutex::new(enforcer));

        let lua = Lua::new();
        // 设置内存限制 (Phase 2/3)
        lua.set_memory_limit(limits.memory_limit_mb * 1024 * 1024)?;

        let preprocessor = QPreprocessor::new(base_path.clone(), cache_path);
        let dep_resolver = DependencyResolver::new();

        let mut engine = Self {
            lua,
            base_path: base_path.clone(),
            preprocessor,
            verifier,
            quota_enforcer,
            audit_logger,
            dep_resolver,
            is_temp_mode,
            profiles: Arc::new(RwLock::new(HashMap::new())),
            active_mods: HashMap::new(),
        };

        engine.setup_secure_sandbox()?;
        engine.register_advanced_hooks()?;

        Ok(engine)
    }

    fn setup_secure_sandbox(&mut self) -> Result<(), EngineError> {
        let globals = self.lua.globals();
        let empty_table = self.lua.create_table()?;

        // 屏蔽危险库
        globals.set("io", empty_table.clone())?;
        globals.set("os", empty_table.clone())?;
        globals.set("package", empty_table.clone())?;
        globals.set("debug", empty_table)?;

        // 受控文件系统
        let fs_api = self.lua.create_table()?;
        let ctx_ref = self.base_path.clone();
        
        let read_file = self.lua.create_function(move |_lua, path: String| {
            let full_path = ctx_ref.join(&path);
            if !full_path.starts_with(&ctx_ref) {
                return Err(mlua::Error::runtime("Access denied: Path traversal"));
            }
            fs::read_to_string(&full_path).map_err(|e| mlua::Error::runtime(e.to_string()))
        })?;
        fs_api.set("read", read_file)?;
        globals.set("ModFS", fs_api)?;

        // 受控打印
        let print_func = self.lua.create_function(|_, args: String| {
            info!(target: "script_output", "{}", args);
            println!("script_output: {}", args);
            Ok(())
        })?;
        globals.set("Print", print_func)?;

        Ok(())
    }

    fn register_advanced_hooks(&mut self) -> Result<(), EngineError> {
        let globals = self.lua.globals();
        let hook_registry = self.lua.create_table()?;

        // 增强版 Hook 注册，携带版本和 Trace ID
        let profiles_ref = self.profiles.clone();
        let register_hook = self.lua.create_function(move |_lua, args: (String, String, Function)| {
            let (name, version_str, _func) = args;
            
            // 记录 Hook 注册事件
            info!(target: "hook_system", name = %name, version = %version_str, "Registered hook");
            println!("hook_system: Registered hook (name = {}, version = {})", name, version_str);
            
            // 可以在这里初始化该 Hook 的性能计数器
            let mut profiles = profiles_ref.write().unwrap();
            let key = format!("hook::{}", name);
            profiles.entry(key).or_insert(ModProfile {
                load_time_ms: 0,
                execution_count: 0,
                total_cpu_time_ms: 0,
                last_error: None,
            });

            Ok(())
        })?;
        
        hook_registry.set("register", register_hook)?;
        globals.set("HookSystem", hook_registry)?;

        let version_info = self.lua.create_table()?;
        version_info.set("engine_version", "1.0.0-Enterprise-Secure")?;
        globals.set("EngineInfo", version_info)?;

        Ok(())
    }

    fn setup_instruction_hook(&self) -> Result<(), EngineError> {
        let enforcer_clone = self.quota_enforcer.clone();
        self.lua.set_hook(HookTriggers::new().every_nth_instruction(10000), move |_lua, _debug| {
            let mut enforcer = enforcer_clone.lock().unwrap();
            enforcer.check_quota().map_err(|e| mlua::Error::external(e.to_string()))?;
            Ok(())
        });
        Ok(())
    }

    // Phase 3: Unload Mod safely
    fn unload_mod_internal(&mut self, mod_key: &str) -> Result<(), EngineError> {
        if let Some(handle) = self.active_mods.remove(mod_key) {
            info!("Unloading mod: {}", mod_key);
            println!("Unloading mod: {}", mod_key);
            
            // 1. 清理 Lua 全局环境中的相关引用
            // 注意：mlua 没有直接的 "delete global" API，通常通过将值设为 nil 来实现
            let globals = self.lua.globals();
            for key in &handle.globals_snapshot {
                globals.set(key.as_str(), mlua::Nil)?;
            }

            // 2. 强制 GC
            self.lua.gc_collect();
            
            // 3. 更新依赖解析器状态
            self.dep_resolver.loaded_mods.remove(mod_key);
            
            info!("Mod {} unloaded successfully.", mod_key);
            println!("Mod {} unloaded successfully.", mod_key);
        }
        Ok(())
    }

    pub fn load_mod(&mut self, mod_path: &Path, trace_id: &str) -> Result<ModMetadata, EngineError> {
        let start_time = Instant::now();
        let _span = span!(Level::INFO, "mod_loading", trace_id = %trace_id, path = ?mod_path).entered();

        if !mod_path.exists() {
            return Err(EngineError::PreprocessorError(format!("Mod not found: {:?}", mod_path)));
        }

        // 1. 签名验证
        let sig_path = mod_path.with_extension("sig");
        self.verifier.verify_mod(mod_path, &sig_path, self.is_temp_mode, trace_id)?;

        if self.is_temp_mode {
            self.audit_logger.log_warning(trace_id, &format!("Loading UNVERIFIED/TEMP Mod: {:?}", mod_path));
        }

        // 2. 元数据解析 (简化：从文件名或注释中提取，这里假设硬编码或简单解析)
        // 实际生产中应解析文件头部的 JSON/YAML 元数据
        let mod_name = mod_path.file_stem().unwrap().to_string_lossy().to_string();
        let namespace = "default".to_string(); // 简化
        
        // 模拟解析版本
        let version = Version::parse("1.0.0").map_err(|e| EngineError::DependencyError(e.to_string()))?;

        let metadata = ModMetadata {
            name: mod_name.clone(),
            version: version.clone(),
            namespace: namespace.clone(),
            dependencies: vec![],
        };
        
        let mod_key = format!("{}::{}", namespace, mod_name);

        // 3. 依赖解析
        let loading_stack = HashSet::new();
        self.dep_resolver.resolve_and_validate(&metadata, &loading_stack)?;

        // 4. 如果已加载，先卸载 (Hot Reload 支持)
        if self.active_mods.contains_key(&mod_key) {
            warn!("Mod {} is already loaded. Performing hot reload...", mod_key);
            self.unload_mod_internal(&mod_key)?;
        }

        // 5. 执行代码
        let code = fs::read_to_string(mod_path)?;
        
        // 记录加载前的全局键，以便后续清理
        let globals_before: Vec<String> = self.lua.globals()
            .pairs::<String, mlua::Value>()
            .filter_map(|res| res.ok().map(|(k, _v)| k))
            .collect();

        let chunk = self.lua.load(&code);
        chunk.exec()?;

        // 记录加载后的新全局键
        let globals_after: Vec<String> = self.lua.globals()
            .pairs::<String, mlua::Value>()
            .filter_map(|res| res.ok().map(|(k, _v)| k))
            .collect();
        
        let new_globals: Vec<String> = globals_after.into_iter()
            .filter(|k| !globals_before.contains(k))
            .collect();

        let load_duration = start_time.elapsed();
        
        // 注册 Handle
        self.active_mods.insert(mod_key.clone(), ModHandle {
            metadata: metadata.clone(),
            globals_snapshot: new_globals,
            load_time: start_time,
        });

        // 更新 Profile
        {
            let mut profiles = self.profiles.write().unwrap();
            profiles.entry(mod_key).or_insert(ModProfile {
                load_time_ms: 0,
                execution_count: 0,
                total_cpu_time_ms: 0,
                last_error: None,
            }).load_time_ms = load_duration.as_millis();
        }

        info!(target: "mod_loader", trace_id = trace_id, "Loaded mod: {} ({}ms)", mod_name, load_duration.as_millis());
        println!("mod_loader: Loaded mod: {} ({}ms), trace_id = {}", mod_name, load_duration.as_millis(), trace_id);
        Ok(metadata)
    }

    pub fn run_script(&mut self, script_path: &Path) -> Result<(), EngineError> {
        let trace_id = generate_trace_id();
        let _span = span!(Level::INFO, "script_execution", trace_id = %trace_id, script = ?script_path).entered();
        
        info!("Starting script execution (Temp Mode: {})", self.is_temp_mode);
        println!("Starting script execution (Temp Mode: {})", self.is_temp_mode);

        // 1. 预处理
        let (processed_code, import_paths) = self.preprocessor.process(script_path)?;

        // 2. 加载 Imports
        for mod_path in import_paths {
            self.load_mod(&mod_path, &trace_id)?;
        }

        // 3. 重置配额
        {
            let mut enforcer = self.quota_enforcer.lock().unwrap();
            enforcer.reset();
        }

        // 4. 激活 CPU 熔断
        self.setup_instruction_hook()?;

        // 5. 执行
        let chunk = self.lua.load(&processed_code);
        let chunk = chunk.set_name(script_path.to_string_lossy().as_ref());
        
        let exec_start = Instant::now();
        match chunk.exec() {
            Ok(_) => {
                let duration = exec_start.elapsed();
                info!("Script executed successfully in {}ms", duration.as_millis());
                println!("Script executed successfully in {}ms", duration.as_millis());
                
                // 更新主脚本的 Profile (如果视为一个临时 Mod)
                // 此处省略具体 Profile 更新逻辑，原理同上
                Ok(())
            }
            Err(e) => {
                let err_msg = e.to_string();
                let duration = exec_start.elapsed();
                
                if err_msg.contains("exceeded time limit") || err_msg.contains("CPU Time Limit") {
                    self.audit_logger.log_violation(&trace_id, &format!("Resource limit exceeded: {}", err_msg));
                    Err(EngineError::ResourceLimitExceeded(err_msg))
                } else {
                    // 精确错误溯源：记录错误到 Profile
                    if let Some(mod_key) = self.get_current_executing_mod() {
                         if let Ok(mut profiles) = self.profiles.write() {
                             if let Some(profile) = profiles.get_mut(&mod_key) {
                                 profile.last_error = Some(err_msg.clone());
                             }
                         }
                    }

                    error!(target: "runtime_error", trace_id = %trace_id, "Execution failed after {}ms: {}", duration.as_millis(), e);
                    Err(EngineError::from(e))
                }
            }
        }
    }

    // 辅助：尝试获取当前正在执行的 Mod 名称 (简化实现)
    fn get_current_executing_mod(&self) -> Option<String> {
        // 在实际复杂场景中，可能需要维护一个执行栈
        // 这里简单返回最后一个加载的 Mod，或者通过 Lua debug info 获取
        None 
    }
    
    // Phase 3: 获取性能报告
    pub fn get_performance_report(&self) -> Result<String, EngineError> {
        let profiles = self.profiles.read().unwrap();
        let mut report = String::from("=== Performance Report ===\n");
        for (key, profile) in profiles.iter() {
            report.push_str(&format!(
                "Mod: {}\n  Load Time: {}ms\n  Exec Count: {}\n  Total CPU: {}ms\n  Last Error: {:?}\n",
                key, profile.load_time_ms, profile.execution_count, profile.total_cpu_time_ms, profile.last_error
            ));
        }
        Ok(report)
    }
}

// ============================================================================
// 8. 主程序入口 (Main)
// ============================================================================

fn main() {

    let args: Vec<String> = std::env::args().collect();
    
    if args.len() < 3 {
        println!("Usage:");
        println!("  engine --Run <script.q>       (Standard Mode, Official Key)");
        println!("  engine --Temp <script.q>      (Temp Mode, Warning Key)");
        println!("  engine --Report               (Show Performance Report)");
        return;
    }

    let mode = &args[1];
    
    if mode == "--Report" {
        // 演示：如果是报告模式，这里通常需要持久化存储，简化起见仅提示
        println!("Performance reporting requires a running instance or persistent storage.");
        return;
    }

    let script_file = &args[2];
    let base_path = std::env::current_dir().expect("Failed to get current directory");
    let script_path = base_path.join(script_file);

    if !script_path.exists() {
        error!("Script file not found: {:?}", script_path);
        return;
    }

    let is_temp = mode == "--Temp";
    
    if is_temp {
        warn!("!!! RUNNING IN TEMPORARY MODE !!!");
        warn!("Using Temp Key: {}", TEMP_PUBLIC_KEY_HEX);
        warn!("Security restrictions may be relaxed for debugging.");
    }

    match ScriptEngine::new(base_path, is_temp) {
        Ok(mut engine) => {
            match engine.run_script(&script_path) {
                Ok(_) => {
                    info!("Process finished.");
                    println!("Process finished.");
                    // 打印性能报告
                    if let Ok(report) = engine.get_performance_report() {
                        info!("\n{}", report);
                    }
                },
                Err(e) => error!("Fatal Error: {}", e),
            }
        }
        Err(e) => {
            error!("Failed to initialize engine: {}", e);
        }
    }
}
