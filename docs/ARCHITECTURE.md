# Quern 工具链架构文档

> 本文件描述 Quern 语言工具链的整体架构、各组件职责、数据流、字节码规范与文件组织。

---

## 1. 总览

Quern 工具链是一个**多语言协作**的编译/运行时系统，包含三大执行路径：

| 路径 | 说明 | 执行引擎 |
|------|------|----------|
| **VM 解释执行** | `.q` → 字节码 `.qb` → 逐条指令解释 | Qvm（Rust） |
| **AOT 本地编译** | `.qb` → 生成 C 代码 → clang 编译 → 原生可执行 | QuernBuild（C++） |
| **启动器统一调度** | 封装上述两条路径 + 模块管理 | QuerncLauncher（Rust） |

三种产物均以 Quernc（Go）产出的 `.qb` 字节码为中间表示，保证语义一致。

---

## 2. 系统架构图

```
                       ┌────────────────────────────────┐
                       │      Quern 源文件 (.q)          │
                       └───────────────┬────────────────┘
                                       │
                                       ▼
                       ┌────────────────────────────────┐
                       │   Quernc  (Go 翻译器)           │
                       │  ┌──────────┐  ┌────────────┐   │
                       │  │ Parser   │→ │ AST        │   │
                       │  └──────────┘  └─────┬──────┘   │
                       │                      ▼          │
                       │  ┌──────────┐  ┌────────────┐   │
                       │  │ ModLoader│→ │ Translator │   │
                       │  │ (goja JS)│  │ + DCE +Cache│  │
                       │  └──────────┘  └─────┬──────┘   │
                       └──────────────────────┼──────────┘
                                              │
                                              ▼
                                  ┌────────────────────┐
                                  │  Bytecode (.qb)    │
                                  │  crt / psh / out   │
                                  │  jmp / cal / fnc   │
                                  └──────────┬─────────┘
                                             │
              ┌──────────────────────────────┴───────────────────────────┐
              ▼                                                          ▼
   ┌────────────────────┐                                     ┌────────────────────┐
   │  Qvm (Rust)        │                                     │  QuernBuild (C++)  │
   │  ┌──────────────┐  │                                     │  ┌──────────────┐  │
   │  │ Parser       │  │                                     │  │ Parser       │  │
   │  │ VM.execute   │  │                                     │  │ CodeGenerator│  │
   │  │ stacks:      │  │                                     │  │ → C code     │  │
   │  │ HashMap      │  │                                     │  └──────┬───────┘  │
   │  └──────────────┘  │                                     │         ▼          │
   └─────────┬──────────┘                                     │  ┌──────────────┐  │
             │                                                │  │ clang -O?    │  │
             ▼                                                │  └──────┬───────┘  │
        解释执行输出                                           └─────────┼──────────┘
                                                                    ▼
                                                            原生可执行文件

         ┌───────────────────────────────────────────────────────────┐
         │              QuerncLauncher (Rust) 统一入口               │
         │  --run / --vm-verbose / --vm-check / --qvm-run            │
         │  --aot-build / --module-* / --web-* / --local-list        │
         │  按模式调用:  Quernc / Qvm / QuernBuild / Qlm              │
         └───────────────────────────────────────────────────────────┘

         ┌───────────────────────────────────────────────────────────┐
         │              Qlm (C++) 模块/项目管理器                    │
         │  --Install / --Delete / --Disable / --Enable               │
         │  --WebList / --ModsList / --WebSearch / --NewProject       │
         │  --InstallPackage / --ProjectRun                           │
         └───────────────────────────────────────────────────────────┘

         ┌───────────────────────────────────────────────────────────┐
         │              Updater (C++) 自动更新器                     │
         │  --update  →  拉取 GitCode 最新 tag 并解压覆盖              │
         └───────────────────────────────────────────────────────────┘
```

---

## 3. 组件详解

### 3.1 Quernc — Go 翻译器

