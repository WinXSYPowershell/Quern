# Quern-Lang

![Langs1](https://img.shields.io/badge/lang-csharp-blue.svg)
![Langs2](https://img.shields.io/badge/lang-javascript-blue.svg)
![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Logo](./logo/quern-lang.png)


** Quern-Lang 是一个基于 C# 实现的、支持通过 JavaScript 扩展功能的脚本语言引擎。它内置了丰富的库，支持文件操作、数据编码、正则表达式、列表与字典处理等功能，并拥有优雅、严格的语法。 **

## 特性
*   **混合架构**: 核心引擎由 C# 编写，保证了执行效率，同时通过 Jint 引擎无缝集成 JavaScript，允许开发者使用 JS 编写功能强大的模块。
*   **丰富的标准库**: 内置了包括文件操作 (`bFileRequests.js`)、数据编码 (`Crypto.js`)、正则表达式 (`Regex.js`)、列表/字典处理 (`bList.js`, `bMap.js`)、数学计算 (`bMath.js`) 等在内的多种功能库。
*   **灵活的语法**: 支持变量定义、函数、条件判断 (`If/Else`)、循环 (`Cycle`) 以及列表操作等。
*   **模块化**: 支持通过 `Import` 语句加载外部的 `.js` 模块，并通过 `Include` 语句包含其他 `.q` 脚本文件，实现代码复用。
*   **调试支持**: 内置调试模式，并提供 `Assert`、`DebugLog` 等调试指令。

## 快速开始
### 运行脚本
` 编译项目后，通过这行命令运行你的脚本
```bash
QI.exe --Run <ScriptName.q>
```
` 代码示例
` 基础
```quern
Fn "Main" (MainFn){
    Print ("Hello World!") > CommandLine
}
```

` 进阶
```quern
# 这是一行注释

# 定义一个主函数
Fn "Main" (MainFn) {
    # 定义变量
    Set String "name" = "Quern-Lang"
    Set Number "version" = 1

    # 打印信息
    Print ("Hello from") > CommandLine
    Print (name) 
    Print (version)

    # 循环
    If (1==1) cycle(3) {:
        Print ("循环运行！") > CommandLine
    }
}
```



# 语法入门
## 变量与数据
### 定义变量
```quern
Set "VarName" = "Value"
# 自动推断类型

Set String "VarName" = "Value"
# 字符串类型（String），不得少于一个字符

Set Numer "VarName" = Number
# 数字类型（Number），不能填入任何字符，只能填入数字
```
` 复合赋值
` 支持+=（Set "cunter" = cunter + Number），就是Set "cunter" += 1 Number
` -=（Set "cunter" = cunter - Number），就是Set "cunter" -= Number
` *=（Set "cunter" = cunter * Number），就是Set "cunter" * Number
` /=（Set "cunter" /= cunter /= Number），就是Set "cunter" = cunter / Number
` //（Set "cunter" = cunter // Number），就是Set "cunter" = cunter // Number（取整除）
` %=（Set "cunter" = cunter %= Number），就是Set "cunter" = cunter %= Nunmber（取模）
` 示例
```quern
Set Number "cunter" = 1
Print (cunter) > CommandLine
Set "cunter" += 1
Print (cunter) > CommandLine
```


### 列表
` 定义列表: List = "列表名" = ["项目1", "项目2", 变量名]
` 操作列表:
```quern
List = "列表名" Add 索引 "值"
List = "列表名" Delete 索引
List = "列表名" Replace 索引 "新值"
访问列表项: 使用 L+列表名+索引 的格式，例如 L+MyList+0
```


## 函数
### 定义函数
```quern
Fn "FunctionName"(Parameter){
   Code...
}
```
` 参数：主函数：MainFn
` QI.exe解释器仅运行MainFn里的代码和MainFn调用到的函数，其他一律视为“死代码”，不运行。
` 调用函数：FunctionName()


## 流程控制
### 条件判断（If/Else）
```quern
If (Conditions) cycle (NumberOfCycles) {:
    # If conditions is true,run:
} Else if (Conditions) {:
    # If conditions is true,run:
} Else: {:
    # All conditions is false,run:
}
```
### 循环（If加cycle）
```quern
If (1==1) cycle (NumberOfCycles) {:
    Code...
```


## 模块与包含
` 导入模块
` Import "模块名.js"

### 内置库指令
** Quern-Lang 通过 JavaScript 模块提供了丰富的内置指令。 **
`文件操作 (bFileRequests.js)
```quern
ReadFile "文件路径" > "变量名": 读取文件内容到变量。
EditFile "内容或变量" > "文件路径": 将内容写入文件（覆盖）。
NewFile "内容" > "文件路径": 创建新文件并写入内容。
DeleteFile "文件路径": 删除文件。
CopyFile "源路径" > "目标路径": 复制文件。
MoveFile "源路径" > "目标路径": 移动/重命名文件。
```

` 数据编码 (Crypto.js)
```quern
Coding <格式> CharacterEncoding = <编码> (<源>) > (<目标>)
支持的格式: HEX, Base16, Base32, Base64, Base58, Base62, Base85, Base91, Binary, Decimal, Caesar, MD5。
示例: Coding Base64 (Q) > (W) (将变量 Q 的内容进行 Base64 编码后存入变量 W)
```

`正则表达式 (Regex.js)
```quern
RegexString(<源>) Regex "正则表达式" > (<目标变量>): 执行正则匹配，将第一个匹配结果存入目标变量。
```

` 日志与调试 (Log.js, bDebug.js)
```quern
Log Level(<级别>) Content("<内容>") > <动作>
级别: INFO, WARN, ERROR
动作: CommandLine (打印到控制台), Save("文件路径") (保存到文件), Add("文件路径") (追加到文件)
Assert (条件), "失败信息": 断言，条件为假时抛出致命错误。
DebugLog "调试信息": 打印调试信息。
```

` 列表与字典 (bList.js, bMap.js)
列表:
```
ListSort "列表名" [ASC/DESC]: 排序并打印列表。
ListFind "列表名" "目标值": 查找值并打印索引。
ListJoin "列表1" "列表2" "新列表名": 合并两个列表。
ListReverse "列表名": 反转并打印列表。
ListDistinct "列表名": 去重并打印列表。
```

` 字典 (Map):
```quern
MapInit "字典名": 初始化一个空字典。
MapSet "字典名" "键1" "值1" ...: 设置键值对。
PrintMap "字典名" ["键"] > CommandLine: 打印整个字典或指定键的值。
MapDel "字典名" "键": 删除键。
MapHas "字典名" "键": 检查键是否存在，结果存入 MapCheckResult 变量。
MapMerge "目标字典" "源字典": 合并字典。
MapGet "字典名" "键" "结果变量": 获取键的值并存入变量。
```

` 数学运算 (bMath.js)
```quern
Pow <底数> <指数>: 幂运算。
Sqrt <数值>: 平方根。
Abs <数值>: 绝对值。
Ceiling <数值>: 向上取整。
Floor <数值>: 向下取整。
Round <数值>: 四舍五入。
Random <最小值> <最大值>: 生成随机整数。
RandomFloat <最小值> <最大值>: 生成随机浮点数。
Sin/Cos/Tan <弧度>: 三角函数。
```

` 字符串操作 (bString.js)
```quern
String "变量名" Split "分隔符": 分割字符串，结果存入 变量名_List，数量存入 变量名_Count。
String "变量名" SuString <起始索引> <长度>: 截取子字符串，结果存入 变量名_Sub。
String "变量名" Trim: 去除首尾空格。
String "变量名" Length: 获取字符串长度，结果存入 变量名_Len。
```

` 其他
` 输入: Input "提示语" = "变量名", InputNumber "变量名"
` 等待: Wait(毫秒数), WaitInput (按任意键继续)
` 时间: Time (打印当前时间), Date (打印当前日期)
` JSON: JsonPrint L+列表名, JsonWriteVar "变量名" L+列表名

## 扩展开发
### JavaScript模块示例
```javascript
// 注册一个名为 "Hello" 的新指令
QuernAPI.Register("Hello", function(line, tokens) {
    // line 是完整的命令行
    // tokens 是分割后的命令参数数组
    if (tokens.length < 2) {
        QuernAPI.Log("[Hello Error] 用法: Hello <名字>");
        return false;
    }
    var name = tokens[1];
    QuernAPI.Log("Hello, " + name + "!");
    return true; // 返回 true 表示指令被成功处理
});
```
` 在脚本中使用
```quern
Import "hello.js"
Fn "Main" (MainFn){
    Hello "World" # 输出: Hello, World!
}
```
### QuernAPI对象
** 在 JS 模块中，你可以使用 QuernAPI 对象与 Quern 引擎交互： **
```javascript
QuernAPI.Log(msg): 打印日志。
QuernAPI.GetVariable(name): 获取变量的值。
QuernAPI.GetList(name): 获取列表对象。
QuernAPI.Runtime.ApiSetVariable(name, type, value): 设置变量的值。
QuernAPI.Register(commandName, handlerFunction): 注册新指令。
```


# 开源
## 本项目基于MIT协议开源
### MIT网址：https://mit-license.org/
## 作者：WinXSYPowershell
### 个人空间：https://space.bilibili.com/3546630315837635?spm_id_from=333.1007.0.0
## 感谢您的下载和Star！