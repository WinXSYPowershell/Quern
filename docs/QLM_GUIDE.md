# Quern 库管理器（QLM）使用指南

> 本文档介绍 `Qlm.exe`（QLM）的完整用法，包括模块安装/删除/启用/禁用、远程与本地检索、项目创建、依赖声明与一键运行。
>
> **适用版本**：QLM 2.0

---

## 目录

1. [QLM 是什么](#1-qlm-是什么)
2. [快速开始](#2-快速开始)
3. [目录结构](#3-目录结构)
4. [命令参考](#4-命令参考)
   - 4.1 [`--Help`](#41---help)
   - 4.2 [`--Install`](#42---install)
   - 4.3 [`--Delete`](#43---delete)
   - 4.4 [`--Disable` / `--Enable`](#44---disable--enable)
   - 4.5 [`--WebList`](#45---weblist)
   - 4.6 [`--ModsList`](#46---modslist)
   - 4.7 [`--WebSearch` / `--ModsSearch`](#47---websearch--modssearch)
   - 4.8 [`--NewProject`](#48---newproject)
   - 4.9 [`--InstallPackage`](#49---installpackage)
   - 4.10 [`--ProjectRun`](#410---projectrun)
5. [依赖声明格式（TOML）](#5-依赖声明格式toml)
6. [版本锁定与 SHA256 校验](#6-版本锁定与-sha256-校验)
7. [通过 Launcher 调用 QLM](#7-通过-launcher-调用-qlm)
8. [环境要求](#8-环境要求)
9. [故障排查](#9-故障排查)
10. [已知限制](#10-已知限制)

---

## 1. QLM 是什么

QLM（Quern Libraries Manager）是 Quern 生态的**模块包管理器**。它负责：

- 从远程 Git 仓库**发现、下载、安装** `.js` 模块（Mod）到本地 `mods/` 目录；
- 支持**版本锁定**（按时间点 checkout）与 **SHA256 完整性校验**；
- 本地模块的**删除、禁用、启用、检索**；
- 基于 `.toml` 的**依赖声明**与**批量安装**；
- 一键**创建新项目**并**运行项目**（自动检查并补全依赖）。

QLM 安装的 `.js` 模块由 Quern 编译器前端（Go）在启动时通过 goja 引擎加载，用于扩展自定义语法（参见 `API_REFERENCE.md` 第 6 节）。

---

## 2. 快速开始

```bash
# 1. 查看远程仓库有哪些模块
Qlm.exe --WebList

# 2. 安装一个模块
Qlm.exe --Install utils.js

# 3. 确认已安装
Qlm.exe --ModsList

# 4. 创建一个新项目
Qlm.exe --NewProject myapp,1.0.0

# 5. 进入项目，安装依赖并运行
cd myapp
Qlm.exe --InstallPackage
Qlm.exe --ProjectRun
```

---

## 3. 目录结构

QLM 约定以下相对路径（均相对于**当前工作目录**）：

```
<工作目录>/
├── mods/                      # 已安装模块（.js）
│   ├── utils.js
│   └── ...
├── disabled/                  # 被禁用的模块
│   └── old_module.js
├── myapp.toml                 # 项目配置（自动查找第一个 .toml）
└── src/
    └── main.q                 # 项目入口脚本
```

| 常量 | 值 | 说明 |
|------|-----|------|
| `REPO_URL` | `https://gitcode.com/2401_86702825/QuernModules-Fork.git` | 远程模块仓库 |
| `MODS_DIR` | `mods` | 已启用模块目录 |
| `DISABLED_DIR` | `disabled` | 禁用模块目录 |

> **注意**：QLM 的"远程仓库"是固定的，硬编码在 `main.cpp` 的 `REPO_URL` 常量中。要切换仓库需要重新编译。

---

## 4. 命令参考

QLM 的入口是 `main.cpp`，第一个参数为动作名，第二个参数（部分命令需要）为参数。

```
Qlm.exe <action> [arg]
```

> **大小写敏感**：QLM 主程序中的动作名比较是**精确字符串匹配**（`action == "--Install"`）。必须使用**大写开头的双横线形式**（如 `--Install`）。
>
> 已知问题：Rust Launcher（`QuerncLauncher`）在 `arg_logics.rs` 中向 `Qlm.exe` 传递的是**小写**动作名（如 `--install`），与 QLM 期望的大写形式**不匹配**，可能导致 `Unknown action`。详见 [第 7 节](#7-通过-launcher-调用-qlm)。

---

### 4.1 `--Help`

```bash
Qlm.exe --Help
```

打印帮助信息并退出（退出码 0）。也可直接不带参数运行 `Qlm.exe`，会打印帮助但退出码为 1。

---

### 4.2 `--Install`

```bash
Qlm.exe --Install <module_name.js>
```

从远程仓库下载并安装指定模块。

**执行流程**：

1. 检查 `git` 是否可用（Windows 下会尝试探测常见安装路径并注入 `PATH`；若仍不可用会提示是否安装）。
2. 确保 `mods/` 目录存在。
3. `git clone <REPO_URL>` 到临时目录 `mods/.qlm_temp_<module_name>`。
4. 在仓库中查找 `<module_name.js>`：
   - 优先精确匹配 `tempDir/<module_name>`；
   - 失败则**大小写不敏感**地在仓库根目录查找同名文件。
5. 若未找到，打印 `Error: File '...' not found in repository.` 并清理临时目录。
6. 复制到 `mods/<实际文件名>`（覆盖已存在文件）。
7. 删除临时目录。

**示例**：

```bash
Qlm.exe --Install utils.js
Qlm.exe --Install JSONParser.js
```

**成功输出**：

```
Success: Module installed to mods/utils.js
```

---

### 4.3 `--Delete`

```bash
Qlm.exe --Delete <module_name.js>
```

从 `mods/` 中删除指定模块。若精确文件名不存在，会尝试**大小写不敏感**匹配。

**示例**：

```bash
Qlm.exe --Delete utils.js
```

**输出**：

```
Success: Deleted utils.js
```

---

### 4.4 `--Disable` / `--Enable`

**禁用**：将模块从 `mods/` 移动到 `disabled/`，使其不再被编译器加载。

```bash
Qlm.exe --Disable <module_name.js>
```

**启用**：将模块从 `disabled/` 移回 `mods/`。

```bash
Qlm.exe --Enable <module_name.js>
```

两个命令都支持大小写不敏感的文件名匹配；若目标位置已存在同名文件，会先删除再移动。

**示例**：

```bash
Qlm.exe --Disable legacy.js
Qlm.exe --Enable  legacy.js
```

---

### 4.5 `--WebList`

```bash
Qlm.exe --WebList
```

克隆远程仓库（`--depth 1`）到系统临时目录，**递归**列出所有 `.js` 文件，然后删除临时目录。

**输出示例**：

```
Fetching module list...
Available Modules (.js):
  - utils.js
  - lib/JSONParser.js
  - lib/network/http.js
```

> 路径为相对于仓库根目录的相对路径。

---

### 4.6 `--ModsList`

```bash
Qlm.exe --ModsList
```

列出本地 `mods/` 目录中所有 `.js` 文件（递归）。

**输出示例**：

```
Installed Modules:
  - utils.js
  - lib/helpers.js
```

若 `mods/` 不存在，打印 `Mods folder does not exist.` 并正常退出（退出码 0）。

---

### 4.7 `--WebSearch` / `--ModsSearch`

**远程搜索**：克隆远程仓库并**按文件名子串**（大小写不敏感）过滤 `.js` 文件。

```bash
Qlm.exe --WebSearch <keyword>
```

**本地搜索**：在本地 `mods/` 中按文件名子串过滤。

```bash
Qlm.exe --ModsSearch <keyword>
```

**示例**：

```bash
Qlm.exe --WebSearch json
# Search Results:
#   - lib/JSONParser.js
#   - json_utils.js
```

> 搜索**仅匹配文件名**，不搜索文件内容。

---

### 4.8 `--NewProject`

```bash
Qlm.exe --NewProject <name>,<version>
# 或
Qlm.exe --NewProject <name> <version>
```

创建一个新项目目录，包含入口脚本和配置文件。

**参数形式**：

| 形式 | 示例 |
|------|------|
| 逗号分隔 | `--NewProject myapp,1.0.0` |
| 空格分隔 | `--NewProject myapp 1.0.0` |

参数两端的引号（`"` 或 `'`）会被自动去除。

**生成内容**：

```
myapp/
├── src/
│   └── main.q          # "Hello World" 模板
└── myapp.toml          # 项目配置
```

**`src/main.q` 内容**：

```quern
Function "Main"() {
    Console.Info("Hello World!");
}
```

**`myapp.toml` 内容**：

```toml
[edition]
name = myapp
ver = 1.0.0

[packages]
# New Format Example:
# need = {
#   ModuleName.js ver = 2023/10/01,12/00/00 sha256 = abc123...
# }
need = {

}
```

**约束**：

- 若目标目录已存在，打印 `Error: Directory already exists.` 并返回失败。
- 若名称或版本为空，返回失败。

---

### 4.9 `--InstallPackage`

```bash
Qlm.exe --InstallPackage
```

在当前目录查找**第一个** `.toml` 文件（按目录遍历顺序），解析其 `[packages]` 段中的 `need = { ... }` 块，逐个安装声明的模块。

**解析规则**（来自 `project_manager.cpp`）：

- 用正则 `need\s*=\s*\{([^}]*)\}` 提取花括号内的内容；
- 逐行解析，跳过空行和包含 `#` 的行；
- 每行第一个空白分隔词为**模块名**；
- 可选 `ver = <时间>`（直到 `sha256` 之前的内容）；
- 可选 `sha256 = <哈希>`；
- 行尾逗号会被去除。

**输出示例**：

```
Reading config: myapp.toml
Found 2 package(s):
Installing: utils.js @ 2023/10/01,12:00:00
Success: Module installed to mods/utils.js
Installing: http.js
Success: Module installed to mods/http.js
```

**没有依赖时**：

```
No packages found.
```

（正常退出，退出码 0）

**找不到 `.toml` 时**：

```
Error: No .toml file found.
```

（退出码 1）

---

### 4.10 `--ProjectRun`

```bash
Qlm.exe --ProjectRun
```

一站式"检查依赖 → 补全 → 运行"。

**执行流程**：

1. 在当前目录查找第一个 `.toml`。
2. 解析 `need = { ... }` 中的依赖列表（与 `--InstallPackage` 相同逻辑）。
3. 扫描 `mods/` 中已安装的 `.js` 文件。
4. 对每个依赖判断是否满足：比较依赖名与已安装文件名（**双向子串匹配**、大小写不敏感）。
   - 例如依赖 `utils.js` 与已安装 `my_utils.js` 视为满足。
5. 对缺失的依赖逐个调用安装流程（支持 `ver` / `sha256`）。
6. 检查当前目录是否存在 `Quernc.exe`；不存在则报错。
7. 检查 `src/main.q` 是否存在；不存在则报错。
8. 执行：

   ```
   Quernc.exe --Run src\main.q
   ```

   并实时转发其标准输出。

**输出示例**：

```
[Info] Dependencies satisfied.

[Running] Quernc.exe --Run src\main.q
----------------------------------------
...（Quernc / Qvm 的输出）...
----------------------------------------
```

**注意**：

- `runProject` 中的编译器路径硬编码为 `Quernc.exe`（Windows 风格），且脚本路径使用 `src\\main.q`（反斜杠）。在非 Windows 平台或 `Quernc` 无 `.exe` 后缀时可能需要手动运行。
- 依赖满足判定使用**子串匹配**，可能误判（例如依赖 `a.js` 与已安装 `data.js` 会匹配成功）。生产项目建议依赖名具有足够区分度。

---

## 5. 依赖声明格式（TOML）

QLM 在 `.toml` 中读取 `[packages]` 段的 `need = { ... }` 块。

### 5.1 最小格式

```toml
[packages]
need = {

}
```

### 5.2 仅模块名

```toml
[packages]
need = {
  utils.js,
  http.js
}
```

> 逗号可选；每行一个模块名即可。

### 5.3 带版本锁定

```toml
[packages]
need = {
  utils.js ver = 2023/10/01,12/00/00,
  http.js  ver = 2024/05/20,09:30:00
}
```

`ver` 的时间格式为 `YYYY/MM/DD,HH/MM/SS`。安装时 QLM 会将 `/` 替换为 `-`、`,` 替换为空格，尝试：

```bash
git checkout "@{2023-10-01 12 00 00}"
```

若 checkout 失败，会打印警告并**使用最新版本**：

```
[Warning] Could not checkout exact date/version. Using latest available.
```

### 5.4 带 SHA256 校验

```toml
[packages]
need = {
  utils.js ver = 2023/10/01,12/00/00 sha256 = 3b1f...e9a2,
  http.js  sha256 = 7c9d...4f01
}
```

安装后 QLM 会计算下载文件的 SHA256 并与声明值（大小写不敏感）比较：

- 匹配：`[Success] Integrity verified.`
- 不匹配：`[Security Error] SHA256 mismatch!`，安装中止，临时目录被清理，**不会**写入 `mods/`。

### 5.5 完整示例

```toml
[edition]
name = myapp
ver = 1.2.0

[packages]
need = {
  utils.js       ver = 2023/10/01,12/00/00 sha256 = 3b1f8c2e9a2d4f6b8c0e1a3d5f7b9c1e3a5d7f9b,
  JSONParser.js  ver = 2024/01/15,00:00:00 sha256 = 7c9d1e3f5a7b9c0d2e4f6a8b0c2d4e6f8a0b2c4d
}
```

> **解析限制**：`project_manager.cpp` 中 `verReg` 正则 `ver\s*=\s*([^s]+)` 以字母 `s` 作为终止符，因此若时间/版本字符串中包含字母 `s`（或紧邻 `sha256` 之外的 `s`），解析可能截断。建议 `ver` 值使用纯数字和分隔符（`/`、`,`、`:`、空格）。

---

## 6. 版本锁定与 SHA256 校验

### 6.1 版本锁定的工作方式

`installModuleWithGit(moduleName, versionDate, expectedSha256)` 的流程：

1. 克隆仓库到临时目录。
2. 若 `versionDate` 非空：
   - **删除**临时目录并**重新**完整克隆（确保有完整历史——注释说明 `--depth` 限制会影响按时间回溯）。
   - 将 `versionDate` 中的 `/` 替换为 `-`、`,` 替换为空格。
   - 执行 `git checkout "@{<日期>}"`。
   - 失败时打印警告，继续使用最新版本。
3. 在仓库中查找文件（精确 → 大小写不敏感）。
4. 若提供 SHA256，校验。
5. 复制到 `mods/`。
6. 清理临时目录。

### 6.2 SHA256 的实现

QLM 内置了轻量级 SHA256 实现（`utills.cpp` 中的 `SHA256` 类），通过 `computeSHA256(filename)` 以 8KB 分块读取文件计算摘要。

- 比较时**两侧都转小写**，因此 TOML 中大小写均可。
- 校验失败会**删除临时目录**并返回失败，不覆盖 `mods/` 中已有文件。

### 6.3 安全建议

- 生产项目应**同时声明 `ver` 与 `sha256`**，防止上游仓库内容被篡改或历史被改写。
- 如果只声明 `sha256`，每次都会下载"最新版本"再校验，可能因上游更新导致校验失败。
- 如果只声明 `ver`，无法检测内容篡改。

---

## 7. 通过 Launcher 调用 QLM

Rust Launcher（`QuerncLauncher`）提供子命令封装：

| Launcher 选项 | 等价 QLM 调用 |
|---------------|---------------|
| `--module-install <name>` | `Qlm.exe --install <name>` |
| `--module-delete <name>` | `Qlm.exe --delete <name>` |
| `--module-disable <name>` | `Qlm.exe --disable <name>` |
| `--module-enable <name>` | `Qlm.exe --enable <name>` |
| `--module-install-all` | `Qlm.exe --InstallPackage` |
| `--web-list` | `Qlm.exe --web-list` |
| `--local-list` | `Qlm.exe --mods-list` |
| `--web-search <query>` | `Qlm.exe --web-search <query>` |

---

## 8. 环境要求

| 依赖 | 用途 | 说明 |
|------|------|------|
| **Git** | 所有远程操作（`--Install`、`--WebList`、`--WebSearch`、`--InstallPackage`、`--ProjectRun`） | Windows 下 QLM 会自动探测 `C:\Program Files\Git\cmd`、`C:\Program Files (x86)\Git\cmd`、`%LOCALAPPDATA%\Programs\Git\cmd` 并注入 PATH；若仍缺失会提示是否打开下载页或通过 apt 安装 |
| **Quernc.exe** | `--ProjectRun` | 需与 QLM 在同一工作目录或 PATH 中 |
| **src/main.q** | `--ProjectRun` | 项目入口脚本 |
| **网络** | 远程操作 | 访问 `gitcode.com` |

> QLM 不会自动安装 Quernc 工具链。请确保 `Quernc.exe`、`Qvm.exe` 已就位。

---

## 9. 故障排查

### 9.1 `'git' is not found in your system PATH`

Git 未安装或不在 PATH 中。按提示输入 `y`：

- **Windows**：自动打开 Git 下载页，需手动安装后重启终端。
- **Linux**：QLM 尝试 `sudo apt update && sudo apt install -y git`。

### 9.2 `Failed to clone repository`

常见原因：

- 网络不可达 `gitcode.com`；
- 仓库地址变更（`REPO_URL` 硬编码）；
- 磁盘空间不足。

可手动测试：

```bash
git clone --depth 1 https://gitcode.com/2401_86702825/QuernModules-Fork.git test_clone
```

### 9.3 `Error: File 'xxx.js' not found in repository`

- 确认模块名拼写（可先 `--WebList` 查看）；
- QLM 会在仓库**根目录**做大小写不敏感匹配，但不支持子目录自动定位；若模块位于子目录（如 `lib/utils.js`），需要使用完整相对路径作为参数（取决于 `findFileCaseInsensitive` 仅扫描一层，实际子目录模块可能无法通过 `--Install` 安装）。

### 9.4 `[Security Error] SHA256 mismatch!`

- 声明的哈希与实际文件不符；
- 上游仓库内容已变更；
- 若使用 `ver` 锁定，检查 checkout 是否成功（失败会回退到最新版本，导致哈希不匹配）。

**处理**：确认文件来源可信后，更新 TOML 中的 `sha256`；否则放弃安装并排查上游。

### 9.5 `--ProjectRun` 报 `Quernc.exe not found`

QLM 只检查当前目录。请将 `Quernc.exe` 放入项目根目录，或从包含它的目录运行，或等待后续版本支持 PATH 查找。

### 9.6 `Unknown action: --install`

Launcher 传参大小写与 QLM 不匹配，见 [第 7 节](#7-通过-launcher-调用-qlm)。直接调用 `Qlm.exe --Install`。

---

## 10. 已知限制

| 限制 | 说明 | 规避方式 |
|------|------|----------|
| 远程仓库硬编码 | `REPO_URL` 写死在 `main.cpp` | 修改源码重新编译 |
| 仅识别 `.js` 模块 | `searchJsFiles` 仅匹配 `.js` 扩展名 | — |
| 子目录模块安装 | 文件查找仅在克隆根目录做大小写不敏感匹配 | 使用完整相对路径，或手动复制到仓库根 |
| 动作名大小写敏感 | `--Install` ≠ `--install` | 使用大写形式 |
| Launcher 与 QLM 动作名不一致 | 见第 7 节 | 直接调用 QLM |
| `ver` 解析正则脆弱 | 以字母 `s` 为终止符 | `ver` 值只用数字与分隔符 |
| `--ProjectRun` 依赖判定用子串 | `a.js` 与 `data.js` 会误判满足 | 依赖名保持高区分度 |
| `--ProjectRun` 路径硬编码 | `Quernc.exe`、`src\\main.q` | Windows 环境使用 |
| 项目配置查找只取第一个 `.toml` | 目录中有多个 `.toml` 时行为不确定 | 每个项目目录只放一个 `.toml` |
| 无并发下载 | 依赖逐个安装 | — |
| 无依赖传递解析 | 只解析当前 TOML 的 `need` | 手动在 TOML 中列全 |
| 无版本升级命令 | 无 `--Update` / `--Publish` | ROADMAP 中规划 |

---

*本文档与 QLM v2.0 代码库同步。若实现有变更，以代码为准。*


---

# Quern AOT 编译指南

> 本文档介绍 Quern 的**提前编译（Ahead-of-Time, AOT）** 后端：将 `.qb` 字节码翻译为 C 源码，再通过 clang 编译为原生可执行文件。
>
> **适用组件**：`QuernBuild`（AOT 前端，README 中亦称 `QuernAOT`）、`ganerator.cpp`（C 代码生成器）、`run_complier.cpp`（工具链检测与 clang 调用）、`QuerncLauncher` 的 `--aot-build`、Go 前端的 AutoAOT。

---

## 目录

1. [AOT 是什么](#1-aot-是什么)
2. [架构与数据流](#2-架构与数据流)
3. [快速开始](#3-快速开始)
4. [命令行选项](#4-命令行选项)
5. [生成的 C 代码结构](#5-生成的-c-代码结构)
6. [优化等级选择](#6-优化等级选择)
7. [AutoAOT 机制（现状说明）](#7-autoaot-机制现状说明)
8. [热点检测（设计现状与路线图）](#8-热点检测设计现状与路线图)
9. [AOT 与 VM 的行为差异与限制](#9-aot-与-vm-的行为差异与限制)
10. [故障排查](#10-故障排查)
11. [性能建议](#11-性能建议)