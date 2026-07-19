Import "bFileRequests.js"
Fn "Main"(MainFn) {
    # 1. 使用正斜杠 \ 代替反斜杠 \
    # 这样路径 "logs\access.log" 不会被错误分割
    NewFile "logs\access.log"
    Print ("Created initial log file.") > CommandLine

    # 2. 读取文件
    EditFile "QWERTYUIIOP\n123" > "logs\access.log"
    Print ("Read content into variable.") > CommandLine

    # 3. 覆盖内容
    EditFile "\nQWERTYUIIOP\n123" > "logs\access.log"
    Print ("Appended new log entry.") > CommandLine

    # 4. 复制文件
    CopyFile "logs\access.log" > "backups\access_backup.log"
    Print ("Backup created.") > CommandLine

    # 5. 移动文件
    MoveFile "backups\access_backup.log" > "archive\2023\access_old.log"
    Print ("File moved to archive.") > CommandLine

    # 6. 删除文件
    NewFile "Temp Data" > "temp.tmp"
    DeleteFile "temp.tmp"
    Print ("Temporary file deleted.") > CommandLine
    
    # 7. 变量写入
    Set String "MyData" = "Hello World from Variable"
    EditFile MyData > "output\from_var.txt"
    Print ("Wrote variable content to file.") > CommandLine
}