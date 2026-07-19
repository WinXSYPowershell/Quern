Import "bString.js"
Fn "Main" (MainFn) {
    # 1. 初始化测试变量
    Set String "MyCSV" = "Apple,Banana,Cherry,Date"
    Set String "MyText" = "  Hello World  "
    
    # 打印原始 CSV
    Print ("原始 CSV:") > CommandLine
    Print (MyCSV) > CommandLine
    
    # 2. 测试 Split (分割)
    # 语法: String "VarName" Split "Delimiter"
    String "MyCSV" Split ","
    
    Print ("--- 分割结果 ---") > CommandLine
    # 打印分割产生的计数变量
    Print ("分割片段数量:") > CommandLine
    Print (MyCSV_Count) > CommandLine
    
    # 3. 测试 SuString (子字符串)
    # 语法: String "VarName" SuString StartIndex Length
    Set String "SourceStr" = "Hello World Quern"
    String "SourceStr" SuString 0 5
    
    Print ("--- 截取结果 ---" > CommandLine)
    # 结果存储在 "SourceStr_Sub"
    Print ("截取内容:") > CommandLine
    Print (SourceStr_Sub) > CommandLine
    
    # 4. 测试 Trim (去空格)
    # 语法: String "VarName" Trim
    Print ("--- 去空格前 ---") > CommandLine
    # 为了显示空格效果，我们直接打印变量，虽然控制台可能看不出来，但值是对的
    Print (MyText) > CommandLine
    
    String "MyText" Trim
    Print ("--- 去空格后 ---") > CommandLine
    Print (MyText) > CommandLine
    
    # 5. 测试 Length (长度)
    # 语法: String "VarName" Length
    String "MyText" Length
    
    Print ("--- 长度结果 ---") > CommandLine
    Print ("当前文本长度:") > CommandLine
    # 结果存储在 "MyText_Len"
    Print (MyText_Len) > CommandLine
}