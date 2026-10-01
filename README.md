# Quern

![Compiler](https://img.shields.io/badge/Compiler-Go-blue.svg)
![Runtime](https://img.shields.io/badge/Runtime-Rust-blue.svg)
![AOT](https://img.shields.io/badge/AOT-C%2B%2B-blue.svg)
![Module](https://img.shields.io/badge/Module-JavaScript-teal.svg)
![License](https://img.shields.io/badge/License-Apache2.0-green.svg)
#### Quern：专为商业软件打造的极速脚本引擎。源码 → 字节码 → 栈式虚拟机 / AOT 原生编译，毫秒级冷启动，极致轻量。让你的插件系统告别卡顿。

![Logo](./logo/quern-lang.png)

## Quern 是什么?

Quern 是一门面向插件场景的脚本语言，配备完整的编译工具链。近期 Quern 完成了底层彻底重构：从“直接解释源码”的单体运行时，进化为 **「Go 编译器 → 字节码 → Rust 虚拟机 / C++ AOT」** 的三段式架构。现在，Quern 拥有一套清晰的分层工具链 —— 每一层用最适合的语言实现，各司其职。

## 为什么选择 Quern?

*   **三段式工具链**：Go 写编译器前端、Rust 写运行时虚拟机、C++ 写 AOT 编译后端，性能与产能兼顾。
*   **字节码中间表示**：所有源码先翻译为基于栈的字节码（`.qb`），虚拟机与 AOT 后端共享同一套中间表示，天然利于跨平台分发与后续优化（如热点 AOT）。
*   **极致性能**：核心运行时代码量极简，冷启动毫秒级；热点逻辑可经 QuernAOT 直接编译为原生二进制，获得接近 C 的执行速度。
*   **工程化就绪**：内置死代码消除、增量编译缓存、严格编译模式、包管理器与模块系统，满足商业软件的开发流程。

## 架构

```
            Quern 源码 (*.q)
                 │
                 ▼
 ┌────────────────────────────────┐
 │  Quernc —— 编译器前端 (Go)      │
 │  词法/语法分析 → AST → 优化     │
 │  DCE · 告警/严格模式 · 增量缓存  │
 └────────────────────────────────┘
                 │  生成字节码
                 ▼
       字节码 (*.qb)           基于栈的指令集：
       crt / psh / pop / out / otn / del / fnc / cal / jmp
                 │
        ┌────────┴─────────┐
        ▼                  ▼
 ┌──────────────┐   ┌───────────────┐
 │ Qvm (Rust)   │   │ QuernAOT (C++) │
 │ 栈式虚拟机    │   │ 字节码 → C 源码 │
 │ 解释执行字节码 │   │ clang → 原生可执行│
 └──────────────┘   └───────────────┘
```

### 工具链组件

| 组件 | 语言 | 职责 |
|------|------|------|
| **Quernc** | Go | 编译器前端：`.q` → AST → `.qb` 字节码。支持 Mod（JS 扩展）加载、死代码消除（DCE）、未使用代码告警与严格模式，按单元做 MD5 哈希实现增量编译缓存 |
| **Qvm** | Rust | 栈式虚拟机：解析并执行 `.qb` 字节码，支持函数、条件跳转、算术运算与字符串插值；提供 `--Check` 语法检查与 `--Verbose` 详细错误报告 |
| **QuernAOT** | C++ | 提前编译后端：将字节码翻译为 C 源码并调用 clang 编译为原生可执行文件，支持多种优化级别 |
| **QLM** | C++ | 包管理器：`--Install` / `--Delete` / `--Disable` / `--Enable` / `--WebList` / `--WebSearch` / `--NewProject` / `--ProjectRun` |
| **Updater** | C++ | 工具链版本更新器 |

## 特性

*   **混合架构**：Go 编译器 + Rust 运行时 + C++ AOT 后端，核心引擎保证执行效率，JavaScript 模块提供灵活的扩展能力。
*   **优雅而严格的语法**：支持变量定义、函数、条件判断 (`If/Else`)、循环 (`Loop`)、列表/字典、类与事件委托 (`Entrust`) 等。
*   **模块化**：`Import` 加载外部 `.q` 模块，`Include` 包含 `.js` 模块，配合 QLM 包管理器实现代码复用与分发。
*   **增量编译**：源码按单元哈希比对，未变化的单元直接复用缓存字节码，显著加速迭代开发。
*   **安全与品质**：死代码分析与未使用代码告警，`--ForceRep` 严格模式可在存在告警时拒绝编译。
*   **跨平台分发自包含**：CI 自动交叉编译 Windows / Linux / macOS 产物并随 Tag 发布，无需手动配置。

## 快速开始

### 运行脚本

编译项目后，通过这条命令一键完成「翻译 → 运行」：

```bash
Quernc.exe --Run <ScriptName.q>
```

该命令会自动完成：解析源码 → 生成/复用字节码（`cache/bytecode/`）→ 调用 Qvm 执行。

也可以直接让 Qvm 运行既有字节码，或使用 QuernAOT 将字节码编译为原生程序：

```bash
Qvm.exe --Run cache/bytecode/<ScriptName>.qb
QuernAOT.exe cache/bytecode/<ScriptName>.qb --ClangOFAST
```

### 代码示例

"基础 - Hello World"
```quern
Function "Main"(Main){
    Console.Info("Hello World!");
}
```

# 开源
## 本项目基于Apache 2.0协议开源
### Apache 2.0网址：https://www.apache.org/licenses/LICENSE-2.0.txt
## 作者：WinXSYPowershell
### 个人空间：https://space.bilibili.com/3546630315837635
## 感谢您的下载和Star！