**源码文件**：`ast.go`、`parser.go`、`dce.go`、`cache.go`、`modloader.go`、`translator_core.go`、`main.go`

**职责**：将 Quern 源文件 `.q` 翻译为字节码 `.qb`。

**处理流水线**：

```
源文件
  │
  ▼  stripQuernComments()          ← 剥离 // ... // 块注释
  ▼  Parser.Parse()                ← 逐行 AST 构建
  ▼  DeadCodeEliminator.Analyze()  ← 可达性分析
  ▼  checkForUnusedItems()         ← 警告/严格模式
  ▼  SplitProgramIntoUnits()       ← 按 Function/Class/Entrust 切分
  ▼  CacheManager                  ← MD5 哈希比对
  ▼  Translator.Translate()        ← 逐节点生成 VM 指令
  ▼
.qb 文件（cache/bytecode/xxx.qb）
```

**关键模块**：

| 模块 | 职责 |
|------|------|
| **Parser** | 逐行解析顶层语句（`Function`/`Class`/`Entrust`/`Import`）与块内语句（`Data.Var`、`Console.Info`、`If`/`Loop`/`DataStruct.*`、`#VMCODE` 逃逸块） |
| **DeadCodeEliminator** | 从 Main 函数 + Entrust 出发，标记 `usedFunctions/usedClasses/usedVars/usedLists/usedDicts`；`Filter()` 生成精简 AST，警告未使用项 |
| **CacheManager** | 单元级（Unit = 一个 Function/Class/Entrust）MD5 缓存；只有变更单元才触发翻译，无变更时直接复用 `.qb` |
| **ModLoader** | 嵌入 `goja` JS 引擎；加载 `mods/*.js`，通过 `Quern.Reg(keyword, handler)` 注册自定义语法处理器；`Include` 指令递归合并 JS 文件并防越权 |
| **Translator** | 核心翻译器：常量折叠（`ConstCalc`）、循环展开（`ExpandLoop`）、别名替换、列表/字典状态机、If/Else/Loop 控制流生成、`#VMCODE` 原文透传 |
| **Translator.evaluateExpression** | 借 goja 运行时安全求值数学表达式（`1+2*3`） |

**Quern 源语言特性**：
- `Data.Var [Private] [Int|String|Bool|Num] Name = Value`
- `Console.Info("...")`
- `DataStruct.List/Dict` + `ListAdd/Delete/Edit/Find.*` + `DictAdd/Delete/Edit/Find.*`
- `Loop(n) { }`、`If (a = b) [Loop(n)] { } Else [If ...] [Loop(n)] { }`（支持 `and`/`or`/`not`）
- `Entrust (cond) { }` — 顶层守护块，每次循环检查
- `#VMCODE _BEGIN ... #VMCODE _END` — 原样注入字节码逃逸块
- `// ... //` 块注释（任意位置）

---

### 3.2 Qvm — Rust 虚拟机

**源码文件**：`main.rs`、`instructions.rs`、`parser.rs`、`vm.rs`、`errors.rs`

**职责**：直接解释执行 `.qb` 字节码。

**数据结构**：

```rust
pub struct VM {
    stacks: HashMap<String, Vec<String>>,   // 每个栈名 → 字符串栈
}

pub enum Instruction {
    CreateStack(String),
    Push(String, (String, bool)),            // (stack, (value, is_var))
    Pop(String),
    Out((String, bool)),                     // (name, is_var)
    PrintNewLine,
    DeleteStack(String),
    CallFunction(String),
    Arithmetic { stack, operator, value },
    ConditionalJump { left_stack, right_stack, op, target_func },
}
```

**执行语义**：
- `Push` 的 `is_var=true` 时，从源栈取栈顶值再压入目标栈（栈传值）
- `Out` 的 `is_var=true` 时，取变量栈的栈顶作为**栈名**再解引用输出（间接输出）
- `Out` 的字面模式支持 `@var@` 字符串插值
- `ConditionalJump` 命中目标函数后**立即 return**（不继续当前 body），实现"跳转即返回"
- `Arithmetic` 使用 f64 解析 + 整数格式化输出

