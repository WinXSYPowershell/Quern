# Quern 项目路线图

> 本文档记录了 Quern 语言项目的开发规划与社区建设方向。

---

## 🎯 一、核心功能（Core）

### 基础功能

- [ ] 添加AOT翻译一条龙（translator.go -> bytecode -> aot.cpp -> c -> exe）怎么添加：

- [ ] 2.给psh添加"+"/"-"/"*"/"/"功能
示例：
```quernvm
crt cunters
psh cunters "1"
pop cunters
psh @cunters@ + "5"
psh @cunters@ - "1"
psh @cunters@ * "2"
psh @cunters@ / "2"
pop cunters
out @cunters@
```
#### 输出
```bash
5
```

- [ ] 3.jmp可以使用变量的功能
示例：
```quernvm
crt cunters
psh cunters "1"

fnc "loop1"{
pop cunters
jmp @cunters@ >= 100
out "Hello Number:@cunters@"
}
```
### 1. 热点检测与 AOT 自动触发（当前优先）

- [ ] 设定阈值（建议 10000 次调用或 1000 次循环迭代）
- [ ] 超过阈值后自动调用 aotbuild.exe 编译该函数
- [ ] 后续调用直接执行编译后的 C 函数，不再解释执行
- [ ] 增加 `--aot-threshold` 命令行参数（让用户可调）

### 3. 宿主 API（Host API）设计

- [ ] 定义宿主调用协议：`host_call <Module>.<Func> <Args...>`
- [ ] 在 qvm.rs 中增加 HostCall 指令
- [ ] 设计一个 FFI 接口，让 Rust/C++ 宿主能注册函数
- [ ] 提供一套标准宿主库（Console、File、Time、Math 等）

### 4. 调试器

- [ ] `--debug` 模式：每条指令执行前打印栈状态
- [ ] 支持断点（`--breakpoint <line>`）
- [ ] 支持单步执行（`--step`）
- [ ] 支持变量查看（`--print-stack`）

---

## 📚 二、文档与教育（Documentation）

### 1. 用户文档（User Documentation）

- [ ] README.md —— 项目介绍、快速开始、安装方式
- [ ] LANGUAGE.md —— 完整的 Quern 语法说明（含示例）
- [ ] API_REFERENCE.md —— 所有内置函数、宿主 API 列表
- [ ] QLM_GUIDE.md —— 包管理器使用指南
- [ ] AOT_GUIDE.md —— 热点检测和 AOT 编译原理

### 2. 开发者文档（Developer Documentation）

- [ ] ARCHITECTURE.md —— Quern 工具链架构图 + 组件说明
- [ ] CONTRIBUTING.md —— 贡献指南
- [ ] BUILD.md —— 如何从源码编译所有组件

### 3. 白皮书（Whitepaper）

- [ ] 介绍 Quern 的设计哲学（为什么要做栈式 VM？为什么要 AOT？）
- [ ] 字节码指令集说明
- [ ] 性能对比数据（和 Python/Lua/JavaScript 对比）
- [ ] 未来发展方向

---

## 💻 三、示例库（Examples）

### 1. 基础示例（Basic）

- [ ] hello_world.q —— Console.Info
- [ ] variables.q —— Data.Var 使用
- [ ] lists.q —— DataStruct.List 使用
- [ ] dicts.q —— DataStruct.Dict 使用
- [ ] if_else.q —— If/Else/ElseIf
- [ ] loops.q —— Loop 和嵌套循环
- [ ] functions.q —— Function 定义和调用
- [ ] entrust.q —— Entrust 事件驱动

### 2. 中级示例（Intermediate）

- [ ] fizzbuzz.q —— FizzBuzz（证明图灵完备）
- [ ] fibonacci.q —— 递归计算斐波那契
- [ ] prime.q —— 质数判断
- [ ] quicksort.q —— 快排实现（用 List）
- [ ] calculator.q —— 简单计算器（支持括号和运算符）

### 3. 高级示例（Advanced）

- [ ] http_client.q —— 用宿主 API 发起 HTTP 请求
- [ ] game_loop.q —— 用 Entrust 实现一个简单游戏循环
- [ ] file_reader.q —— 读取文件并处理
- [ ] chat_client.q —— TCP 客户端

---

## 🧪 四、测试（Testing）

### 1. 单元测试

