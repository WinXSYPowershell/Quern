Import "bmath.js"

Fn "Main"(MainFn) {
    Print ("=== bMath.js 功能测试 ===") > CommandLine

    # 1. 基础算术与幂运算
    Set "Base" = 2
    Set "Exp" = 10
    Print ("计算 2 的 10 次方:") > CommandLine
    Pow Base Exp

    # 2. 平方根
    Set "Num" = 144
    Print ("计算 144 的平方根:") > CommandLine
    Sqrt Num

    # 3. 三角函数 (注意：输入为弧度)
    # PI/2 约为 1.5708
    Set "Angle" = 1.5708
    Print ("计算 Sin(PI/2):") > CommandLine
    Sin Angle
    
    Print ("计算 Cos(0):") > CommandLine
    Cos 0

    # 4. 取整函数
    Set "FloatNum" = 9.8
    Print ("对 9.8 进行向上取整 (Ceiling):") > CommandLine
    Ceiling FloatNum
    
    Print ("对 9.8 进行向下取整 (Floor):") > CommandLine
    Floor FloatNum
    
    Print ("对 9.8 进行四舍五入 (Round):") > CommandLine
    Round FloatNum

    # 5. 绝对值
    Set "NegNum" = -50
    Print ("计算 -50 的绝对值:") > CommandLine
    Abs NegNum

    # 6. 随机数生成
    Print ("生成 1 到 100 之间的随机整数:") > CommandLine
    Random 1 100

    Print ("生成 0.0 到 1.0 之间的随机浮点数:") > CommandLine
    RandomFloat 0 1

    Print ("=== 测试结束 ===") > CommandLine
}
