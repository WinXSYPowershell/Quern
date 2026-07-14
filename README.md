
Quern Language (C# Edition) - 快速入门指南

作者: Pwershell 


【简介】
Quern 是一种基于 C# 开发的轻量级脚本语言，旨在提供简单易用的语法
来处理文件、字符串、列表和逻辑控制。它支持通过 JavaScript 扩展功能。

----------------------------------------------------------------
1. 环境依赖
----------------------------------------------------------------
要运行或开发 Quern 脚本，你需要：
- .NET Framework 4.7.2 或更高版本 (用于运行 QI.exe)
- Visual Studio Build Tools (包含 cl.exe 和 rc.exe，仅用于打包)
- PowerShell (Windows 自带，用于解包资源)

----------------------------------------------------------------
2. 目录结构
----------------------------------------------------------------
```
Quern/
│
├── QI.exe            <-- [核心] Quern 解释器，用于运行 .q 脚本
├── QP.exe            <-- [工具] Quern 打包器，用于生成 .exe (假设名为QP)
├── main.cs           <-- 引擎源代码
│
├── mods/             <-- [库] 标准库和扩展脚本目录
│   ├── bmath.js      (数学库)
│   ├── bstring.js    (字符串库)
│   ├── blist.js      (列表库)
│   ├── btime.js      (时间库)
│   ├── json.js       (JSON处理库)
│   ├── bdebug.js     (调试库)
│   └── bfilerequests.js (文件与系统操作)
│
└── Packger_ZIP/      <-- [资源] 打包所需的解释器核心压缩包
    └── QuernInterpreter.zip
```
----------------------------------------------------------------
3. 如何运行脚本 (解释器模式)
----------------------------------------------------------------
使用 QI.exe 来测试和运行你的代码。

命令格式:
    QI.exe --Run <脚本文件名.q>

示例:
    QI.exe --Run test.q

----------------------------------------------------------------
4. 如何打包程序 (发布模式)
----------------------------------------------------------------
使用 QP.exe (QuernPackger) 将脚本和解释器打包成一个独立的 .exe 文件。
*注意：打包需要系统已配置 C++ 编译环境 (cl.exe, rc.exe)。*

命令格式:
    QP.exe --Build <脚本文件名.q>

示例:
    QP.exe --Build my_game.q
    > 输出: my_game.exe

打包后的程序会自动释放解释器到临时目录并运行，无需用户安装 .NET 环境
(前提是打包时包含了所有必要的运行时文件)。

----------------------------------------------------------------
5. 语言语法速查
----------------------------------------------------------------
```
# 定义变量
Set String "Name" = "Quern"
Set Number "Age" = 18
Set "Score" += 10

# 定义列表
List = "Fruits" = ["Apple", "Banana", "Orange"]

Set Numebr "Age" = "18"
If ("Age" == "18") cycle(4) {:
    Print ("成年") > CommandLine
Else:
    Print ("未成年") > CommandLine
}

# 流程控制
If ("18" == "18")  cycle(4) {:
    Print ("成年") > CommandLine
Else:
    Print ("未成年") > CommandLine
}


# 函数定义
Fn "Main" (MainFn) {
    Print ("程序启动") > CommandLine
}
```
----------------------------------------------------------------
6. 标准库 API 参考
----------------------------------------------------------------
在使用以下命令前，请在脚本开头使用 Import 导入对应模块。

// 导入模块示例
```
Import "bmath.js"
Import "bstring.js"
```
[数学库 bmath.js]
Pow <底数> <指数>       -> 计算幂
Sqrt <数值>             -> 开平方
Random <最小值> <最大值> -> 生成随机整数
Sin/Cos/Tan <弧度>      -> 三角函数

[字符串库 bstring.js]
String "变量名" Split "分隔符"   -> 分割字符串为列表
String "变量名" Trim             -> 去除首尾空格
String "变量名" Length           -> 获取长度
String "变量名" SuString <起> <止> -> 截取子串

[列表库 blist.js]
ListSort "列表名" [ASC/DESC]     -> 排序
ListFind "列表名" "值"           -> 查找索引
ListJoin "列表1" "列表2" "新名"   -> 合并列表
ListReverse "列表名"             -> 反转
ListDistinct "列表名"            -> 去重

[JSON库 json.js]
JsonPrint L+列表名               -> 打印列表为JSON格式
JsonWriteVar "变量名" L+列表名   -> 将列表转为JSON字符串存入变量
JsonToList "新列表名" "JSON字符串" -> 解析JSON到列表

[调试库 bdebug.js]
Assert (条件), "错误信息"        -> 断言，失败则终止
DebugLog "信息"                  -> 输出调试日志

[时间库 btime.js]
Time                             -> 打印当前时间
Date                             -> 打印当前日期

----------------------------------------------------------------
7. 高级特性
----------------------------------------------------------------
```
// 包含其他脚本文件
Include "common.q"

// 访问列表元素
// 语法: L+列表名+索引
Print (L+Fruits+0) > CommandLine // 输出 Apple

// 系统交互 (通过 bfilerequests.js)
// 该库允许脚本调用系统命令或检查环境
```