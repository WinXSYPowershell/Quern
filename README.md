# Quern-Lang

![Langs1](https://img.shields.io/badge/MainLang-Rust-blue.svg)
![Langs2](https://img.shields.io/badge/MainLang-Go-blue.svg)
![Langs2](https://img.shields.io/badge/ModuleLang-Javascript-teal.svg)
![License](https://img.shields.io/badge/License-Apache2.0-green.svg)
#### Quern：专为商业软件打造的极速脚本引擎。基于 Rust，毫秒级冷启动，4MB 极致轻量。让你的插件系统告别卡顿。

![Logo](./logo/quern-lang.png)

##  为什么选择 Quern?

*   **极致性能**：核心引擎由 Rust 编写，启动速度毫秒级，执行效率远超传统解释型语言。
*   **混合架构**：核心逻辑用高性能 Rust 实现，业务逻辑可通过 Complier提供JavaScript 模块灵活扩展，兼顾性能与开发效率。

** Quern-Lang 是一个基于 Rust 实现的、支持通过 JavaScript 扩展功能的脚本语言引擎。它内置了丰富的库，支持文件操作、数据编码、正则表达式、列表与字典处理等功能，并拥有优雅、严格的语法。 **

## 特性
*   **混合架构**: 核心引擎由 Rust 编写，保证了执行效率。翻译机使用Go编写，Go的JavaScript标准库提供了稳定的可扩展性
*   **灵活的语法**: 支持变量定义、函数、条件判断 (`If/Else`)、循环 (`Loop`) 以及列表操作等。
*   **模块化**: 支持通过 `Import` 语句加载外部的 `.q` 模块，并通过 `Include` 语句包含其他 `.js` mod文件，实现代码复用。

## 快速开始
### 运行脚本
` 编译项目后，通过这行命令运行你的脚本
```bash
Quernc.exe --Run <ScriptName.q>
```
` 代码示例
` 基础
```quern
Fn "Main" (MainFn){
    Print ("Hello World!") > CommandLine
}
```


# 开源
## 本项目基于Apache 2.0协议开源
### Apache 2.0网址：https://www.apache.org/licenses/LICENSE-2.0.txt
## 作者：WinXSYPowershell
### 个人空间：https://space.bilibili.com/3546630315837635?spm_id_from=333.1007.0.0
## 感谢您的下载和Star！