# 1. 导入 bMap 库
Import "bMap.js"

Fn "MainFn"(MainFn) {
    # --- 初始化 ---
    Print ("=== 1. 初始化玩家数据地图 ===") > CommandLine
    MapInit "PlayerStats"

    # --- 设置数据 (Set) ---
    # 语法: MapSet "MapName" "Key" "Value" ...
    Print ("正在设置基础属性...") > CommandLine
    MapSet "PlayerStats" "Name" "Hero" "Level" "10" "Gold" "500" "HP" "100"

    # --- 读取单个值 (Get) ---
    Print ("=== 2. 读取单个属性 ===") > CommandLine
    
    # 获取 Name
    MapGet "PlayerStats" "Name" "TempName"
    Print ("玩家姓名: ") > CommandLine
    Print (TempName) > CommandLine
    
    # 获取 Level (数字类型)
    MapGet "PlayerStats" "Level" "TempLevel"
    Print ("玩家等级: ") > CommandLine
    Print (TempLevel) > CommandLine

    # --- 打印整个地图 (Debug) ---
    Print ("=== 3. 打印完整地图 JSON ===") > CommandLine
    PrintMap "PlayerStats"

    # --- 检查键是否存在 (Has) ---
    Print ("=== 4. 检查键是否存在 ===") > CommandLine
    MapHas "PlayerStats" "Gold"
    # MapHas 会将结果存入全局变量 "MapCheckResult" ("True" 或 "False")
    Print ("是否有金币字段? ") > CommandLine
    Print (MapCheckResult) > CommandLine

    MapHas "PlayerStats" "Mana"
    Print ("是否有魔力字段? ") > CommandLine
    Print (MapCheckResult) > CommandLine

    # --- 修改/追加数据 ---
    Print ("=== 5. 获得战利品 (追加/修改) ===") > CommandLine
    # 增加 MP 字段，并更新 Gold
    MapSet "PlayerStats" "MP" "50" "Gold" "600"
    PrintMap "PlayerStats"

    # --- 合并地图 (Merge) ---
    Print ("=== 6. 合并装备地图 ===") > CommandLine
    MapInit "Equipment"
    MapSet "Equipment" "Weapon" "IronSword" "Armor" "LeatherVest"
    
    Print ("合并前 PlayerStats:") > CommandLine
    PrintMap "PlayerStats"
    
    MapMerge "PlayerStats" "Equipment"
    
    Print ("合并后 PlayerStats:") > CommandLine
    PrintMap "PlayerStats"

    # --- 删除数据 (Del) ---
    Print ("=== 7. 丢弃装备 (删除键) ===") > CommandLine
    MapDel "PlayerStats" "Armor"
    Print ("删除 Armor 后:") > CommandLine
    PrintMap "PlayerStats"
    
    # --- 错误处理演示 ---
    Print ("=== 8. 尝试获取不存在的键 ===") > CommandLine
    MapGet "PlayerStats" "NonExistentKey" "FailVar"
    # 此时 FailVar 不会被赋值，且控制台会输出 Warning
}