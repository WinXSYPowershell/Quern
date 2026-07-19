# 导入 bList.js 模块
# 引擎会自动在 mods 目录下寻找该文件
Import "bList.js"

Fn "Main"(MainFn) {
    # --- 1. 初始化列表 ---
    # 创建一个包含乱序数字的列表
    List = "Numbers" = [5, 12, 3, 8, 1, 99, 42]
    
    # 创建一个包含字符串的列表
    List = "Fruits" = ["Banana", "Apple", "Cherry", "Date", "Apple"]

    Print ("原始数字列表:") > CommandLine
    # 注意：Quern 原生没有直接打印整个列表的命令，我们依靠 JS 模块的日志或手动打印
    # 这里为了演示，我们假设 JS 模块内部会 Log，或者我们可以逐个打印
    # 但为了展示 bList.js 的功能，我们将直接调用它的命令
    
    # --- 2. 测试 ListSort (排序) ---
    Print ("--- 执行 ListSort Numbers ASC ---") > CommandLine
    # 语法: ListSort <ListName> [ASC|DESC]
    ListSort "Numbers" ASC
    
    Print ("--- 执行 ListSort Fruits DESC ---") > CommandLine
    ListSort "Fruits" DESC

    # --- 3. 测试 ListFind (查找) ---
    Print ("--- 执行 ListFind Fruits Apple ---") > CommandLine
    # 语法: ListFind <ListName> <TargetValue>
    # 结果会打印在控制台 (-1 表示未找到, >=0 表示索引)
    ListFind "Fruits" "Apple"
    
    Print ("--- 执行 ListFind Fruits Grape ---") > CommandLine
    ListFind "Fruits" "Grape"

    # --- 4. 测试 ListDistinct (去重) ---
    Print ("--- 执行 ListDistinct Fruits ---") > CommandLine
    # 语法: ListDistinct <ListName>
    ListDistinct "Fruits"

    # --- 5. 测试 ListReverse (反转) ---
    Print ("--- 执行 ListReverse Numbers ---") > CommandLine
    # 语法: ListReverse <ListName>
    ListReverse "Numbers"

    # --- 6. 测试 ListJoin (合并) ---
    # 先创建第二个列表用于合并
    List = "MoreNumbers" = [100, 200]
    
    Print ("--- 执行 ListJoin Numbers MoreNumbers AllNumbers ---") > CommandLine
    # 语法: ListJoin <List1> <List2> <NewListName>
    # 注意：根据 bList.js 代码，它目前只是 Log 合并后的结果，并没有真正更新到 QuernRuntime 的新列表中
    # 如果需要真正更新，需要修改 JS 代码调用 QuernAPI.Runtime.SetList
    ListJoin "Numbers" "MoreNumbers" "AllNumbers"
}