- [ ] VM 指令测试（每条指令单独测）
- [ ] 编译器测试（每个语法特性单独测）
- [ ] AOT 测试（热点检测 + 编译 + 执行一致性）
- [ ] QLM 测试（安装/删除/启用/禁用）

### 2. 集成测试

- [ ] 完整编译流程测试（.q → .qb → 执行）
- [ ] 缓存机制测试（增量编译）
- [ ] 包管理端到端测试

### 3. 性能测试

- [ ] 解释执行性能（对比 Lua/Python）
- [ ] AOT 编译后性能（对比 C/Rust）
- [ ] 热点检测的准确性和开销

---

## 🛠️ 五、工具链完善（Toolchain）

### 1. QLM 包管理器

- [x] --Install
- [x] --Delete
- [x] --Disable/--Enable
- [x] --WebList
- [x] --ModsList
- [x] --WebSearch
- [x] --NewProject
- [x] --InstallPackage
- [x] --ProjectRun
- [ ] --Publish —— 发布模块到远程仓库
- [ ] --Version —— 显示当前版本
- [ ] --Update —— 更新 QLM 自身

### 2. 远程仓库（Repository）

- [ ] 搭建一个静态 HTTP 服务器存放模块（GitHub Pages / 自建）
- [ ] 定义模块元数据格式（module.json 或 module.toml）
- [ ] 支持版本锁定（ver/sha256 已支持）
- [ ] 支持依赖解析（--InstallPackage 已支持）

### 3. 编辑器支持

- [ ] VS Code 插件（语法高亮）
- [ ] 基础自动补全
- [ ] .q 文件图标

---

## 🌍 六、跨平台与分发

### 1. 跨平台支持

- [ ] Linux 测试（Ubuntu/Debian）
- [ ] macOS 测试
- [ ] 条件编译适配（Windows/Linux/macOS）

### 2. 打包与分发

- [ ] 提供预编译二进制包（.zip / .tar.gz）
- [ ] 提供一键安装脚本（curl ... | bash）
- [ ] 发布到 Homebrew（macOS）
- [ ] 发布到 Scoop（Windows）
- [ ] 发布到 AUR（Arch Linux）

---

## 🔁 七、自举（Bootstrap）

> 这是 Quern 的终极目标——用 Quern 语言写 Quern 编译器。

### 阶段 1：最小编译器（用 Go 写一个"Quern 编译器"的 Quern 版本）

- [ ] 设计 Quern 编译器的架构（词法分析 → 语法分析 → 字节码生成）
- [ ] 用 Quern 语言重写 translator.go 的核心逻辑
- [ ] 让 Quern 编译器能编译一个最小的 Quern 程序（比如 hello_world.q）

### 阶段 2：自举验证

- [ ] 用 Go 编译的 Quern 编译器编译"Quern 编译器（Quern 版）"
- [ ] 用新生成的编译器再次编译自己（验证一致性）
- [ ] 对比两次生成的字节码（应该完全一致）

### 阶段 3：完全自举

- [ ] 移除对 Go 的依赖
- [ ] Quern 编译器完全用 Quern 语言实现
- [ ] 发布 "Quern 编译器 v1.0"（纯 Quern 实现）

---

## 🎨 八、社区与生态

### 1. 社区建设

- [ ] 创建 Discord / Telegram 群组
- [ ] 在 Reddit r/ProgrammingLanguages 发布介绍帖
- [ ] 在 Hacker News 发布 Show HN
- [ ] 撰写中文技术博客（知乎/掘金）

### 2. 生态建设

- [ ] 鼓励社区贡献模块（通过 QLM）
- [ ] 建立模块审核机制（防止恶意代码）
- [ ] 举办 Quern 编程比赛（比如"用 Quern 实现一个最小 Web 服务器"）

---

## 📊 进度概览

| 分类 | 已完成 | 进行中 | 待开始 |
|------|--------|--------|--------|
| 核心功能 | 0 | 1（热点检测与AOT） | 3 |
| 文档与教育 | 0 | 0 | 10 |
| 示例库 | 0 | 0 | 15 |
| 测试 | 0 | 0 | 9 |
| 工具链完善 | 9 | 0 | 5 |
| 跨平台与分发 | 0 | 0 | 6 |
| 自举 | 0 | 0 | 7 |
| 社区与生态 | 0 | 0 | 5 |

---

*最后更新：2026-09-13*