**命令行**：
```
Qvm --Run <file.qb> [--Check] [--Verbose]
```

---

### 3.3 QuernBuild — C++ AOT 编译器

**源码文件**：`main.cpp`、`instruction.cpp`、`parser.cpp`、`errors.cpp`、`ganerator.cpp`、`run_complier.cpp`

**职责**：将 `.qb` 字节码翻译成 C 代码，再调用 `clang` 编译为原生可执行文件。

**处理流水线**：

```
.qb 文件
  ▼
Parser.parse()           ← 与 Qvm 同构的字节码解析
  ▼  Program { instructions, functions }
  ▼
CodeGenerator.generate()
  ├─ needs_stack_support()         ← 无栈指令则不注入运行时
  ├─ collect_stack_names()         ← 登记所有出现过的栈名
  ├─ 条件注入 runtime code        ← get_runtime_code() 返回 Stack/var_set/compare_values 等
  ├─ 生成全局 Stack 变量
  ├─ 函数前置声明 + 定义
  ├─ 入口 main() 内联全部顶层指令
  └─ 清理阶段（free_stack） + 调用 qb_exit()
  ▼
<file>.q.c
  ▼
build_clang_cmd()               ← 拼装 clang 命令（-O2/-Os/-Oz/-Ofast/-O0 + 警告选项）
  ▼
run_command()  →  <file>.exe（Linux/macOS 为 <file>）
```

**生成的 C 运行时（`get_runtime_code`）**：

| 函数 | 用途 |
|------|------|
| `init_stack` / `push_stack` / `pop_stack` / `free_stack` / `print_top` | `Stack` 结构体基本操作 |
| `register_stack` / `find_stack` / `get_top` / `get_top_by_name` | 256 槽栈名注册表（支持 `@var@` 间接寻址） |
| `out_variable` / `out_varval` | 变量输出（值即栈名 → 解引用） |
| `var_get` / `var_set` | 解耦的"变量数值寄存器"（供 `psh <stack> + N` 累加） |
| `psh_arith` | `psh <name> <op> "<operand>"` 算术：base=寄存器值 → 写回寄存器 |
| `compare_values` / `compare_stacks` | 数值优先（`strtod` 双端成功则按数值比较），回退字符串字典序 |

**C 代码生成策略**：
- `sanitize_c_id`：把栈名转为合法 C 标识符（`100` → `_100`，非字母数字 → `_`）
- `qb_func_name`：用户函数统一加 `qb_` 前缀，避免与 C 库冲突
- `exit` 函数自动追加 `return 0;`（避免用 `exit(0)` 跳过栈清理）
- 字面字符串统一走 `process_string` / `escape_c_string` 双重转义
- 无栈操作时**不注入**运行时（`needs_stack_support` 短路），减小产物体积

**命令行**：
```
QuernBuild [--ClangOSize|--ClangOSizeBest|--ClangODebug|--ClangOFAST|--NotO]
           [--CVerbose] [--ForceWarn] [--NoWarn] [--VMVerbose]
           <input_file.qb>
```

**工具链自愈**：`check_clang()` 找不到 clang 时，`download_toolchain()` 按平台自动安装（Windows PowerShell / macOS xcode-select+brew / Linux apt/dnf/pacman）。

---

### 3.4 QuerncLauncher — Rust 统一入口

**源码文件**：`main.rs`、`args.rs`、`arg_logics.rs`、`traceing.rs`

**职责**：暴露一个统一 CLI，按参数把工作分派到 Quernc / Qvm / QuernBuild / Qlm.exe。

**Args → 执行路径映射**：

