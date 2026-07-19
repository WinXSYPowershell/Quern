# Quern Crypto Example Script
# Requires: Crypto.js to be placed in the 'mods' directory relative to this script.

Import "Crypto.js"

Fn "MainFn"(MainFn) {
    # 1. 初始化测试字符串
    Set "MySecret" = "Hello Quern World!"
    
    Print ("Original Text") > CommandLine
    Print (MySecret) > CommandLine
    
    # ------------------------------------------
    # 2. Base64 编码
    # Syntax: Coding <Format> (<Source>) > (<Target>)
    # ------------------------------------------
    Coding Base64 (MySecret) > (EncodedB64)
    
    Print ("--- Base64 Encoding ---") > CommandLine
    Print (EncodedB64) > CommandLine
    
    # ------------------------------------------
    # 3. Base16 (Hex) 编码
    # ------------------------------------------
    Coding Base16 (MySecret) > (EncodedHex)
    
    Print ("--- Hex Encoding ---") > CommandLine
    Print (EncodedHex) > CommandLine
    
    # ------------------------------------------
    # 4. Base58 编码 (常用于比特币地址等)
    # ------------------------------------------
    Coding Base58 (MySecret) > (EncodedB58)
    
    Print ("--- Base58 Encoding ---") > CommandLine
    Print (EncodedB58) > CommandLine
    
    # ------------------------------------------
    # 5. Caesar Cipher (凯撒密码)
    # 默认偏移量为 3，可以通过 Offset=5 修改
    # ------------------------------------------
    Coding Caesar Offset=5 (MySecret) > (CaesarText)
    
    Print ("--- Caesar Cipher (Offset 5) ---") > CommandLine
    Print (CaesarText) > CommandLine
    
    # ------------------------------------------
    # 6. MD5 Hash
    # 注意：JS 实现的 MD5 仅支持纯文本输入
    # ------------------------------------------
    Coding MD5 (MySecret) > (HashVal)
    
    Print ("--- MD5 Hash ---") > CommandLine
    Print (HashVal) > CommandLine
    
    # ------------------------------------------
    # 7. 处理数字变量
    # 先将数字转为字符串，再编码
    # ------------------------------------------
    Set Number "PinCode" = 123456
    
    Coding Base64 (PinCode) > (PinB64)
    
    Print ("--- PIN Code Base64 ---") > CommandLine
    Print (PinB64) > CommandLine
}
