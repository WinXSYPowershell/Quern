# 导入 bdebug 模块
Import "bDebug.js"

Fn "Main"(MainFn) {
    # --- 1. 初始化变量 ---
    Set "Score" = 95
    Set "Name" = "PlayerOne"
    Set "Level" = 5
    
    # --- 2. 使用 DebugLog ---
    DebugLog "Starting Game Initialization..."

    Print ("Current Player: ") > CommandLine
    Print (Name) > CommandLine
    
    Print ("Score: ") > CommandLine
    Print (Score) > CommandLine

    # --- 3. 基本断言 ---
    Assert (Score > 0), "Score must be positive!"
    Assert (Name == "PlayerOne"), "Player name mismatch!"

    DebugLog "Basic assertions passed."

    # --- 4. 高级条件检查 ---
    Assert (Score In 0-100), "Score out of valid range (0-100)!"
    Assert (Level IsNumber), "Level variable is not a number!"
    Assert (Name IsString), "Name variable is not a string!"

    DebugLog "Advanced checks passed."

    # --- 5. 列表操作与断言 ---
    List = "Inventory" = ["Sword", "Shield", "Potion"]
    
    # 我们可以直接在 Assert 中使用 L+ 引用，或者先 Print 出来验证
    
    Print ("First Item: ") > CommandLine
    Print (L+Inventory+0) > CommandLine

    # 断言：列表第一个元素必须是 "Sword"
    Assert (L+Inventory+0 == "Sword"), "First item in inventory should be Sword!"

    DebugLog "All tests completed successfully!"
}