# Quern 库管理器（QLM）使用指南

> 本文档介绍 `QuernBuild.exe`的完整用法。
>
> **适用版本**：最新

## 1. AOT 是什么

Quern 提供两条执行路径：

| 路径 | 组件 | 输入 | 过程 | 输出 |
|------|------|------|------|------|
| **解释执行** | `Qvm`（Rust） | `.qb` 字节码 | 逐条解释 | 程序运行结果 |
| **AOT 编译** | `QuernBuild`（C++）+ clang | `.qb` 字节码 | 字节码 → C 源码 → 原生二进制 | 可执行文件 |

AOT 的核心思想是**把栈式字节码直接翻译成等价的 C 代码**，让 clang 的优化器（LLVM）接手后续的性能工作。它**不是** JIT，也不在运行时做动态优化——所有翻译都在构建阶段完成。

**适合 AOT 的场景**：

- 频繁启动、对冷启动延迟敏感的工具脚本；
- 需要分发给终端用户、不希望用户安装 Qvm 的场景；
- 计算密集、希望获得接近 C 性能的热点逻辑；
- 需要嵌入到现有 C/C++ 工程的场景（可直接复用生成的 C 源码）。

**不适合 AOT 的场景**：

- 开发迭代阶段（每次修改都需要重新编译）；
- 依赖 Mod 自定义语法的脚本（AOT 前端不加载 Mod，见第 9 节）；
- 需要 `<=` 运算符或 `Alias` 展开的脚本（见第 9 节）。

---

## 2. 架构与数据流

```
             Quern 源码 (hello.q)
                    │
                    ▼
        ┌───────────────────────┐
        │ Quernc (Go 前端)       │
        │ 词法/语法 → AST → 字节码 │
        └───────────────────────┘
                    │
                    ▼
          字节码 (hello.qb)
                    │
        ┌───────────┴────────────┐
        ▼                        ▼
 ┌──────────────┐      ┌─────────────────────────┐
 │ Qvm (Rust)   │      │ QuernBuild (C++ AOT 前端) │
 │ 解释执行      │      │ 读取 .qb → 生成 .qb.c     │
 └──────────────┘      └─────────────────────────┘
                                 │
                                 ▼
                        hello.qb.c（C 源码）
                                 │
                                 ▼
                        clang -O? hello.qb.c
                                 │
                                 ▼
                        hello.qb.exe（原生可执行）
```

**关键组件职责**：

| 组件 | 语言 | 职责 |
|------|------|------|
| `Quernc` | Go | 源码 → AST → 字节码；DCE、缓存、Mod 加载；可触发 AutoAOT |
| `QuernBuild` | C++ | 读取字节码 → 调用 `CodeGenerator` 生成 C → 调用 clang |
| `CodeGenerator` | C++ | 把每条字节码翻译为 C 语句；注入运行时库 |
| `clang` | C | 将 C 源码编译为原生二进制 |

> **命名说明**：README 中称 AOT 后端为 **QuernAOT**，而代码（`run_complier.cpp`、`arg_logics.rs`、`main.go`）中实际使用的可执行文件名是 **QuernBuild.exe**。两者指同一组件，本文档统一用 `QuernBuild`。

---

## 3. 快速开始

### 3.1 直接编译已有字节码

```bash
# 1. 先用 Quernc 生成字节码（--Run 也会执行，此处只为生成 .qb）
Quernc.exe --Run hello.q
# 字节码位于 cache/bytecode/hello.qb

# 2. AOT 编译
QuernBuild.exe --ClangOFAST cache/bytecode/hello.qb

# 3. 运行（Windows 下生成 .qb.exe）
cache/bytecode/hello.qb.exe
```

### 3.2 通过 Launcher 两步构建

```bash
QuerncLauncher.exe --aot-build hello.q --ofast
```

`--aot-build` 的执行流程（来自 `arg_logics.rs` 的 `execute_aot_build`）：

