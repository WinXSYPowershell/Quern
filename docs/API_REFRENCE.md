# Quern API 参考

 本文档列出 Quern 工具链对外暴露的全部 API，包括：
 源码层语句、字节码指令、生成程序运行时库、命令行工具、Mod 系统 API，以及计划中的宿主 API。

 版本：对应 Quern 重构后的三段式架构（Go 编译器 + Rust VM + C++ AOT）
 文档状态：与当前代码库同步

---

## 目录

1. [分层总览](#1-分层总览)
2. [Quern 源码内置 API](#2-quern-源码内置-api)
3. [字节码指令参考](#3-字节码指令参考)
4. [生成程序运行时库（C）](#4-生成程序运行时库c)
5. [命令行工具](#5-命令行工具)
6. [Mod 系统 API（JavaScript）](#6-mod-系统-apijavascript)
7. [宿主 API（计划中）](#7-宿主-api计划中)
8. [错误代码参考](#8-错误代码参考)
9. [后端兼容性矩阵与已知问题](#9-后端兼容性矩阵与已知问题)

---

## 1. 分层总览

Quern 的API分布在五个层面，理解它们各自的职责是正确使用本文档的前提：

 层级  使用者  形式  典型入口 
------------------------------
 L1 源码语句  脚本作者  `.q` 文件中的关键字语句  `Console.Info`、`Data.Var`、`DataStruct.` 
 L2 字节码指令  编译器  手写字节码  `.qb` 文件中的指令  `crt`、`psh`、`out`、`jmp` 
 L3 运行时库  生成的原生程序  C 函数  `push_stack`、`var_set`、`compare_values` 
 L4 命令行  终端用户  CI  可执行文件参数  `Quernc --Run`、`Qvm --Run`、`QLM --Install` 
 L5 扩展 API  Mod 作者  宿主程序  JavaScript  FFI  `Quern.Reg`、`host_call`（计划中） 

 注意：Quern 没有传统编程语言意义上的标准库函数（如 `println!`、`strlen`）。所有源码层能力都以语句形式提供，由编译器在翻译阶段展开为字节码。

---

## 2. Quern 源码内置 API

以下 API 均可直接书写于 `.q` 源文件的函数体、委托体、循环体和条件体中。

### 2.1 `Console.Info` — 输出

```quern
Console.Info(内容)
```

 参数  类型  说明 
------------------
 `内容`  字符串  要输出的内容，建议使用双引号包裹 

行为：

- 在编译期进行别名替换（见 [`Alias`](#23-alias--别名)）与 `--ConstCalc` 常量求值。
- 在运行期将内容中的 `@name@` 替换为对应栈的栈顶值。
- 输出后自动追加换行（生成 `out` + `otn`）。

示例：

```quern
Console.Info(Hello World!)
Console.Info(User @name@)            运行时插值
```

 多词内容必须加引号：`Console.Info(Hello World)` 在 Go 前端会生成含空格的 `out` 指令，Qvm（Rust）的 tokenizer 按空格切分，会导致 `World` 被当作未知指令而报错。始终使用 `Console.Info(Hello World)`。

---

### 2.2 `Data.Var` — 变量定义

```quern
Data.Var [Private] [IntStringBoolNum] name = value
```

 组成部分  可选  说明 
----------------------
 `Private`  是  标记私有变量（语义标记，暂未在 VM 侧强制） 
 `Int`  `String`  `Bool`  `Num`  是  类型提示；`Int`、`Num` 触发编译期数值表达式求值 
 `name`  否  变量名（也是底层栈名） 
 `value`  否  初始值；可为字面值或 `@other@` 

示例：

```quern
Data.Var age = 18
Data.Var String name = Quern
Data.Var Int result = 10 + 5  2         编译期为 20
Data.Var Private secret = hidden
Data.Var copy = @name@                   复制另一栈的栈顶
```

语义：`Data.Var x = v` 等价于 `crt x` + `psh x v`。变量即栈。

---

### 2.3 `Alias` — 别名

```quern
Alias aliasName = replacement
```

 参数  说明 
------------
 `aliasName`  别名标识符 
 `replacement`  替换文本 

行为：在编译期替换 `Console.Info` 中不带引号的别名引用。

示例：

```quern
Alias greet = Hello, Quern!

Console.Info(greet)           编译后等价于 Console.Info(Hello, Quern!)
```

 别名仅在 `Console.Info(...)` 的无引号实参位置生效；其他上下文中不生效。

---

### 2.4 `DataStruct.List` — 列表定义

```quern
DataStruct.List name = [item1, item2, ...]
```

底层表示：

 栈名  含义 
------------
 `name_len`  列表长度 
 `name_0`, `name_1`, ...  各元素 

示例：

```quern
DataStruct.List fruits = [Apple, Banana, Cherry]

Console.Info(@fruits_len@)       输出 3
Console.Info(@fruits_0@)         输出 Apple
```

  列表内容是编译期的常量数据；所有列表操作（见 2.6）也在翻译阶段完成。运行时不可动态修改列表结构。

---

### 2.5 `DataStruct.Dict` — 字典定义

```quern
DataStruct.Dict name = {key1 value1, key2 value2}
```

底层表示：

 栈名  含义 
------------
 `name_key_key`  键 
 `name_val_key`  值 

示例：

```quern
DataStruct.Dict config = {host localhost, port 8080}

Console.Info(@config_val_host@)    输出 localhost
Console.Info(@config_key_port@)    输出 port
```

---

### 2.6 `DataStruct.List` — 列表操作

所有列表操作均为编译期展开，结果直接反映到字节码中。

#### `DataStruct.ListAdd` — 添加元素（前插）

```quern
DataStruct.ListAdd value  listName
```

#### `DataStruct.ListDelete` — 删除首次出现的元素

```quern
DataStruct.ListDelete value  listName
```

#### `DataStruct.ListEdit` — 按索引替换

```quern
DataStruct.ListEdit listName - index  newValue
```

 参数  类型  说明 
------------------
 `listName`  字符串  目标列表 
 `index`  整数  0-based 索引，越界时输出 `# Error` 注释 
 `newValue`  字符串  新值 

#### `DataStruct.ListFind.Bool` — 存在性查找

```quern
DataStruct.ListFind.Bool listName - target  resultVar
```

结果：`resultVar` 被定义为 `True` 或 `False`。

#### `DataStruct.ListFind.Index` — 索引查找

```quern
DataStruct.ListFind.Index listName - target  indexVar
```

结果：`indexVar` 为 0-based 索引，未找到为 `-1`。

示例：

```quern
DataStruct.List nums = [10, 20, 30]

DataStruct.ListAdd 5  nums                   nums = [5, 10, 20, 30]
DataStruct.ListFind.Bool nums - 20  has20  has20 = True
DataStruct.ListFind.Index nums - 30  idx30 idx30 = 2
DataStruct.ListEdit nums - 0  99             nums[0] = 99
DataStruct.ListDelete 10  nums               删除首个 10
```

---

### 2.7 `DataStruct.Dict` — 字典操作

#### `DataStruct.DictAdd` — 添加修改键值对

```quern
DataStruct.DictAdd key - value  dictName
```

#### `DataStruct.DictDelete` — 删除键

```quern
DataStruct.DictDelete key  dictName
```

#### `DataStruct.DictEdit` — 修改已有键的值

```quern
DataStruct.DictEdit dictName - key  newValue
```

#### `DataStruct.DictFind.Bool` — 按值查找

```quern
DataStruct.DictFind.Bool dictName - targetValue  resultVar
```

结果：`True`  `False`。

#### `DataStruct.DictFind.Key` — 按值反查键

```quern
DataStruct.DictFind.Key dictName - targetValue  keyVar
```

结果：匹配的键；未找到为 `None`。

示例：

```quern
DataStruct.Dict person = {name Alice, age 30}

DataStruct.DictAdd city - Beijing  person
DataStruct.DictFind.Key person - Beijing  foundKey    foundKey = city
DataStruct.DictFind.Bool person - Alice  isAlice       isAlice = True
DataStruct.DictEdit person - age  31
DataStruct.DictDelete city  person
```

---

### 2.8 `Template` — 类成员实例化

```quern
Template ClassName
```

将 `Class ClassName { ... }` 中定义的成员展开到当前位置。

示例：

```quern
Class Logger {
    Console.Info([LOG] Application started)
}

Function Main(Main) {
    Template Logger
    Console.Info(Main continues)
}
```

---

### 2.9 `Import` — 模块导入（占位）

```quern
Import ModuleName
```

  当前实现是占位符：Go 前端仅打印 `[Info] Importing ...`，不执行任何模块加载。

 真正的模块复用请使用：
 - QLM（包管理器）安装 `.js` Mod
 - Mod 文件中的 `Include` 语句（见 [6.3](#63-include--js-模块包含)）

---

### 2.10 控制流

#### `If`  `Else If`  `Else`

```quern
If (left op right) {
    ...
} Else If (left2 op right2) {
    ...
} Else {
    ...
}
```

逻辑运算符：

```quern
If (a = 1 and b = 2) { ... }
If (a = 1 or  b = 2) { ... }
If (not a = 1)       { ... }
```

#### `Loop` — 循环

```quern
Loop(N) {
    ...
}
```

 参数  说明 
------------
 `N`  循环次数；`-1` 表示无限循环（递归实现） 

  必须带 `--ExpandLoop` 标志才能生效（Go 前端实现）。详见第 9 节。

#### `If ... Loop` — 循环语法糖

```quern
If (cond) Loop(N) {
    ...
}
```

条件成立时执行 `N` 次循环体。同样需要 `--ExpandLoop`（`Loop(-1)` 除外）。

#### `Entrust` — 事件委托

```quern
Entrust (left op right) {
    ...
}
```

 参数  说明 
------------
 `left`  `right`  操作数（`@var@` 或字面值） 
 `op`  比较运算符 

执行模型：VM 在主循环中反复检查所有委托，条件成立时执行其函数体。委托按声明顺序执行，每帧全部检查。

示例：

```quern
Data.Var flag = 1

Entrust (@flag@ = 1) {
    Console.Info(Flag is set)
}
```

---

### 2.11 函数与类

#### `Function`

```quern
Function Name() { ... }
Function Main(Main) { ... }     主入口
Function Name() { ... }           引号可省略
```

- `(Main)` 标记主入口，翻译后在文件末尾自动生成 `cal Main`。
- 函数名会被加上 `qb_` 前缀以避免与 C 标准库符号冲突。

#### `Class`

```quern
Class Name {
     成员（任意语句）
}
```

类本身不执行；通过 `Template Name` 展开成员。

---

## 3. 字节码指令参考

Quern 编译器输出的 `.qb` 字节码使用以下指令。手写字节码（或 VmCode 逃生舱）时可直接使用。

 指令  语法  参数  说明 
------------------------
 `crt`  `crt name`  `name`：栈名  创建空栈 
 `psh`  `psh stack value`  `stack`：目标栈；`value`：字面值或 `@var@`  压入值 
 `psh`（算术）  `psh stack op operand`  `op` ∈ `+ -  `；`operand`：数值  栈顶值 `op` 操作数，结果写回栈顶与变量寄存器 
 `pop`  `pop stack`   弹出栈顶 
 `out`  `out text`  `text`：字符串（可含 `@var@`）  输出文本；若 `text` 匹配已知栈名则输出该栈栈顶 
 `otn`  `otn`   输出换行 
 `del`  `del name`   销毁栈并释放内存 
 `fnc`  `fnc name { ... }`   定义函数（可嵌套） 
 `cal`  `cal name`   调用函数 
 `jmp`  `jmp left right op cal target`  `op`：比较运算符  若 `left op right` 成立则调用 `target` 

操作数类型：

 写法  含义 
------------
 `123`、`text`  字面值 
 `name`  取名为 `name` 的栈的栈顶 
 `@name@`  同 `name`（语义等价，意图更明确） 

算术运算示例（字节码层）：

```
crt x
psh x 10          # x = 10
psh x + 5         # x = 15
psh x  2         # x = 30
psh x  3         # x = 10
```

 算术运算的 C 运行时实现在 `psh_arith`（见 [4.3](#43-变量寄存器)），结果格式化为整数或 `%g` 浮点。

---

## 4. 生成程序运行时库（C）

以下函数由 AOT 后端写入生成的 C 源文件，供生成的程序内部使用。它们不直接暴露给 Quern 用户，但理解它们有助于调试生成代码。

### 4.1 栈操作

 函数  签名  说明 
------------------
 `init_stack`  `void init_stack(Stack s)`  初始化容量为 16 的空栈 
 `push_stack`  `void push_stack(Stack s, const char val)`  压入字符串（`strdup` 复制） 
 `pop_stack`  `void pop_stack(Stack s)`  弹出并释放栈顶 
 `free_stack`  `void free_stack(Stack s)`  释放整个栈 
 `print_top`  `void print_top(Stack s)`  打印栈顶（空栈打印 `(empty)`） 

### 4.2 栈注册表

 函数  签名  说明 
------------------
 `register_stack`  `void register_stack(const char name, Stack s)`  注册栈名到全局表（上限 256） 
 `find_stack`  `Stack find_stack(const char name)`  按名查找栈，未找到返回 `NULL` 
 `get_top`  `const char get_top(Stack s)`  取栈顶，空栈返回 `` 
 `get_top_by_name`  `const char get_top_by_name(const char name)`  按名取栈顶，未找到返回 `` 

### 4.3 变量寄存器

变量寄存器是独立于物理栈的数值存储，用于支持 `psh stack op operand` 的累积运算。

 函数  签名  说明 
------------------
 `var_get`  `const char var_get(const char name)`  读取寄存器值，未设置返回 `NULL` 
 `var_set`  `void var_set(const char name, const char val)`  写入寄存器，并同步物理栈栈顶 
 `out_varval`  `void out_varval(const char name)`  输出寄存器值（回退到间接解析） 
 `psh_arith`  `void psh_arith(const char name, const char op, const char operand)`  执行算术并写回寄存器 

### 4.4 比较

 函数  签名  说明 
------------------
 `compare_values`  `int compare_values(const char lv, const char rv, const char op)`  通用比较；自动区分数值字符串 
 `compare_stacks`  `int compare_stacks(Stack l, Stack r, const char op)`  比较两个栈的栈顶（旧接口） 

`compare_values` 支持的操作符：`=`、`!=`、``、``、`=`、`=`。

 数值比较使用 `1e-9` 容差；字符串比较使用 `strcmp`。

---

## 5. 命令行工具

### 5.1 `Quernc` — Go 编译器前端

将 `.q` 源码翻译为 `.qb` 字节码，并调用 Qvm 执行。

```
Quernc --Run file.q [选项]
```

 选项  说明 
------------
 `--Run file`  必需。指定源文件 
 `--UnuseDeadCodeEli`  启用死代码消除（DCE） 
 `--LgnWarning`  忽略未使用代码告警 
 `--ForceRep`  严格模式：存在告警则拒绝编译 
 `--ExpandLoop`  启用静态循环展开（独立 `Loop` 语句必须加此项） 
 `--ConstCalc`  全局启用编译期常量表达式求值 
 `--AutoAOTMax N`  AutoAOT 函数调用阈值（默认 1000） 
 `--AutoAOTPar params`  传给 QuernBuild 的额外参数（空格分隔） 
 `--AutoAOTClose`  关闭 AutoAOT 

示例：

```bash
Quernc --Run hello.q --ExpandLoop --UnuseDeadCodeEli
Quernc --Run app.q --AutoAOTMax 500 --AutoAOTPar --ClangOFAST --CVerbose
```

---

### 5.2 `Qvm` — Rust 虚拟机

直接解释执行 `.qb` 字节码。

```
Qvm --Run file.qb [选项]
```

 选项  说明 
------------
 `--Run file`  必需。指定字节码文件 
 `--Check`  仅做语法检查，不执行 
 `--Verbose`  详细错误报告（多行格式，带指针） 

示例：

```bash
Qvm --Run cachebytecodehello.qb
Qvm --Check --Run app.qb
Qvm --Verbose --Run broken.qb
```

---

### 5.3 `QuernBuild`  `QuernAOT` — C++ AOT 后端

将 `.qb` 字节码翻译为 C 源码，调用 clang 编译为原生可执行文件。

```
QuernBuild [选项] file.qb
```

 选项  说明  对应 clang 标志 
-----------------------------
 `--ClangOSize`  体积优化  `-Os` 
 `--ClangOSizeBest`  极致体积  `-Oz` 
 `--ClangODebug`  调试优化  `-Og` 
 `--ClangOFAST`  性能优先  `-Ofast` 
 `--NotO`  无优化  `-O0` 
 `--CVerbose`  详细编译输出  `-v` 
 `--ForceWarn`  警告视为错误  `-Werror` 
 `--NoWarn`  抑制警告  `-w` 
 `--VMVerbose`  VM 详细错误格式  — 

示例：

```bash
QuernBuild --ClangOFAST cachebytecodeapp.qb
QuernBuild --ClangOSize --NoWarn cachebytecodesmall.qb
```

---

### 5.4 `QLM` — 包管理器

管理 Quern 模块（`.js` Mod 文件）。

```
qlm [选项]
```

 选项  参数  说明 
------------------
 `--Help`  —  显示帮助 
 `--Install`  `module.js`  从远程仓库安装模块 
 `--Delete`  `module.js`  删除模块 
 `--Disable`  `module.js`  移动到 `disabled` 
 `--Enable`  `module.js`  从 `disabled` 恢复 
 `--WebList`  —  列出远程仓库所有模块 
 `--ModsList`  —  列出本地已安装模块 
 `--WebSearch`  `keyword`  在远程仓库按文件名搜索 
 `--ModsSearch`  `keyword`  在本地按文件名搜索 
 `--InstallPackage`  —  读取当前目录 `.toml`，安装所有依赖 
 `--ProjectRun`  —  检查依赖 → 安装缺失 → 运行 `srcmain.q` 
 `--NewProject`  `name,ver`  创建新项目（`srcmain.q` + `name.toml`） 

依赖声明格式（`.toml` 中的 `[packages]` 段）：

```toml
[packages]
need = {
  ModuleName.js ver = 20231001,120000 sha256 = abc123...
}
```

 字段  说明 
------------
 `ver`  可选。格式 `YYYYMMDD,HHMMSS`，按该时间点 checkout 
 `sha256`  可选。下载后校验文件完整性 

---

### 5.5 `QuerncLauncher` — Rust 启动器

统一入口，封装 Quernc  Qvm  QLM 的调用。

```
QuerncLauncher [选项]
```

编译与运行模式：

 选项  参数  说明 
------------------
 `--run`  `script.q`  编译并运行 
 `--vm-verbose`  `script.q`  编译 + 详细模式 Qvm 
 `--vm-check`  `script.q`  编译 + Qvm 语法检查 
 `--qvm-run`  `script.qb`  直接运行字节码 
 `--aot-build`  `script.q`  两步 AOT：Quernc → QuernBuild 

模块管理：

 选项  参数  说明 
------------------
 `--module-install`  `name`  调用 `Qlm.exe --install` 
 `--module-delete`  `name`  调用 `Qlm.exe --delete` 
 `--module-disable`  `name`  调用 `Qlm.exe --disable` 
 `--module-enable`  `name`  调用 `Qlm.exe --enable` 
 `--module-install-all`  —  调用 `Qlm.exe --InstallPackage` 
 `--web-list`  —  列出远程模块 
 `--local-list`  —  列出本地模块 
 `--web-search`  `query`  远程搜索 

AOT 优化等级（配合 `--aot-build` 或 `--run`）：

 选项  对应标志 
----------------
 `--not-o`  `--NotO` 
 `--ofast`  `--ClangOFAST` 
 `--osize`  `--ClangOSize` 
 `--osize-bast`  `--ClangOSizeBest` 
 `--aot-c-verbose`  `--CVerbose` 
 `--aot-force-warn`  `--ForceWarn` 
 `--aot-no-warn`  `--NoWarn` 

日志与杂项：

 选项  说明 
------------
 `--logs`  启用文件日志（保存到 `.LogsYYYYMMDD`） 
 `--no-print-trace`  不打印 Trace ID 
 `--logo`  打印 Logo 并退出 

---

### 5.6 `Updater` — 工具链更新器

```
updater --update
```

 选项  说明 
------------
 `--update`  从 GitCode 拉取最新 Tag 并下载解压 

---

## 6. Mod 系统 API（JavaScript）

Quern 的 Mod 以 `.js` 文件形式存放于 `mods` 目录，由 `Quernc`（Go 前端）启动时通过 `goja` 引擎加载。

### 6.1 `Quern.Log`

```javascript
Quern.Log(msg)
```

向控制台输出 Mod 日志（前缀 `[Mod Log]`）。

示例：

```javascript
Quern.Log(Custom syntax module loaded);
```

---

### 6.2 `Quern.Reg` — 注册自定义语法

```javascript
Quern.Reg(keyword, handler)
```

 参数  类型  说明 
------------------
 `keyword`  `string`  触发关键字（区分大小写） 
 `handler`  `function(input string) string`  接收原始行，返回字节码文本 

行为：当 Quern 源码中出现以 `keyword` 开头的未知语句时，翻译器将其包装为 `CustomNode`，并调用对应 handler，将返回值按行拆分为字节码指令插入。

示例：

```javascript
 modshello.js
Quern.Reg(Greet, function(line) {
     line 形如 Greet World
    const name = line.split( )[1].replace(g, );
    return `out Hello, ${name}!notn`;
});

Quern.Log(Greet syntax registered);
```

对应的 Quern 源码：

```quern
Function Main(Main) {
    Greet World
}
```

生成字节码：

```
out Hello, World!
otn
```

---

### 6.3 `Include` — JS 模块包含

```javascript
Include pathtofile.js;
```

在 Mod 文件（`.js`）中使用，由 `ProcessIncludes` 预处理展开。

约束：

- 路径相对于当前 Mod 文件所在目录解析。
- 路径不能逃逸出 `mods` 根目录（安全检查）。
- 循环包含会被检测并跳过（打印警告）。

示例：

```javascript
 modsmain.js
Include libhelpers.js;
Include libsyntax.js;

Quern.Log(All includes loaded);
```

---

## 7. 宿主 API（计划中）

  以下 API 尚未实现，列于 ROADMAP，仅作接口设计参考。

### 7.1 `host_call` — 宿主函数调用（计划）

```quern
host_call Module.Func Args...
```

设计意图：

- 在 `qvm.rs` 中新增 `HostCall` 指令。
- 设计 FFI 接口，允许 RustC++ 宿主注册函数。
- Mod 侧通过 `Quern.Reg` 或专用 API 暴露。

状态：未实现。当前宿主扩展只能通过 Mod（JS 生成字节码）或 VmCode 逃生舱实现。

---

## 8. 错误代码参考

编译器（Go  C++）与虚拟机（Rust）统一使用以下错误码：

 代码  名称  触发场景 
----------------------
 `1000`  `UnknownCommand`  遇到未定义的顶层块内语句 
 `1001`  `SyntaxError`  `MissingArgument`  语法不匹配、缺少参数 
 `1002`  `UnexpectedEnd`  输入意外结束（EOF） 
 `1003`  `ParseError`  通用解析错误 
 `1004`  `MissingBrace`  缺少 `}` 
 `1005`  `UnexpectedBrace`  `BlockParseError`  意外的 `}` 或块解析失败 
 `1006`  `InvalidOperator`  未知比较运算符 
 `1007`  `MissingCalKeyword`  `jmp` 后缺少 `cal` 关键字 

错误输出格式：

普通模式（`Qvm` 默认  `Quernc` 默认）：

```
[Error!]In file.q Detected Name,Code Code at line N.Sourcesource-[HERE!]token
```

详细模式（`Qvm --Verbose`  `Quernc --VMVerbose`）：

```
ErrorName CodeCode
Source~~~~~
source 前段-[HERE]source 后段
   ^
~~~~~~~~~~
```

---

## 9. 后端兼容性矩阵与已知问题

### 9.1 特性支持矩阵

 特性  Go 前端 + Qvm  C++ 前端 + AOT  备注 
---------------------------------------
 `Console.Info`  ✅  ✅  多词内容需加引号 
 `Data.Var`  ✅  ✅  
 `Alias`  ✅    AOT 前端不展开别名 
 `DataStruct.ListDict`  ✅  ✅  编译期展开 
 `DataStruct.` CRUD  ✅  ✅  编译期展开 
 `If`  `Else If`  `Else`  ✅  ✅  
 `Loop(N)` 独立语句   需 `--ExpandLoop`   需 `--ExpandLoop`  不启用时完全跳过 
 `If ... Loop(-1)` 无限循环  ✅  ✅  递归实现 
 `If ... Loop(N)`（N≠-1）   需 `--ExpandLoop`   需 `--ExpandLoop`  
 `Entrust`  ✅  ✅  
 `Function`  `Class`  ✅  ✅  
 `Template`  ✅  ✅  
 `Import`   no-op   no-op  仅打印信息 
 `#VMCODE` 逃生舱  ✅  ✅  
 `psh stack op operand`  ❌  ✅  仅字节码层 

---

## 附录：API 速查表

### 源码语句

 语句  用途 
------------
 `Console.Info(...)`  输出 + 换行 
 `Data.Var [Private] [Type] n = v`  定义变量 
 `Alias a = b`  定义编译期别名 
 `DataStruct.List n = [...]`  定义列表 
 `DataStruct.Dict n = {...}`  定义字典 
 `DataStruct.ListAdd v  n`  列表前插 
 `DataStruct.ListDelete v  n`  列表删除 
 `DataStruct.ListEdit n - i  v`  列表按索引编辑 
 `DataStruct.ListFind.Bool n - t  r`  列表存在性查找 
 `DataStruct.ListFind.Index n - t  r`  列表索引查找 
 `DataStruct.DictAdd k - v  n`  字典添加修改 
 `DataStruct.DictDelete k  n`  字典删除键 
 `DataStruct.DictEdit n - k  v`  字典修改值 
 `DataStruct.DictFind.Bool n - v  r`  字典按值查找 
 `DataStruct.DictFind.Key n - v  r`  字典按值反查键 
 `Template ClassName`  展开类成员 
 `Import Module`  导入（占位） 
 `If (c) { } Else If (c) { } Else { }`  条件 
 `If (c) Loop(n) { }`  条件循环糖 
 `Loop(n) { }`  循环（需 `--ExpandLoop`） 
 `Entrust (c) { }`  事件委托 
 `Function N(Main) { }`  函数  主入口 
 `Class N { }`  类 
 `#VMCODE _BEGIN ... _END`  原始字节码注入 

### 字节码指令

 指令  用途 
------------
 `crt n`  创建栈 
 `psh s v`  `psh s op v`  压栈  算术压栈 
 `pop s`  弹栈 
 `out t`  输出 
 `otn`  换行 
 `del n`  销毁栈 
 `fnc n { }`  定义函数 
 `cal n`  调用函数 
 `jmp l r op cal t`  条件调用 

### Mod JS API

 API  用途 
-----------
 `Quern.Log(msg)`  输出 Mod 日志 
 `Quern.Reg(kw, fn)`  注册自定义语法 
 `Include p`  包含 JS 文件 

---

本文档随 Quern 代码库演进持续更新。如发现 API 与实现不一致，以代码库为准，并欢迎提交 Issue。