
# 1. 导入 bInput.js 库
# 确保 bInput.js 文件位于当前脚本目录下的 mods 文件夹中
Import "bInput.js"

Fn "MainFn"(MainFn) {
    # --- 演示 1: 基础字符串输入 ---
    Set String "UserName" = "A"
    Print ("=== 演示 1: 字符串输入 ===") > CommandLine
    # 语法: Input "提示语" = "变量名"
    Input "请输入你的名字:" = "UserName"
    
    # 打印欢迎信息
    Print ("你好, ") > CommandLine
    Print (UserName) > CommandLine
    Print ("!") > CommandLine
    
    Wait (1000) # 暂停 1 秒

    # --- 演示 2: 数字输入 (带自动验证) ---
    Print ("") > CommandLine
    Print ("=== 演示 2: 数字输入 ===") > CommandLine
    # 语法: InputNumber "提示语" = "变量名"
    # 如果输入非数字，JS 端会自动循环提示重新输入
    InputNumber "请输入你的年龄 (整数):" = "UserAge"
    
    Print ("你今年 ") > CommandLine
    Print (UserAge) > CommandLine
    Print (" 岁。") > CommandLine
    
    Wait(1000)

    # --- 演示 3: 简单算术 ---
    Print ("") > CommandLine
    Print ("=== 演示 3: 简单计算 ===") > CommandLine
    InputNumber "请输入第一个数字:" = "NumA"
    InputNumber "请输入第二个数字:" = "NumB"
    
    # Quern 支持简单的表达式计算
    Set "Sum" = NumA + NumB
    Set "Product" = NumA * NumB
    
    Print ("两数之和: ") > CommandLine
    Print (Sum) > CommandLine
    Print ("两数之积: ") > CommandLine
    Print (Product) > CommandLine

    Wait (1500)

    # --- 结束 ---
    Print ("") > CommandLine
    WaitInput # 等待用户按任意键退出
}