1. 计算字节码路径：`./cache/bytecode/<文件名去扩展名>.qb`；
2. 执行 `Quernc --Run hello.q`（生成字节码）；
3. 根据 Launcher 的优化标志选择 `opt_flag`（默认 `--ClangOSize`）；
4. 执行 `QuernBuild <opt_flag> ./cache/bytecode/hello.qb`；
5. 若指定，追加 `--CVerbose` / `--ForceWarn` / `--NoWarn`。

### 3.3 通过 Go 前端 AutoAOT

```bash
Quernc.exe --Run hello.q --AutoAOTMax 500 --AutoAOTPar "--ClangOFAST --CVerbose"
```

执行 Qvm 后自动调用 `./QuernBuild.exe`（默认 `--ClangOFAST`），详见第 7 节。

---

## 4. 命令行选项

`QuernBuild` 接受一个字节码文件作为位置参数，以及若干优化/诊断标志：

```
QuernBuild [选项] <file.qb>
```

| 选项 | 对应 clang 标志 | 说明 |
|------|-----------------|------|
| `--ClangOSize` | `-Os` | 优化体积（**Launcher 默认**） |
| `--ClangOSizeBest` | `-Oz` | 极致体积优化 |
| `--ClangODebug` | `-Og` | 调试友好优化 |
| `--ClangOFAST` | `-Ofast` | 性能优先（**Go AutoAOT 默认**） |
| `--NotO` | `-O0` | 关闭优化（便于调试生成的 C） |
| `--CVerbose` | `-v` | 输出 clang 详细编译信息 |
| `--ForceWarn` | `-Werror` | 警告视为错误 |
| `--NoWarn` | `-w` | 抑制所有警告 |

**构建命令**（`run_complier.cpp`）：

```bash
clang <input>.qb.c -o <output> <optimization> [额外标志]
```

**输出路径**（`main.cpp`）：

| 平台 | 输出 |
|------|------|
| Windows | `<input_file>.exe`（例如 `hello.qb.exe`） |
| Linux/macOS | `<input_file>`（例如 `hello.qb`，**会覆盖输入文件同名路径，注意**） |

>  非 Windows 平台输出名与输入字节码文件相同（`out_file = input_file`），例如输入 `cache/bytecode/hello.qb` 时输出也是 `cache/bytecode/hello.qb`。这会覆盖原字节码文件。建议在非 Windows 平台手动备份 `.qb`，或在调用前重命名输入。

---

## 5. 生成的 C 代码结构

`CodeGenerator::generate(prog)` 生成的 C 文件由以下几部分组成：

### 5.1 头部与条件包含

```c
#include <stdio.h>

// 仅当程序使用栈/比较/变量相关指令时，才注入完整运行时
#ifdef _WIN32
#define strdup _strdup
#endif
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
// ... Stack 结构、函数 ...
```

**`needs_stack_support` 判定**：只要程序中存在 `crt` / `psh` / `psh_op` / `pop` / `del` / `jmp` / `out_var` 中任意一条指令（包括函数体内），就注入完整运行时；否则仅保留 `#include <stdio.h>`。

### 5.2 栈运行时

```c
typedef struct {
    char** items;
    int size;
    int capacity;
} Stack;

void init_stack(Stack* s);
void push_stack(Stack* s, const char* val);
void pop_stack(Stack* s);
void free_stack(Stack* s);
void print_top(Stack* s);
```

- 初始容量 16，按 2 倍扩容；
- 元素用 `strdup` 复制，`free_stack` 统一释放；
- `print_top` 输出栈顶并**追加一个空格**，空栈输出 `(empty) `。

### 5.3 全局栈注册表

```c
typedef struct { const char* name; Stack* s; } StackRef;
StackRef __stack_registry[256];
int __stack_registry_len;

void register_stack(const char* name, Stack* s);
Stack* find_stack(const char* name);
const char* get_top(Stack* s);
const char* get_top_by_name(const char* name);
```

