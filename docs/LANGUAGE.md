# Quern 语言语法参考

> 本文档介绍 Quern 脚本语言的完整语法与使用方式。
> 适用于 `Quernc`（编译器前端）、`Qvm`（虚拟机）与 `QuernBuild`（AOT 编译后端）组成的工具链。

---

## 目录

1. [概述](#1-概述)
2. [注释](#2-注释)
3. [程序结构](#3-程序结构)
   - 3.1 [导入模块](#31-导入模块)
   - 3.2 [函数](#32-函数)
   - 3.3 [类](#33-类)
   - 3.4 [委托（Entrust）](#34-委托entrust)
4. [语句](#4-语句)
   - 4.1 [变量定义](#41-变量定义)
   - 4.2 [控制台输出](#42-控制台输出)
   - 4.3 [别名](#43-别名)
   - 4.4 [列表](#44-列表)
   - 4.5 [字典](#45-字典)
   - 4.6 [列表操作](#46-列表操作)
   - 4.7 [字典操作](#47-字典操作)
   - 4.8 [循环](#48-循环)
   - 4.9 [条件判断](#49-条件判断)
5. [变量引用 `@name@`](#5-变量引用-name)
6. [VmCode 逃生舱](#6-vmcode-逃生舱)
7. [字节码指令参考](#7-字节码指令参考)
8. [完整示例](#8-完整示例)

---

## 1. 概述

Quern 是**栈式**脚本语言。每个变量本质上是一个“栈”，变量定义就是向栈中压入初始值，变量读取就是读取栈顶。这与传统的基于寄存器的语言不同，理解这一点对掌握 Quern 至关重要。

Quern 的执行模型有两种：

- **解释执行**：`Qvm` 直接解释执行字节码（`.qb`）。
- **AOT 编译**：`QuernBuild` 将字节码翻译为 C 源码，再由 clang 编译为原生可执行文件。

---

## 2. 注释

Quern 使用一对 `//` 作为块注释定界符，支持单行、多行和内联注释。

```quern
// 这是单行注释 //

//
这是多行注释
可以跨越多行
//

Function "Main"(Main) {
    Console.Info(// 内联注释 //"Hello World!");
}
```

如果文件中出现一个 `//` 之后在文件结束前没有第二个 `//`，则该 `//` 退化为经典的行注释（到行尾结束）。

字符串字面量内部的 `//` **不会**被当作注释定界符。

---

## 3. 程序结构

Quern 程序由若干**顶层节点**组成：导入、函数、类、委托，以及可选的 VmCode 逃生舱块。

### 3.1 导入模块

```quern
Import "ModuleName"
```

导入外部 `.q` 模块。模块文件需要位于可搜索路径中。

---

### 3.2 函数

```quern
Function "FunctionName"() {
    // 函数体
}
```

**主函数**——程序入口：

```quern
Function "Main"(Main) {
    Console.Info("Hello World!");
}
```

也可以省略引号：

```quern
Function Main() {
    Console.Info("Hello World!");
}
```

函数体中可以使用任意语句（见第 4 节）。

---

### 3.3 类

```quern
Class "ClassName" {
    // 成员定义
}
```

类用于组织可复用的成员，通过 `Template` 语句实例化其内容：

```quern
Template "ClassName"
```

`Template` 会将类中定义的成员“展开”到当前位置。

---

### 3.4 委托（Entrust）

```quern
Entrust (condition) {
    // 条件满足时执行的代码
}
```

委托是一种**事件驱动**结构。虚拟机在主循环中反复检查所有委托的条件，一旦条件成立就执行其函数体。

条件语法为 `left op right`：

```quern
Entrust (@counter@ < 10) {
    Console.Info("Still running");
}
```

委托的执行顺序与声明顺序一致，每帧所有委托都会被检查。

---

## 4. 语句

以下语句可用于函数体、委托体、循环体和条件体中。

### 4.1 变量定义

```quern
Data.Var name = value
```

**带类型：**

```quern
Data.Var Int age = 18
Data.Var String name = "Quern"
Data.Var Bool flag = True
Data.Var Num pi = 3.14159
```

**私有变量：**

```quern
Data.Var Private secret = "hidden"
Data.Var Private Int age = 18
```

变量本质上是栈。定义变量 `x = 5` 等价于：创建一个名为 `x` 的栈，并向其中压入 `5`。

---

### 4.2 控制台输出

```quern
Console.Info("Hello World!")
```

输出内容并在末尾自动换行。内容中可以包含 `@变量名@` 进行字符串插值：

```quern
Data.Var name = "Quern"
Console.Info("Hello, @name@!")
```

输出：

```
Hello, Quern!
```

---

### 4.3 别名

```quern
Alias "aliasName" = "value"
```

定义别名后，在 `Console.Info` 中写 `Console.Info(aliasName)`（注意**不加引号**）会用别名值替换。

```quern
Alias "greeting" = "Hello"
Console.Info(greeting)       // 输出 Hello
```

---

### 4.4 列表

```quern
DataStruct.List "myList" = [1, 2, 3, 4, 5]
```

列表内部实现为一组独立的栈：`myList_len` 存储长度，`myList_0`、`myList_1` … 存储各个元素。

```quern
DataStruct.List "names" = ["Alice", "Bob", "Charlie"]
```

**单元素列表：**

```quern
DataStruct.List "single" = ["only"]
```

**获取列表长度：**

```quern
Console.Info("@myList_len@")    // 输出列表长度
```

---

### 4.5 字典

```quern
DataStruct.Dict "myDict" = {"key1": "value1", "key2": "value2"}
```

字典内部同样以独立栈存储：`myDict_key_<key>` 和 `myDict_val_<key>`。

```quern
DataStruct.Dict "person" = {"name": "Alice", "age": "30"}
```

---

### 4.6 列表操作

所有列表操作都是**编译期**执行的——操作结果在翻译阶段就反映到字节码中。

**添加元素（前插）：**

```quern
DataStruct.ListAdd "value" > "myList"
```

**删除元素（首次出现）：**

```quern
DataStruct.ListDelete "value" > "myList"
```

**按索引编辑：**

```quern
DataStruct.ListEdit "myList" - 2 > "newValue"
```

**查找元素是否存在：**

```quern
DataStruct.ListFind.Bool "myList" - "target" > "resultVar"
```

结果 `resultVar` 为 `True` 或 `False`。

**查找元素索引：**

```quern
DataStruct.ListFind.Index "myList" - "target" > "indexVar"
```

结果 `indexVar` 为索引值（未找到为 `-1`）。

---

### 4.7 字典操作

**添加/修改键值对：**

```quern
DataStruct.DictAdd "key" - "value" > "myDict"
```

**删除键：**

```quern
DataStruct.DictDelete "key" > "myDict"
```

**编辑值：**

```quern
DataStruct.DictEdit "myDict" - "key" > "newValue"
```

**查找值是否存在：**

```quern
DataStruct.DictFind.Bool "myDict" - "targetValue" > "resultVar"
```

**根据值查找键：**

```quern
DataStruct.DictFind.Key "myDict" - "targetValue" > "keyVar"
```

未找到时 `keyVar` 为 `None`。

---

### 4.8 循环

```quern
Loop(5) {
    Console.Info("Hello")
}
```

循环体执行 5 次。

**无限循环：**

```quern
Loop(-1) {
    // 永远执行（实际实现为递归调用）
}
```

> 注意：独立 `Loop(N) { }` 语句的展开需要在编译时加上 `--ExpandLoop` 标志（静态展开）。在某些实现路径下，若未启用该标志，独立循环可能被跳过。推荐使用 **If 循环语法糖**（见下）以获得稳定的循环行为。

---

### 4.9 条件判断

**基本形式：**

```quern
If (a = 1) {
    Console.Info("a is 1")
}
```

**逻辑运算符**（`and`、`or`、`not`）：

```quern
If (a = 1 and b = 2) {
    Console.Info("both true")
}

If (a = 1 or b = 2) {
    Console.Info("at least one true")
}

If (not a = 1) {
    Console.Info("a is not 1")
}
```

**比较运算符：**

| 运算符 | 含义 |
|--------|------|
| `=` | 相等 |
| `!=` | 不等 |
| `<` | 小于 |
| `>` | 大于 |
| `=<` | 小于等于（Rust VM 与 Go 编译器使用此写法；AOT 后端接受 `<=`） |
| `>=` | 大于等于 |

**Else / Else If：**

```quern
If (a = 1) {
    Console.Info("one")
} Else If (a = 2) {
    Console.Info("two")
} Else {
    Console.Info("other")
}
```

**循环语法糖**——条件成立时循环执行：

```quern
If (a < 10) Loop(5) {
    Console.Info("repeat 5 times when a < 10")
}
```

**Else 循环语法糖**：

```quern
If (a = 1) {
    Console.Info("one")
} Else Loop(3) {
    Console.Info("else repeated 3 times")
}
```

> 条件表达式中的字符串值不能包含空格。例如 `If (name = "hello world")` 会被解析失败，应写成 `If (@name@ = "hello")` 或将比较值放在变量中。

---

## 5. 变量引用 `@name@`

`@name@` 是 Quern 中读取“栈顶值”的核心语法，可用于多种上下文：

### 5.1 字符串插值

```quern
Data.Var name = "Quern"
Console.Info("Hello, @name@!")
```

### 5.2 压入变量值

```quern
Data.Var a = 1
Data.Var b = @a@
```

`psh b @a@` 会把 `a` 栈顶的值压入 `b` 栈。

### 5.3 条件比较

```quern
Data.Var counter = 0
If (@counter@ < 10) {
    Console.Info("less than 10")
}
```

> 在条件表达式中，始终推荐使用 `@name@` 形式引用栈值。直接写 `name` 在不同后端下可能被解释为栈名或字面字符串，行为不一致。

### 5.4 输出变量引用

```quern
Console.Info("@counter@")
```

---

## 6. VmCode 逃生舱

对于翻译器无法处理、或需要手写精确字节码的场景，可以使用 VmCode 逃生舱块：

```quern
#VMCODE _BEGIN
crt myStack
psh myStack "hello"
out "hello"
#VMCODE _END
```

块内的每一行都会**原样**写入输出的字节码，不经过任何翻译、别名替换或表达式求值。标记大小写不敏感。

逃生舱块可以出现在顶层，也可以出现在任何函数体、循环体、条件体中。

---

## 7. 字节码指令参考

Quern 编译器输出的字节码使用以下指令：

| 指令 | 语法 | 说明 |
|------|------|------|
| `crt` | `crt name` | 创建名为 `name` 的栈 |
| `psh` | `psh stack value` | 将 `value` 压入 `stack` |
| `psh`（算术） | `psh stack op operand` | `stack` 栈顶值与 `operand` 做运算（`+` `-` `*` `/`），结果写回栈顶和变量寄存器 |
| `pop` | `pop stack` | 弹出 `stack` 栈顶 |
| `out` | `out text` | 输出文本（自动处理栈名和字符串插值） |
| `otn` | `otn` | 输出换行 |
| `del` | `del name` | 删除名为 `name` 的栈 |
| `fnc` | `fnc name { ... }` | 定义函数 |
| `cal` | `cal name` | 调用函数 `name` |
| `jmp` | `jmp left right op cal target` | 若 `left op right` 成立则调用 `target` |

**操作数类型：**

- **字面值**：`1`、`"text"`
- **栈名**：`name` —— 取该栈栈顶
- **变量引用**：`@name@` —— 取该栈栈顶（语义与栈名相同，但表达意图更明确）

**算术运算示例：**

```quern
Data.Var x = 10
Data.Var x + 5          // x 现在为 15
Data.Var x * 2          // x 现在为 30
Data.Var x - 3          // x 现在为 27
Data.Var x / 9          // x 现在为 3
```

**比较运算示例：**

```quern
jmp @x@ 10 > cal BigNumber
```

---

## 8. 完整示例

### 示例 1：Hello World

```quern
Function "Main"(Main) {
    Console.Info("Hello World!");
}
```

### 示例 2：变量与字符串插值

```quern
Function "Main"(Main) {
    Data.Var String name = "Quern"
    Data.Var Int version = 2
    Console.Info("Welcome to @name@ v@version@!");
}
```

### 示例 3：条件判断

```quern
Function "Main"(Main) {
    Data.Var score = 85
    If (@score@ >= 90) {
        Console.Info("A");
    } Else If (@score@ >= 80) {
        Console.Info("B");
    } Else {
        Console.Info("C");
    }
}
```

### 示例 4：循环

```quern
Function "Main"(Main) {
    Data.Var counter = 0
    If (@counter@ < 5) Loop(5) {
        Data.Var counter + 1
        Console.Info("Counter: @counter@");
    }
}
```

### 示例 5：列表

```quern
Function "Main"(Main) {
    DataStruct.List "fruits" = ["Apple", "Banana", "Cherry"]

    DataStruct.ListFind.Bool "fruits" - "Banana" > "hasBanana"
    Console.Info("Has Banana: @hasBanana@");

    DataStruct.ListFind.Index "fruits" - "Cherry" > "cherryIdx"
    Console.Info("Cherry Index: @cherryIdx@");

    DataStruct.ListAdd "Orange" > "fruits"
    Console.Info("Length after add: @fruits_len@");
}
```

### 示例 6：字典

```quern
Function "Main"(Main) {
    DataStruct.Dict "config" = {"host": "localhost", "port": "8080"}

    DataStruct.DictFind.Key "config" - "8080" > "keyOf8080"
    Console.Info("Value 8080 belongs to key: @keyOf8080@");

    DataStruct.DictAdd "debug" - "true" > "config"
    Console.Info("Config keys: @config_key_host@, @config_key_port@, @config_key_debug@");
}
```

### 示例 7：类与委托

```quern
Class "Greeter" {
    Data.Var message = "Hello from class"
    Console.Info("@message@")
}

Entrust (@event_flag@ = 1) {
    Console.Info("Event triggered!")
}

Function "Main"(Main) {
    Template "Greeter"
    Console.Info("Main function done");
}
```

### 示例 8：VmCode 逃生舱

```quern
Function "Main"(Main) {
    #VMCODE _BEGIN
    crt raw_stack
    psh raw_stack "handwritten"
    out "handwritten bytecode pushed"
    #VMCODE _END

    Console.Info("Regular translation continues here");
}
```

---

## 附录：工具链命令速查

| 命令 | 说明 |
|------|------|
| `Quernc --Run script.q` | 编译并运行 |
| `Qvm --Run script.qb` | 直接运行字节码 |
| `Qvm --Check --Run script.qb` | 仅做语法检查 |
| `Qvm --Verbose --Run script.qb` | 详细错误报告 |
| `QuernBuild script.qb --ClangOFAST` | AOT 编译为原生可执行文件 |
| `QuernBuild script.qb --ClangOSize` | AOT 编译并优化体积 |

---

*本文档随 Quern 语言演进持续更新。*