| 参数 | 执行链 |
|------|--------|
| `--run <script>` | `Quernc --run <script>` |
| `--vm-verbose <script>` | `Quernc --run` + `Qvm --run --verbose` |
| `--vm-check <script>` | `Quernc --run` + `Qvm --run --check` |
| `--qvm-run <qb>` | 仅 `Qvm --run <qb>` |
| `--aot-build <script>` | `Quernc --Run <script>` → `QuernBuild [opt flags] cache/bytecode/<stem>.qb` |
| `--module-install/delete/enable/disable <name>` | `Qlm.exe --Install/--Delete/...` |
| `--web-list` / `--local-list` | `Qlm.exe --WebList` / `--ModsList` |
| `--web-search <kw>` | `Qlm.exe --WebSearch <kw>` |
| `--module-install-all` | `Qlm.exe --InstallPackage` |
| `--logo` | 打印 logo 并退出 |
| 无 flag + 位置参数 | 默认按 `--run` 处理 |

**AOT 优化等级映射**：

| Rust Args | QuernBuild flag |
|-----------|-----------------|
| `--not-o` | `--NotO` |
| `--ofast` | `--ClangOFAST` |
| `--osize` | `--ClangOSize`（默认） |
| `--osize-bast` | `--ClangOSizeBest` |

**Trace 系统（`traceing.rs`）**：
- 每进程生成 UUID 作为 `trace_id`
- 双 layer：控制台层 + 文件层（`./Logs/<YYYY>/<MM>/<DD>/<HHMMSS>_<mode>_<trace_id>_log.log`）
- `--logs` 启用文件层，`--no-print-trace` 抑制控制台 trace 输出

---

### 3.5 Qlm — Quern Library Manager (C++)

**源码文件**：`main.cpp`、`git.cpp`、`module_manager.cpp`、`project_manager.cpp`、`utills.cpp`

**职责**：JS 模块的云端检索 / 安装 / 启停，以及 Quern 项目管理。

**模块目录约定**：
```
./mods/          已启用模块（*.js）
./disabled/      已停用模块
```

**命令一览**：

| 命令 | 行为 |
|------|------|
| `--Install <name.js>` | `git clone` → 定位文件（大小写不敏感）→ 复制到 `mods/` |
| `--Delete <name.js>` | 从 `mods/` 移除 |
| `--Disable <name.js>` | `mods/` → `disabled/` |
| `--Enable <name.js>` | `disabled/` → `mods/` |
| `--WebList` | 浅克隆远端仓库到临时目录，列出所有 `.js` |
| `--WebSearch <kw>` | 同上 + 文件名过滤 |
| `--ModsList` / `--ModsSearch <kw>` | 本地列表/搜索 |
| `--NewProject <name>,<ver>` | 创建 `<name>/`，含 `src/main.q`、`<name>.toml` 模板 |
| `--InstallPackage` | 解析当前目录 `.toml` 的 `need = {...}` 块，按 `ver`/`sha256` 安装 |
| `--ProjectRun` | 检查依赖 → 缺失则安装 → 调 `Quernc.exe --Run src/main.q` |

**依赖版本语法（TOML `need` 块）**：
```toml
need = {
  ModuleName.js ver = 2023/10/01,12/00/00 sha256 = abc123...
}
```
- `ver`：转换为 `git checkout "@{YYYY-MM-DD HH:MM:SS}"` 回退到指定时间点
- `sha256`：安装后由 `computeSHA256()`（`utills.cpp` 内置轻量实现）校验，不匹配则**拒绝安装**
- 行尾逗号 / `#` 行内注释自动跳过

**Git 依赖自愈**（`git.cpp`）：
- `isCommandAvailable("git")` 检查
- Windows 下尝试追加常见安装路径到 `PATH`（Program Files / AppData）
- 仍失败则交互式提示，Windows 打开下载页，Linux 走 `apt install git`

---

### 3.6 Updater — 自动更新器 (C++)