- 上限 **256 个**栈；超出时静默不注册（后续 `find_stack` 返回 `NULL`）；
- 栈名到 `Stack*` 的映射由 `main()` 在启动时注册。

### 5.4 变量寄存器

```c
typedef struct { char* name; char* val; } VarVal;
VarVal __var_store[256];
int __var_store_len;

const char* var_get(const char* name);
void var_set(const char* name, const char* val);
void out_varval(const char* name);
void psh_arith(const char* name, const char* op, const char* operand);
```

**设计动机**：`psh <stack> <op> <operand>` 需要在不弹栈的情况下累积运算结果。`var_set` 会把值同时写入：

1. 物理栈的栈顶（**仅当栈非空**时覆盖）；
2. 变量寄存器（新增或覆盖）。

`psh_arith` 从寄存器（或物理栈顶，或 0）读取基数，执行运算，结果格式化为整数（`%lld`）或 `%g` 浮点，写回寄存器。

**示例**：

```quern
Data.Var x = 10
Data.Var x + 5     // 生成 psh_arith("x", "+", "5")，x 寄存器与栈顶均变为 15
```

### 5.5 比较

```c
int compare_values(const char* lv, const char* rv, const char* op);
int compare_stacks(Stack* l, Stack* r, const char* op);
```

- 自动区分数值（`strtod` 完整消费）与字符串（`strcmp`）；
- 数值相等使用 `1e-9` 容差；
- 支持 `=`、`!=`、`<`、`>`、`<=`、`>=`（注意：**不支持** `=<`）。

### 5.6 全局栈变量与函数

```c
// 每个收集到的栈名一个全局变量（名称经 sanitize_c_id 处理）
Stack myStack;
Stack _100;   // 名字以数字开头时加下划线

// 函数统一加 qb_ 前缀
void qb_Main();
void qb_helper();
```

`sanitize_c_id` 规则：

- 空名保持空；
- 以数字开头时前置 `_`；
- 非字母数字下划线字符替换为 `_`。

### 5.7 `main()` 布局

```c
int main() {
    // 1. 注册所有栈
    register_stack("myStack", &myStack);
    // 2. 初始化所有栈
    init_stack(&myStack);
    // 3. 执行顶层指令
    ...
    // 4. 释放所有栈
    free_stack(&myStack);
    // 5. 若存在名为 "exit" 的函数，调用它
    qb_exit();
    return 0;
}
```

### 5.8 `exit` 函数的特殊处理

若用户定义了名为 `exit` 的函数，`generate` 会在该函数体末尾追加：

```c
return 0;
```

注释说明这是为了避免使用 `exit(0)` 导致提前退出、跳过栈释放。**但该函数被声明为 `void qb_exit()`**，在 `void` 函数中 `return 0;` 不符合 C 标准，clang 通常会报：

```
error: 'return' with a value, in function returning void
```

**规避**：不要定义名为 `exit` 的函数；或使用 `--NoWarn` 之外的方式处理（实际上这是编译错误而非警告，`--NoWarn` 无法抑制）。这是一个已知实现问题，建议在后续版本修复。

---

## 6. 优化等级选择

| 场景 | 推荐标志 | 理由 |
|------|----------|------|
| 开发调试生成代码 | `--NotO --CVerbose` | 保留源码结构，便于对照字节码 |
| 发布工具脚本 | `--ClangOSize` | 体积小、启动快 |
| 极致体积（嵌入式） | `--ClangOSizeBest` | `-Oz` |
| 计算密集 | `--ClangOFAST` | 激进优化（含 `-ffast-math` 语义） |
| 需要可调试原生程序 | `--ClangODebug` | `-Og` |
| 严格 CI | `--ForceWarn` | 警告即失败，保证代码质量 |

