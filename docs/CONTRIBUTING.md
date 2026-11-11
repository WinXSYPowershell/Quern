# CONTRIBUTING.md — 贡献指南

> 感谢你愿意为 **Quern** 出一份力。Quern 是一个多语言协作的语言工具链：Go 翻译器、Rust 虚拟机 + 启动器、C++ AOT 编译器 / 模块管理器 / 更新器，加上 JS Mod 扩展。这份文档帮助你**第一次提 PR 就符合项目预期**。
>
> 先读这两个再动手：
> - [`ARCHITECTURE.md`](./ARCHITECTURE.md) —— 各组件职责、字节码规范、数据流
> - [`BUILD.md`](./BUILD.md) —— 从源码构建全部组件、已知硬编码路径清单

---

## 目录

1. [行为准则](#1-行为准则)
2. [我能贡献什么](#2-我能贡献什么)
3. [开发环境](#3-开发环境)
4. [仓库结构速览](#4-仓库结构速览)
5. [工作流：Fork → 分支 → PR](#5-工作流fork--分支--pr)
6. [提交信息规范](#6-提交信息规范)
7. [代码风格](#7-代码风格)
8. [测试要求](#8-测试要求)
9. [改字节码？先读这一节](#9-改字节码先读这一节)
10. [各组件注意事项](#10-各组件注意事项)
11. [文档贡献](#11-文档贡献)
12. [Mod 贡献](#12-mod-贡献)
13. [已知陷阱（务必避坑）](#13-已知陷阱务必避坑)
14. [PR 检查清单](#14-pr-检查清单)
15. [审阅与沟通](#15-审阅与沟通)

---

## 1. 行为准则

- 对事不对人。技术争论可以有，人身攻击不接受。
- 讨论使用**中文或英文**均可，但**代码、注释、commit message 用英文**（便于国际协作与工具链兼容）。
- 提出问题前，请先搜索 [Issues](../../issues) 和 `ARCHITECTURE.md` / `BUILD.md`，避免重复。

---

## 2. 我能贡献什么

不一定非要写编译器。以下都是**有价值的贡献**：

| 类型 | 具体举例 | 涉及目录 |
|------|----------|----------|
| **Bug 报告** | 用最小 `.q` 复现 VM 崩溃、AOT 产物与 VM 解释结果不一致 | — |
| **功能请求** | 新的字节码指令、新优化 pass、CLI 参数 | — |
| **Bug 修复** | Qvm 比较语义、QuernBuild 内存泄漏、Qlm 的 sha256 校验 | `src/qvm`、`src/aot`、`src/qlm` … |
| **新特性** | DCE 增强、Mod 系统 API、`.toml` 依赖新字段 | 对应组件 |
| **测试** | 补 `.q` 示例、字节码快照、C 生成快照 | `tests/`（建议新建） |
| **文档** | 修正 `ARCHITECTURE.md` / `BUILD.md`、补充示例 | `*.md` |
| **Mod 扩展** | 发布一个 JS 语法扩展，放进 `mods/` | `mods/` |
| **构建/CI** | 修复 `build.py` 硬编码路径、加 GitHub Actions | `build.py`、`.github/` |

不确定从哪开始？看 Issues 上标了 **`good first issue`** 的项，或直接在 Issue 里问"我可以做这个吗？"。

---

## 3. 开发环境

完整的搭建步骤见 [`BUILD.md`](./BUILD.md)。快速版：

- **Windows**：Python 3.8+、Go 1.20+、Rust（rustup）、clang++、WSL2（Linux 交叉构建）、7-Zip、Inno Setup
- **Linux / macOS**：Go、Rust、clang++/g++、Git 即可本地构建对应平台产物

自检：

```bash
go version
cargo --version
clang++ --version
git --version
```

如果你只想改 **Qvm**（Rust），环境里只需要 Rust + Git；改 **QuernBuild** 只要 clang++。**不必为每个 PR 装全套工具链** —— 只构建你改动的组件即可。

---

## 4. 仓库结构速览

```
quern-toolchain/
├── src/
│   ├── translator/          # Quernc（Go）
│   ├── qvm/                 # Qvm（Rust）       → main.rs
│   ├── logger/              # Launcher（Rust）   → main.rs
│   ├── aot/                 # QuernBuild（C++）
│   ├── qlm/                 # Qlm（C++）
│   └── update/              # Updater（C++）
├── mods/                    # 已启用 JS Mod
├── tests/                   # 测试用例（欢迎补充）
├── build.py                 # 主构建脚本
├── build.json               # 构建配置
├── ARCHITECTURE.md
├── BUILD.md
└── CONTRIBUTING.md          # ← 你在这里
```

对应表见 [`ARCHITECTURE.md` §6 文件组织](./ARCHITECTURE.md#6-文件组织) 和 [`BUILD.md` §10](./BUILD.md#10-组件与构建产物对照表)。

---

## 5. 工作流：Fork → 分支 → PR

### 5.1 分支命名

| 前缀 | 场景 | 示例 |
|------|------|------|
| `feat/` | 新特性 | `feat/qvm-arithmetic-precision` |
| `fix/` | Bug 修复 | `fix/aot-var-set-leak` |
| `docs/` | 纯文档 | `docs/contributing-guide` |
| `refactor/` | 不改行为的重构 | `refactor/translator-split-if` |
| `test/` | 只加/改测试 | `test/qvm-compare-numeric` |
| `chore/` | 构建、CI、依赖 | `chore/build-py-wsl-paths` |
| `perf/` | 性能优化 | `perf/dce-reachability` |

**一条分支只解决一个问题**。混在一起会让 review 变慢，也更容易被拒。

### 5.2 基本流程

```bash
# 1. Fork 到你自己账号，然后：
git clone https://github.com/<you>/quern-toolchain.git
cd quern-toolchain

# 2. 加 upstream
git remote add upstream https://github.com/<org>/quern-toolchain.git

# 3. 从最新 main 切分支
git fetch upstream
git checkout -b feat/your-feature upstream/main

# 4. 提交（多次小 commit 也可以）
git add -A
git commit -m "feat(qvm): support hex literals in arithmetic"

# 5. 保持与 upstream 同步（可选但推荐）
git fetch upstream
git rebase upstream/main

# 6. 推送到你的 fork
git push origin feat/your-feature
```

然后在 GitHub 上开 PR，基分支选 **`upstream/main`**。

### 5.3 遇到 DCE / 缓存问题

Quernc 的缓存以 **Unit**（Function / Class / Entrust）为单位。若你改了 `translator_core.go` 的翻译逻辑，请**清掉 `cache/` 目录**再测，否则可能拿到旧 `.qb`：

```bash
rm -rf cache/
```

---

## 6. 提交信息规范

采用 **Conventional Commits** 的**作用域**变体：

```
<type>(<scope>): <subject>

[optional body]

[optional footer]
```

### 6.1 type

`feat` / `fix` / `docs` / `style` / `refactor` / `test` / `chore` / `perf` / `build` / `ci`

### 6.2 scope（必填）

必须指明**主要影响组件**：

| scope | 组件 |
|-------|------|
| `translator` | Quernc（Go） |
| `qvm` | Qvm（Rust VM） |
| `launcher` | QuerncLauncher（Rust） |
| `aot` | QuernBuild（C++） |
| `qlm` | Qlm（C++） |
| `updater` | Updater（C++） |
| `mod` | JS Mod 系统 / `modloader.go` |
| `build` | `build.py` / `build.json` |
| `docs` | Markdown |
| `bc` | **字节码规范**（跨组件） |
| `repo` | 仓库级杂项 |

### 6.3 示例

```
feat(qvm): support @var@ in jmp operands
fix(aot): free var_set store on exit
docs(contributing): add bc-change checklist
refactor(translator): extract if/else codegen into helpers
bc: add psh_op instruction for in-place arithmetic
perf(qlm): cache git ls-remote result for 60s
```

### 6.4 破坏性变更

如果 PR 改变了**字节码语义**或**对外 CLI 行为**，在 footer 加：

```
BREAKING CHANGE: <具体描述>
```

或在 type 后加 `!`：`feat(bc)!: ...`。

---

## 7. 代码风格

### 7.1 通用

- 缩进：**4 空格**（Rust 除外，见下）
- 行宽：建议 ≤ 120
- 文件编码：**UTF-8 without BOM**
- 换行：`.gitattributes` 中统一 **LF**（Windows 用户请配 `core.autocrlf=input`）
- 注释：**英文**，复杂逻辑上方解释**为什么**，而不是复述代码在做什么

### 7.2 Quernc（Go）

```bash
cd src/translator
gofmt -w .
go vet ./...
```

- 严格遵循 `gofmt`（PR 会跑 CI 检查）
- import 分组：**标准库 / 第三方 / 项目内**，三组空行分隔
- 错误：`fmt.Errorf("context: %w", err)`，不要 `panic` 传播用户输入错误
- 命名：`Parser`、`Translator`、`DeadCodeEliminator` —— 已用的名字保持一致
- 不要在 `translator_core.go` 里加超过 100 行的单函数；拆到 helper

### 7.3 Qvm / Launcher（Rust）

```bash
cargo fmt
cargo clippy -- -D warnings
```

- 遵循 `rustfmt` 默认配置
- 避免 `unwrap()` 处理外部输入 —— 用 `?` / `ok_or` / `match`
- 错误优先用 `thiserror`（若引入）或 `String`（现有风格）
- `main.rs` 被 build.py 覆写是**预期行为**（见 BUILD.md §6.1），提交 PR 时若发现 `src/main.rs` 被改动，**请还原**：`git checkout src/main.rs`

### 7.4 QuernBuild / Qlm / Updater（C++）

```bash
# 建议（若没装 clang-format 可手工遵守）
clang-format -i src/aot/*.cpp src/qlm/*.cpp src/update/*.cpp
```

- C++17，不使用 C++20 特性
- 优先 `std::string` / `std::vector` / `std::filesystem`；避免裸 `new` / `delete`；已有 `Stack` 是 C 风格，但在其附近代码里保持一致
- Windows 特定代码必须包 `#ifdef _WIN32`，参照 `errors.cpp` / `run_complier.cpp` / `windows.cpp` 的写法
- 头文件包含顺序：标准库 → 平台 → 项目内
- 不改 `ganerator.cpp` 的**文件名拼写**（`ganerator` 是历史遗留，PR 里不要顺手"修正"，会破坏现有脚本）

### 7.5 JS Mod

见 [§12](#12-mod-贡献)。

---

## 8. 测试要求

项目当前**没有强制单测框架**，但 PR 必须**手工验证**并把验证方法写进 PR 描述。**欢迎为任何组件补测试**：

| 组件 | 建议测试方式 |
|------|--------------|
| Quernc | `.q` 输入 → 期望 `.qb` 快照；`--UnuseDeadCodeEli` 前后对比 |
| Qvm | `vm.rs` 内 `#[test]`；`.qb` 夹具 → 预期 stdout |
| QuernBuild | `.qb` → 生成 `.c` 快照；再 `clang` 编译 → 预期 stdout |
| Launcher | CLI 参数组合的集成测试；`--logo`、`--help` 快照 |
| Qlm | 临时目录 + fake `git` 脚本模拟安装流程；sha256 校验命中/不命中 |
| ModLoader | JS handler 注册与调用；`Include` 越权用例 |

**关键点：VM 与 AOT 必须产出相同 stdout**。涉及 `qvm` 或 `aot` 的 PR，请对同一 `.qb` 跑两条路径：

```bash
# VM
./Qvm --Run tests/cases/hello.qb > out.vm.txt

# AOT
./QuernBuild tests/cases/hello.qb      # → tests/cases/hello.qb.c + 可执行文件
./tests/cases/hello.qb                 # 若 QuernBuild 输出为 <input>.exe/<input>
> out.aot.txt

diff out.vm.txt out.aot.txt
```

> `QuernBuild` 产物命名见 `main.cpp` 中 `out_file` 逻辑（Windows 为 `<input>.qb.exe`，其它平台去掉 `.exe`）。

---

## 9. 改字节码？先读这一节

**字节码规范是 Quern 工具链的唯一定义契约**，被 3 个组件同时消费：

```
Translator（生成）──▶ .qb ──▶ Qvm（解释）
                        └──▶ QuernBuild（生成 C）
```

因此，**任何对 `.qb` 指令集、参数格式、比较语义的改动，必须在同一个 PR 里同步更新所有相关组件**，否则会出现 "VM 能跑但 AOT 崩溃" / "旧 `.qb` 不能跑" 的严重回归。

### 9.1 改字节码的固定动作清单

假设要新增一条指令 `foo <a> <b>`：

- [ ] **`ARCHITECTURE.md` §4.1**：更新指令表
- [ ] **`src/qvm/instructions.rs`**：`enum Instruction` 加变体
- [ ] **`src/qvm/parser.rs`**：解析该指令
- [ ] **`src/qvm/vm.rs`**：实现语义
- [ ] **`src/aot/instruction.cpp`**：`Instruction` 结构体 / `parse_comparison_op` 等若涉及
- [ ] **`src/aot/parser.cpp`**：解析该指令
- [ ] **`src/aot/ganerator.cpp`**：`generate_instr` 中生成 C 代码；必要时在 `get_runtime_code()` 里加 C 运行时辅助函数
- [ ] **`src/translator/translator_core.go`**：若新指令可由 Quern 源语言触达，`emit` 处同步
- [ ] **`src/translator/ast.go` / `parser.go`**：若 Quern 源语法也变了
- [ ] **测试**：至少一份 `.qb` 用例，跑通 VM + AOT 两条路径
- [ ] **PR 标题加 `bc` scope**，footer 若破坏兼容加 `BREAKING CHANGE`

### 9.2 兼容性策略

- **加**指令：默认兼容（旧 `.qb` 不含它）
- **改**指令语义：**破坏性变更**，需要 `BREAKING CHANGE:` footer + 在 `ARCHITECTURE.md` 标记版本
- **删**指令：破坏性变更，且要在 CHANGELOG 里给出迁移说明
- 编译缓存的 `.qb` 若与新语义冲突，需在 Quernc 缓存层加**版本号**（欢迎作为独立 PR）

### 9.3 `@var@` 与寄存器语义

`@name@` 是**虚拟变量**语法糖，物理载体是栈，但 Qvm 与 QuernBuild 都在 `var_get` / `var_set` 层维护了一个与栈顶解耦的数值寄存器（用于 `psh <name> + N` 跨指令累加）。**修改 `var_set` / `var_get` 的语义属于高危变更**，请：

1. 在 Issue 里先讨论方案
2. PR 中同时提供 `.q` 复现用例 + `Qvm` 与 `QuernBuild` 输出对比

---

## 10. 各组件注意事项

### 10.1 Quernc（`src/translator/`）

- **不要**移除 `stripQuernComments` 的块注释兼容分支（历史上是有意设计）
- **不要**把 `CustomNode` 的兜底逻辑改成 `error`——Mod 系统依赖它承载未知关键字
- `DeadCodeEliminator.Filter` 中 `usedFunctions` / `usedClasses` / `usedVars` / `usedLists` / `usedDicts` 的分类必须同步更新
- `CacheManager` 的 unit 名必须是 `Function/Class/Entrust` 的稳定标识，改名字会让缓存全失效
- `ModLoader` 的 `ProcessIncludes` 有目录越权检查，任何"支持 `../` 相对路径"的需求都是**安全需求变更**，需走 Issue 讨论

### 10.2 Qvm（`src/qvm/`）

- `Instruction::ConditionalJump` 命中后**立即 `return`**（不继续当前 body），这是语义不是 bug
- `Out` 的 `is_var=true` 语义是"**栈顶值作为栈名**再解引用"，不是"输出栈顶值"
- `Arithmetic` 用 f64 解析 + 整数格式化，改这个要同步改 `QuernBuild` 的 `psh_arith`
- `errors.rs` 的 `format_normal` / `format_verbose` 输出格式被 Launcher 解析，**改动前请先 grep** `format_normal` / `format_verbose` 的所有调用方

### 10.3 QuernBuild（`src/aot/`）

- `ganerator.cpp` 的**文件名拼写**是有意保留，别改
- `get_runtime_code()` 返回的 C 运行时是**内联字符串**，改动时务必用**原始字符串 `R"(...)"` 内的转义规则**检查 `\"` 和 `\\`
- `needs_stack_support()` 短路很关键，别让它返回 `true`（纯打印脚本产物会变大）
- 生成 C 代码里的**变量命名规则**（`sanitize_c_id`）和 `qb_` 前缀必须保持稳定，否则 PR 会破坏现有用户代码
- 新增平台支持（如 macOS）时，请照 `run_complier.cpp` 的 `check_clang()` / `download_toolchain()` 分支模式补全

### 10.4 QuerncLauncher（`src/logger/`）

- `Args` 里任何新命令行参数都要同步更新 `arg_logics.rs` 与 `main.rs` 的 `operation` 匹配
- `traceing.rs` 的日志文件路径变更要更新 `BUILD.md` / `ARCHITECTURE.md` 的日志目录约定
- `execute_aot_build` 的两步调用（`Quernc` → `QuernBuild`）中，**QuernBuild 的输入必须是 Quernc 产出的 `.qb` 路径**，不要自己拼

### 10.5 Qlm（`src/qlm/`）

- `sha256` 校验**必须**保留（安全需求，见 `module_manager.cpp`）
- `ProcessIncludes` 的目录越权检查（在 `modloader.go`）同理
- `--NewProject` 生成的 `.toml` 模板里的 `need = { ... }` 解析器在 `project_manager.cpp`，正则改动请补测试
- Windows 上 `--ProjectRun` 会调用 `Quernc.exe --Run src\main.q`，**要求 `Quernc.exe` 在 `PATH` 或同目录**

### 10.6 Updater（`src/update/`）

- Windows 实现里 `Base64Encode` + UTF-16 编码是**规避 PowerShell 转义**的必要手段，不要简化
- Linux 实现里 `getLatestTag()` 的 tag 排序是**字符串比较**（不是 semver），改这个要跟发布流程对齐

---

## 11. 文档贡献

- 三份主文档：`ARCHITECTURE.md`、`BUILD.md`、`CONTRIBUTING.md`
- 任何**改变用户可见行为**的 PR，必须同步改 `ARCHITECTURE.md`
- 任何**改变构建方式**的 PR，必须同步改 `BUILD.md`
- 新增 CLI 参数 / 指令 / 语法糖，都算"用户可见行为"
- 文档 PR 不需要跑构建，但请保持格式：Markdown、中文（技术名词可保留英文）、章节编号一致

---

## 12. Mod 贡献

Quern 的 Mod 系统允许用 **JavaScript** 扩展 Quern 源语言的语法（通过 `Quern.Reg(keyword, handler)` 注册，见 `modloader.go`）。

### 12.1 一个最小 Mod

```js
// mods/HelloMod.js
Quern.Reg("Hello", function (input) {
    // input: 原始源码行，例如: Hello "world" world
    // 返回: 一段合法 Quern 源语句（后续会被重新翻译）
    return `Console.Info("hello mod says: world")`;
});
```

对应源文件：

```
Hello "world" world
```

会被 Quernc 展开为 `Console.Info(...)` 并继续走正常流程。

### 12.2 提交 Mod 的约定

- 文件放 `mods/` 下（启用状态）或 `disabled/`（停用），**不要**直接改 `modloader.go`
- 文件名用 **PascalCase**（`HelloMod.js`）
- 注册的 keyword 必须**大写开头**（避免与内置关键字 `crt` / `psh` / `fnc` 冲突）
- handler 内**不要**做文件 I/O / 网络请求 —— Mod 沙箱只保证语法扩展
- `Include "xxx.js"` 只允许相对 `mods/` 的子路径，越权会被 `ProcessIncludes` 拒绝
- 在 PR 里附一个能跑通的 `.q` 示例 + 预期输出

### 12.3 破坏性 Mod 变更

Mod 系统的 `ModLoader` API 是**内部契约**，改名/改签名属于 `BREAKING CHANGE`，PR 必须同时更新 `ARCHITECTURE.md` §3.1（ModLoader 一节）与所有 `mods/*.js` 示例。

---

## 13. 已知陷阱（务必避坑）

来自 [`BUILD.md` §6 / §6.1](./BUILD.md#6-️-硬编码路径清单首次使用必改) 的提醒，PR 时注意：

1. **`build.py` 有 5 处硬编码路径**（`I:\Quern\...`、`/mnt/i/...`），跨机器构建前必须改。**欢迎发 PR 把它们改为基于 `root_path` 动态生成** —— 这是当前最容易上手的 `chore(build)` 贡献。
2. **`build_rust_*` 会覆写 `src/main.rs`**。提交 PR 前执行 `git checkout src/main.rs`，否则 diff 会带上无关改动。
3. **`build_rust_linux` 产物位置有 bug**（Linux 产物被 move 到 `Windows/`）。若修，请作为独立 PR，标题 `fix(build): linux rust artifacts target`。
4. **`qlm_linux` 输出名有 bug**（`-o .../QuernBuild` 但脚本 move `root_path/"Qlm"`）。修法：把 `-o` 改为 `.../Qlm`。
5. **`Cargo.toml` 的 `[[bin]] name` 必须是 `Quern_cargo`**，否则 `safe_move` 静默失败。新增 bin target 时请同步更新 `build.py`。
6. **不要改 `ganerator.cpp` 的文件名**（历史拼写），改了会破坏所有引用它的脚本 / IDE 配置。
7. **不要顺手"修正" `run_complier.cpp` 拼写**（历史遗留 `comlier`），同上。
8. **`src/main.rs` 的 diff 是 git status 变脏的常见来源** —— 构建后第一件事 `git status`，清理它。

---

## 14. PR 检查清单

提交前请逐项确认（可以在 PR 描述里勾选）：

**通用**
- [ ] 分支名符合 §5.1
- [ ] 一个 PR 只解决一个问题
- [ ] commit 信息符合 §6
- [ ] `git status` 干净（无 `src/main.rs` 等构建残留）
- [ ] 已基于最新 `upstream/main` 且无 merge commit（建议 rebase）

**代码**
- [ ] `gofmt` / `rustfmt` / `clang-format` 通过（按你改的语言）
- [ ] 无 `TODO` / `FIXME` / 调试 `printf` / `println!` 残留
- [ ] 无新增 `unwrap()`（Rust）/ 裸 `new`（C++）
- [ ] 错误信息包含组件名与上下文（便于用户定位）

**行为变更**
- [ ] 若改字节码：§9.1 清单全勾
- [ ] 若改 CLI：更新 `ARCHITECTURE.md` / `BUILD.md`
- [ ] 若改 `.qb` 语义：footer 加 `BREAKING CHANGE:`
- [ ] 若改 `var_set` / `var_get`：Issue 已讨论过方案

**测试**
- [ ] 手工验证步骤写进了 PR 描述（命令 + 输入 + 预期输出）
- [ ] 涉及 VM / AOT 的：两条路径 stdout 对比一致
- [ ] 涉及 DCE / 缓存的：清 `cache/` 后重跑

**文档**
- [ ] `ARCHITECTURE.md` 已同步（若涉及组件职责/字节码）
- [ ] `BUILD.md` 已同步（若涉及构建方式）

---

## 15. 审阅与沟通

- **PR 响应时间**：维护者通常在 **3 个工作日内**给出首轮反馈。超过 1 周无响应，@ 一下维护者。
- **审阅关注点**（按优先级）：
  1. 语义正确性（尤其字节码跨组件一致性）
  2. 破坏性变更是否必要 + 迁移说明
  3. 是否有回归风险（缓存、DCE、Mod 系统）
  4. 代码风格
- **反馈形式**：`nit:`（可改可不改）、`suggestion:`（建议）、`issue:`（必须改）、`question:`（问一下）。看到 `nit:` 可以不处理，但请回复"接受/忽略 + 理由"。
- **被拒 PR**：维护者会说明原因。**不要重开同一个 PR**，在新分支上按反馈重做再提。

### 沟通渠道

| 渠道 | 用途 |
|------|------|
| **GitHub Issues** | Bug、功能请求、设计讨论 |
| **GitHub PR** | 代码审阅 |
| **仓库 Discussions**（若开启） | 开放性问题、Mod 展示 |
| **提交者自己的 README / 日志** | 复现信息应包含：`Quernc --version`、OS、最小 `.q`、实际 vs 预期输出 |

---

## 附：常用命令速查

```bash
# 清理缓存（改翻译器后必做）
rm -rf cache/

# 只构建你改的组件（Windows）
python build.py 1          # 配合 build.json 只开对应 target

# 手动构建单个组件
cd src/translator && go build -o ../../bin/Quernc .
cd src/qvm && cp main.rs ../../src/main.rs && cargo build --release   # 注意覆写 main.rs

# 格式检查
gofmt -l src/translator
cargo fmt --check
clang-format --dry-run --Werror src/aot/*.cpp

# 还原 build.py 被覆写的 Rust 入口
git checkout src/main.rs
```

---

**感谢你的贡献！** 有任何不清楚的地方，开 Issue 或直接在 PR 里问，我们很乐意帮你把 PR 打磨到可合并。

*文档版本：v1.0*