**源码文件**：`main.cpp`、`updater.cpp`、`windows.cpp`、`linux.cpp`

**职责**：从 GitCode 拉取 Quern 最新 tag 并解压覆盖当前目录。

- Windows：构造 PowerShell 脚本（取 tag → 下载 zip → `Expand-Archive` → 进入 `Quern/`），Base64+UTF-16 编码后通过 `powershell -EncodedCommand` 执行（规避转义问题）
- Linux：`git ls-remote --tags` → 取最大 tag → `wget` 下载 → `unzip -o` → 进入 `Quern/`

---

## 4. 字节码规范（.qb）

### 4.1 指令集

| 指令 | 参数 | 语义 |
|------|------|------|
| `crt <stack>` | 栈名 | 创建/获取栈 |
| `psh <stack> <value>` | 目标栈、值（字面或 `@var@`） | 压栈；`@var@` 表示取变量当前值 |
| `psh <stack> <op> "<operand>"` | `op ∈ {+ - * /}` | 算术累加：寄存器/栈顶 OP operand，结果写回 |
| `pop <stack>` | 栈名 | 弹出栈顶 |
| `out <expr>` | 栈名 / 字符串 / `@var@` | 输出；栈名→打印栈顶；`@var@`→间接解引用 |
| `otn` | — | 输出换行 |
| `del <stack>` | 栈名 | 删除栈 |
| `cal <func>` | 函数名 | 调用函数（可递归） |
| `jmp <l> <r> <op> cal <target>` | `op ∈ {= != < > =< >=}` | `compare(l, r, op)` 为真时调用 target 并**立即 return** |
| `fnc <name> { ... }` | 函数名 | 函数定义 |
| `# ...` | — | 注释 / 诊断输出 |

### 4.2 变量约定

- **`@name@` 语法**：在 `psh` / `out` / `jmp` 的参数中表示"变量 `name` 的当前值"
- 变量的物理载体仍是栈，但 Qvm/QuernBuild 在 `var_get`/`var_set` 层提供与栈解耦的数值寄存器，保证 `psh <name> + N` 的累加跨多次 push 仍成立

### 4.3 比较语义

数值优先：**两端都**能被 `strtod` 完整解析成数值时走数值比较（`=` 用 1e-9 容差）；否则走 `strcmp` 字典序。两端存在空值时返回 `false`。

---

## 5. 组件间数据流

### 5.1 解释执行路径

```
script.q ──Quernc──▶ cache/bytecode/script.qb ──Qvm──▶ stdout
                          │
                          └─ 单元级 MD5 缓存，改动单元才重译
```

### 5.2 AOT 编译路径

```
script.q ──Quernc──▶ cache/bytecode/script.qb
                          │
                          ▼
                    QuernBuild
                          │
                          ├─ 解析 .qb
                          ├─ 条件注入 C 运行时
                          ├─ 生成 script.q.c
                          └─ clang -O2 -o script.exe
                          ▼
                    script.exe（原生可执行）
```

### 5.3 模块加载路径

```
mods/*.js ──Quernc(ModLoader/goja)──▶ handlers{keyword: fn}
                                          │
                                          ▼
                 Parser 遇到未知关键字 → CustomNode
                                          │
                                          ▼
                Translator 查 handler 表 → 调 JS 函数 → 逐行 emit 指令
```

### 5.4 Launcher 调度

```
QuerncLauncher
  ├─ mode=Run           → Quernc
  ├─ mode=VMVerbose     → Quernc → Qvm --verbose
  ├─ mode=VMCheck       → Quernc → Qvm --check
  ├─ mode=QvmRun        → Qvm
  ├─ mode=AOTBuild      → Quernc → QuernBuild
  └─ mode=Module*/Web*  → Qlm.exe
```

---

## 6. 文件组织