>  `-Ofast` 会启用浮点重排与非 IEEE 语义。若脚本依赖精确浮点相等或特殊值（NaN/Inf）行为，优先使用 `--ClangOSize` 或 `--ClangODebug`。

**注意优化等级与运行时的关系**：生成的 C 运行时使用 `strdup`、`malloc`、`realloc`、`strtod` 等标准库函数，`-Ofast` 不会改变其语义；主要影响的是 `compare_values` 等内联后的代码。

---

## 7. AutoAOT 机制（现状说明）

Go 前端（`main.go`）在 Qvm 执行完成后，若未关闭 AutoAOT，会自动调用 `QuernBuild`。

### 7.1 触发条件

```go
if !autoAOTClose {
    // 构造 QuernBuild 命令并执行
}
```

**默认行为**：**每次** `Quernc --Run` 执行完都会触发一次 AOT 编译（默认 `--ClangOFAST`）。

### 7.2 相关标志

| 标志 | 默认 | 作用 |
|------|------|------|
| `--AutoAOTClose` | 关闭（即默认**开启**） | 完全禁用 AutoAOT |
| `--AutoAOTMax N` | `1000` | **函数调用阈值**（见下） |
| `--AutoAOTPar "params"` | 空 | 追加给 QuernBuild 的参数，按空格分割 |

### 7.3 `--AutoAOTMax` 的当前语义

`main.go` 中解析 `--AutoAOTMax` 后，仅用于**打印日志**：

```go
fmt.Printf("[Info] AutoAOT: threshold=%d, extra_params=%q\n", autoAOTMax, autoAOTPar)
```

**代码中没有任何基于该阈值的判断逻辑**——不存在函数调用计数器，也不存在"达到阈值才编译"的分支。因此当前 `--AutoAOTMax` **不会改变是否触发 AOT**，只是把值打印出来。

> 这是与 ROADMAP 中"热点检测"设计之间的**实现差距**：阈值参数已预留，热点计数尚未实现。

### 7.4 执行日志示例

```
[Info] AutoAOT: threshold=1000, extra_params=""
[AutoAOT] Running: ./QuernBuild.exe --ClangOFAST cache/bytecode/hello.qb
...
[AutoAOT] AOT compilation completed successfully.
```

失败时：

```
[AutoAOT] QuernBuild failed: exit status 1
```

但**不会**影响 `Quernc` 的整体退出码（AOT 失败仅记录，脚本解释执行已经完成）。

### 7.5 何时应关闭 AutoAOT

- **开发迭代期**：每次运行都触发 clang 编译，显著拖慢反馈循环 → 使用 `--AutoAOTClose`；
- **仅需解释执行**：CI 中只跑 Qvm；
- **脚本不兼容 AOT**（如依赖 Mod 自定义语法）→ 必须关闭，否则每次运行末尾都会打印 AOT 失败。

---

## 8. 热点检测（设计现状与路线图）

>  **重要**：截至当前代码库，Quern **尚未实现运行时热点检测**。ROADMAP 将"热点检测与 AOT"标记为"进行中"。

### 8.1 当前实际状态

| 能力 | 状态 |
|------|------|
| 全量 AOT（每次运行后编译整个字节码） | ✅ 已实现（AutoAOT） |
| 按函数调用次数统计热点 | ❌ 未实现 |
| 只 AOT 编译热点函数 | ❌ 未实现 |
| 字节码级 profile 数据输出 | ❌ 未实现 |
| 运行时 JIT 替换热点函数 | ❌ 未实现 |

### 8.2 设计意图（来自 ROADMAP 与代码预留）

- `--AutoAOTMax N`：计划作为"函数调用计数达到 N 时触发 AOT"的阈值；
- `--AutoAOTPar`：为后续按函数编译提供参数通道；
- `Qvm`（Rust）侧需要增加**调用计数器**，并在达到阈值时输出热点函数列表；
- `QuernBuild` 需要支持**按函数子集编译**（当前 `CodeGenerator` 总是生成完整程序）。