```
quern-toolchain/
├── Quernc/                  # Go 翻译器
│   ├── main.go
│   ├── ast.go
│   ├── parser.go
│   ├── dce.go
│   ├── cache.go
│   ├── modloader.go
│   └── translator_core.go
│
├── Qvm/                     # Rust 虚拟机
│   ├── main.rs
│   ├── instructions.rs
│   ├── parser.rs
│   ├── vm.rs
│   └── errors.rs
│
├── QuernBuild/              # C++ AOT 编译器
│   ├── main.cpp
│   ├── instruction.cpp
│   ├── parser.cpp
│   ├── errors.cpp
│   ├── ganerator.cpp
│   └── run_complier.cpp
│
├── QuerncLauncher/          # Rust 启动器
│   ├── main.rs
│   ├── args.rs
│   ├── arg_logics.rs
│   └── traceing.rs
│
├── Qlm/                     # C++ 模块管理器
│   ├── main.cpp
│   ├── git.cpp
│   ├── module_manager.cpp
│   ├── project_manager.cpp
│   └── utills.cpp
│
├── Updater/                 # C++ 自动更新器
│   ├── main.cpp
│   ├── updater.cpp
│   ├── windows.cpp
│   └── linux.cpp
│
├── mods/                    # 已启用 JS 模块
├── disabled/                # 已停用 JS 模块
├── cache/                   # Quernc 缓存
│   ├── source/              # 单元源码快照（MD5 比对基准）
│   └── bytecode/            # 单元拼装后的 .qb
└── Logs/                    # Launcher 文件日志
    └── <YYYY>/<MM>/<DD>/<HHMMSS>_<mode>_<traceid>_log.log
```

---

## 7. 设计要点 / 权衡

1. **多实现同字节码**：Qvm（Rust）与 QuernBuild（C++）各自独立实现 `.qb` 解析与语义，互为交叉验证；字节码规范是唯一契约。
2. **栈 + 变量寄存器双层**：物理栈用于 `out`/`pop`/`print_top` 的 LIFO 语义；`var_set`/`var_get` 寄存器用于 `psh <name> + N` 的跨指令累加，避免"push 即覆盖"。
3. **`jmp ... cal f` 立即 return 语义**：确保 If 命中后不会继续执行 else 分支，也不需要显式 `ret` 指令。
4. **单元级缓存**：以 `Function/Class/Entrust` 为单位做 MD5，小改动只重译受影响单元，大项目增量编译友好。
5. **JS Mod 系统**：`goja` 嵌入 + `Quern.Reg(keyword, handler)`，让用户扩展语法而不改编译器；`CustomNode` 作为未识别关键字的载体。
6. **AOT 条件注入运行时**：`needs_stack_support()` 短路，纯打印脚本生成极简 C。
7. **工具链自愈**：QuernBuild 会检测并自动安装 clang；Qlm 会检测并引导安装 git。
8. **安全**：ModLoader 的 `Include` 做目录越权校验；Qlm 的 `sha256` 校验拒绝被篡改的模块。
9. **日志与可观测**：Launcher 每进程 UUID 作为 trace_id，双 layer（控制台 + 文件），便于追踪一次 AOT 构建的全过程。

---

## 8. 快速构建 / 运行

```bash
# 1) 编译各组件
cd Quernc        && go build -o Quernc.exe
cd Qvm           && cargo build --release
cd QuernBuild    && clang++ *.cpp -o QuernBuild.exe -std=c++17
cd QuerncLauncher && cargo build --release
cd Qlm           && cl /MT /UTF-8 /O2 /std:c++17 /EHsc main.cpp /link shell32.lib
cd Updater       && cl /MT /UTF-8 /O2 /std:c++17 /EHsc main.cpp /link shell32.lib

# 2) 使用
QuerncLauncher.exe --run        hello.q
QuerncLauncher.exe --vm-check   hello.q
QuerncLauncher.exe --aot-build  hello.q --osize
QuerncLauncher.exe --module-install SomeModule.js
```