### 8.3 现阶段用户可用的替代方案

1. **手动 AOT**：先用 `Qvm` 跑通、确认稳定，再 `QuernBuild --ClangOFAST` 全量编译。
2. **选择性优化**：将热点逻辑抽成独立函数，单独用 `--ClangOFAST` 全量编译，非热点部分影响有限。
3. **基准对比**：分别用 `Qvm` 与 AOT 产物运行同一脚本，用外部工具（`time`、`hyperfine`）对比，决定是否值得 AOT。

---

### 9.6 栈名与变量名规范

- `collect_stack_names` 会把 `@name@` 规范化为 `name` 收集，因此 `@x@` 与 `x` 指向同一个 `Stack` 全局变量；
- 栈名上限 256（`__stack_registry` 与 `__var_store` 数组大小）；
- 栈名会经 `sanitize_c_id` 转换；两个不同的 Quern 栈名可能映射到同一个 C 标识符（例如 `a-b` 与 `a_b`），导致冲突。**规避**：栈名只使用字母、数字、下划线，且不以数字开头。

### 9.7 `psh_arith` 的操作数不支持变量

```c
void psh_arith(const char* name, const char* op, const char* operand) {
    double rhs = strtod(operand, NULL);
    ...
}
```

`operand` 直接 `strtod`，因此 `psh x + @y@` 的 `@y@` 会解析为 0（`strtod` 失败返回 0）。**规避**：算术操作数只使用数值字面量。

### 9.8 `print_top` 的尾部空格

```c
printf("%s ", s->items[s->size - 1]);
```

`out` 指令在目标为栈名时，输出栈顶并**追加一个空格**。若脚本依赖精确输出，需注意这一点；Qvm 的 `Out` 同样追加空格，因此两者一致。

---

## 10. 故障排查

### 10.1 `Clang not found`

`main.cpp` 在编译前调用 `check_clang()`：

- 通过：继续；
- 失败：调用 `download_toolchain()`：
  - **Windows**：通过 PowerShell 下载 LLVM 17.0.6 安装包（`LLVM-17.0.6-win64.exe`）并静默安装（`/S`）；
  - **macOS**：先 `xcode-select --install`，失败则 `brew install llvm`；
  - **Linux**：依次尝试 `apt-get` / `dnf` / `pacman`。

安装后仍需 `check_clang()` 通过，否则报：

```
Clang is still not found after installation attempt. Please install manually and ensure it's in PATH.
```

**手动修复**：从 [llvm.org](https://releases.llvm.org/) 安装对应平台 LLVM，确保 `clang` 在 PATH 中，重启终端。

### 10.2 `Compilation failed`

`run_command(cmd)` 返回非 0。常见原因：

| 现象 | 原因 | 处理 |
|------|------|------|
| 大量 `warning` 导致失败 | 使用了 `--ForceWarn` | 去掉该标志，或修复警告源 |
| `'return' with a value, in function returning void` | 定义了名为 `exit` 的函数 | 重命名函数（见 5.8） |
| 未定义符号 | 生成 C 缺少运行时（`needs_stack_support` 误判） | 检查字节码是否含栈指令；必要时手动加 `crt` |
| 栈名冲突 | 两个 Quern 栈名 sanitize 后相同 | 重命名栈 |

### 10.3 AOT 产物运行结果与 Qvm 不一致

按以下顺序排查：

1. **是否使用了 `<=` 或 `=<`**？→ 改用 `not (x > y)`（见 9.2）；
2. **是否依赖 `Alias`**？→ 改用 `Data.Var` + `@var@`（见 9.3）；
3. **是否使用 Mod 自定义语法**？→ AOT 不加载 Mod（见 9.4）；
4. **是否使用 `Loop` 但未加 `--ExpandLoop`**？→ 循环被跳过（见 9.5）；
5. **是否在 `psh ... <op> @var@` 中使用变量操作数**？→ 改为数值字面量（见 9.7）。

### 10.4 AutoAOT 每次运行都编译、拖慢开发

```bash
Quernc.exe --Run hello.q --AutoAOTClose
```

或仅在发布时手动 `QuernBuild`。

### 10.5 非 Windows 平台输出覆盖输入

`main.cpp` 在非 Windows 下 `out_file = input_file`。若输入 `cache/bytecode/hello.qb`，输出会覆盖该文件。

**规避**：编译前复制一份 `.qb`，或将输入重命名为临时文件。

---

## 11. 性能建议

### 11.1 让 AOT 真正"跑得快"的前提

AOT 的性能收益来自 clang 对生成 C 的优化。以下几点影响优化效果：

1. **热路径避免字符串化数值运算**：Quern 的栈元素是 `char*`，算术走 `strtod` / `snprintf`。即使 AOT 后，这部分仍是字符串↔数值转换开销。计算密集逻辑建议尽量用 `Data.Var Int` 并在编译期完成常量折叠（`--ConstCalc`）。
2. **减少栈操作**：每次 `psh` 都 `strdup`，每次 `pop` 都 `free`。高频循环中栈操作是主要开销。可用局部变量（`crt` 一个新栈）替代反复压弹。
3. **避免动态字符串拼接**：`out` 中的 `@var@` 插值在生成 C 中表现为多次 `printf` + `get_top_by_name` 查找（线性扫描 256 项注册表）。热点输出路径建议预先拼接好字符串再单次 `out`。
4. **控制栈数量**：注册表线性查找，栈越少 `find_stack` 越快（上限 256，超过不注册）。

### 11.2 优化等级选择

| 目标 | 建议 |
|------|------|
| 冷启动延迟 | `--ClangOSize` 或 `--ClangOSizeBest`（更小的二进制、更少的页加载） |
| 吞吐量 | `--ClangOFAST` |
| 可复现的数值行为 | `--ClangOSize`（避免 `-ffast-math`） |
| 调试生成 C | `--NotO --CVerbose` |

### 11.3 基准测试方法

```bash
# 1. 生成字节码
Quernc.exe --Run bench.q

# 2. 分别构建两个版本
QuernBuild.exe --ClangOSize cache/bytecode/bench.qb
cp cache/bytecode/bench.qb.exe bench_os.exe

QuernBuild.exe --ClangOFAST cache/bytecode/bench.qb
cp cache/bytecode/bench.qb.exe bench_ofast.exe

# 3. 用 hyperfine 对比（排除冷启动噪声）
hyperfine --warmup 3 './bench_os.exe' './bench_ofast.exe'

# 4. 与 Qvm 解释执行对比
hyperfine --warmup 3 'Qvm.exe --Run cache/bytecode/bench.qb' './bench_ofast.exe'
```

> 基准时注意：Qvm 的输出编码、栈空格规则与 AOT 产物一致，可直接对比 stdout。

### 11.4 何时不该用 AOT

- 脚本运行时间 < 编译时间（一次性工具）；
- 频繁修改的开发期；
- 依赖 Mod 扩展语法的脚本；
- 需要 `Alias` / `<=` 且不愿改写代码的存量脚本。

---

## 附录：AOT 工作流速查

```bash
# 开发期：解释执行，关闭 AutoAOT
Quernc.exe --Run app.q --AutoAOTClose --ExpandLoop

# 调试生成 C
QuernBuild.exe --NotO --CVerbose cache/bytecode/app.qb

# 发布（体积优先）
QuernBuild.exe --ClangOSize cache/bytecode/app.qb

# 发布（性能优先）
QuernBuild.exe --ClangOFAST cache/bytecode/app.qb

# 一键两步构建（Launcher）
QuerncLauncher.exe --aot-build app.q --ofast --aot-no-warn
```