// bFileRequests.js
// 提供文件读写、新建、删除、移动和复制功能

/**
 * 解析参数，支持 "字符串" 或 变量名
 */
function parseArg(arg) {
    if (!arg) return "";
    // 如果是带引号的字符串，去除引号
    if (arg.startsWith('"') && arg.endsWith('"')) {
        return arg.substring(1, arg.length - 1);
    }
    // 否则尝试作为变量名获取值
    var val = QuernAPI.GetVariable(arg);
    // 如果 GetVariable 返回了不同的值，说明是变量；否则可能是字面量或不存在
    if (val !== arg) {
        return val;
    }
    // 如果没找到变量，且不是引号包裹，可能是一个未定义的路径字面量（虽然规范要求引号，但做容错处理）
    return arg;
}

/**
 * 处理换行符：将 \n 替换为实际换行符
 */
function processNewlines(str) {
    if (typeof str !== 'string') return str;
    return str.replace(/\\n/g, '\n');
}

/**
 * 反转义换行符：将实际换行符替换为 \n 字符串
 */
function escapeNewlines(str) {
    if (typeof str !== 'string') return str;
    return str.replace(/\n/g, '\\n').replace(/\r/g, '');
}

// 1. ReadFile ("文件路径") > (变量名/"字符串")
QuernAPI.Register("ReadFile", function(line, tokens) {
    // 格式: ReadFile "path" > "var"
    // tokens: ["ReadFile", "\"path\"", ">", "\"var\""]
    
    if (tokens.length < 4 || tokens[2] !== '>') {
        QuernAPI.Log("[File Error] Usage: ReadFile \"FilePath\" > \"VarName\"");
        return false;
    }

    var filePath = parseArg(tokens[1]);
    var targetName = parseArg(tokens[3]);

    try {
        if (!File.Exists(filePath)) {
            QuernAPI.Log("[File Error] File not found: " + filePath);
            return false;
        }

        var content = File.ReadAllText(filePath);
        // 将实际换行符转义为 \n 以便存储在变量中
        var escapedContent = escapeNewlines(content);

        // 如果目标是变量名（不带引号的原始token判断，或者通过parseArg逻辑反推）
        // 这里简化逻辑：如果 tokens[3] 是带引号的，我们直接把它当字符串？
        // 不，规范说 > (变量名/"字符串")。通常 > 后面跟的是接收者。
        // 如果是 "StringLiteral"，我们无法“写入”一个字符串字面量到运行时环境，只能写入变量。
        // 假设这里的 "字符串" 指的是变量名的字符串表示，或者用户意图是打印？
        // 根据常规脚本逻辑，> 通常用于赋值给变量。
        
        // 检查 tokens[3] 是否是变量名（即没有引号，或者即使有引号我们也将其视为变量名键）
        // 为了安全，我们统一将其作为变量名处理。
        
        QuernAPI.Runtime.ApiSetVariable(targetName, "String", escapedContent);
        return true;
    } catch (e) {
        QuernAPI.Log("[File Error] Read failed: " + e.message);
        return false;
    }
});

// 2. EditFile (变量名/"字符串") > ("文件路径")
QuernAPI.Register("EditFile", function(line, tokens) {
    // 格式: EditFile "content" > "path"  或者 EditFile VarName > "path"
    
    if (tokens.length < 4 || tokens[2] !== '>') {
        QuernAPI.Log("[File Error] Usage: EditFile \"ContentOrVar\" > \"FilePath\"");
        return false;
    }

    var sourceRaw = tokens[1];
    var filePath = parseArg(tokens[3]);

    var content = "";
    
    // 判断源是变量还是直接字符串
    if (sourceRaw.startsWith('"') && sourceRaw.endsWith('"')) {
        content = parseArg(sourceRaw);
    } else {
        content = QuernAPI.GetVariable(sourceRaw);
        if (content === sourceRaw) {
             // 如果没找到变量，且不是引号包裹，可能出错
             QuernAPI.Log("[File Warning] Variable '" + sourceRaw + "' not found or invalid.");
             // 依然尝试使用它作为内容，以防万一
             content = sourceRaw;
        }
    }

    // 将 \n 替换为实际换行符
    var finalContent = processNewlines(content);

    try {
        // 确保目录存在
        var dir = Path.GetDirectoryName(filePath);
        if (dir && !Directory.Exists(dir)) {
            Directory.CreateDirectory(dir);
        }
        
        File.WriteAllText(filePath, finalContent);
        return true;
    } catch (e) {
        QuernAPI.Log("[File Error] Write failed: " + e.message);
        return false;
    }
});

// 3. DeleteFile ("文件路径")
QuernAPI.Register("DeleteFile", function(line, tokens) {
    if (tokens.length < 2) {
        QuernAPI.Log("[File Error] Usage: DeleteFile \"FilePath\"");
        return false;
    }

    var filePath = parseArg(tokens[1]);

    try {
        if (File.Exists(filePath)) {
            File.Delete(filePath);
            return true;
        } else {
            QuernAPI.Log("[File Warning] File not found for deletion: " + filePath);
            return false;
        }
    } catch (e) {
        QuernAPI.Log("[File Error] Delete failed: " + e.message);
        return false;
    }
});

// 4. NewFile ("文件路径")
QuernAPI.Register("NewFile", function(line, tokens) {
    // 格式 A: NewFile "path"
    // 格式 B: NewFile "content" > "path" (见下方合并逻辑，但这里先处理无内容的创建)
    
    // 检查是否有 > 符号，如果有，则属于带内容的创建，应该由下面的逻辑处理？
    // 不，为了区分，我们看 tokens 长度和结构。
    // 如果 tokens 包含 '>'，则可能是 NewFile Content > Path
    
    var hasRedirect = tokens.indexOf('>') !== -1;
    
    if (!hasRedirect) {
        // 简单创建空文件
        if (tokens.length < 2) {
            QuernAPI.Log("[File Error] Usage: NewFile \"FilePath\"");
            return false;
        }
        var filePath = parseArg(tokens[1]);
        try {
            // 确保目录存在
            var dir = Path.GetDirectoryName(filePath);
            if (dir && !Directory.Exists(dir)) {
                Directory.CreateDirectory(dir);
            }
            
            if (!File.Exists(filePath)) {
                File.Create(filePath).Close(); // Create returns a FileStream, close it immediately
            } else {
                QuernAPI.Log("[File Warning] File already exists: " + filePath);
            }
            return true;
        } catch (e) {
            QuernAPI.Log("[File Error] Create failed: " + e.message);
            return false;
        }
    } else {
        // 处理 NewFile (内容) > (路径)
        // 这种格式其实和 EditFile 很像，但语义是“新建”。如果文件已存在，EditFile 会覆盖。
        // 我们可以复用 EditFile 的逻辑，或者在这里明确禁止覆盖？
        // 通常 NewFile 如果存在则报错或忽略。这里选择如果存在则报错。
        
        if (tokens.length < 4) {
             QuernAPI.Log("[File Error] Usage: NewFile \"Content\" > \"FilePath\"");
             return false;
        }
        
        var sourceRaw = tokens[1];
        var filePath = parseArg(tokens[3]);
        
        if (File.Exists(filePath)) {
            QuernAPI.Log("[File Error] File already exists, cannot NewFile: " + filePath);
            return false;
        }

        var content = "";
        if (sourceRaw.startsWith('"') && sourceRaw.endsWith('"')) {
            content = parseArg(sourceRaw);
        } else {
            content = QuernAPI.GetVariable(sourceRaw);
            if (content === sourceRaw) content = sourceRaw;
        }
        
        var finalContent = processNewlines(content);
        
        try {
            var dir = Path.GetDirectoryName(filePath);
            if (dir && !Directory.Exists(dir)) {
                Directory.CreateDirectory(dir);
            }
            File.WriteAllText(filePath, finalContent);
            return true;
        } catch (e) {
            QuernAPI.Log("[File Error] Create/Write failed: " + e.message);
            return false;
        }
    }
});

// 5. MoveFile ("文件路径") > ("目标路径")
QuernAPI.Register("MoveFile", function(line, tokens) {
    if (tokens.length < 4 || tokens[2] !== '>') {
        QuernAPI.Log("[File Error] Usage: MoveFile \"SourcePath\" > \"DestPath\"");
        return false;
    }

    var sourcePath = parseArg(tokens[1]);
    var destPath = parseArg(tokens[3]);

    try {
        if (!File.Exists(sourcePath)) {
            QuernAPI.Log("[File Error] Source file not found: " + sourcePath);
            return false;
        }
        
        // 确保目标目录存在
        var destDir = Path.GetDirectoryName(destPath);
        if (destDir && !Directory.Exists(destDir)) {
            Directory.CreateDirectory(destDir);
        }

        File.Move(sourcePath, destPath);
        return true;
    } catch (e) {
        QuernAPI.Log("[File Error] Move failed: " + e.message);
        return false;
    }
});

// 6. CopyFile ("文件路径") > ("目标路径")
QuernAPI.Register("CopyFile", function(line, tokens) {
    if (tokens.length < 4 || tokens[2] !== '>') {
        QuernAPI.Log("[File Error] Usage: CopyFile \"SourcePath\" > \"DestPath\"");
        return false;
    }

    var sourcePath = parseArg(tokens[1]);
    var destPath = parseArg(tokens[3]);

    try {
        if (!File.Exists(sourcePath)) {
            QuernAPI.Log("[File Error] Source file not found: " + sourcePath);
            return false;
        }
        
        // 确保目标目录存在
        var destDir = Path.GetDirectoryName(destPath);
        if (destDir && !Directory.Exists(destDir)) {
            Directory.CreateDirectory(destDir);
        }

        // true 表示如果目标存在则覆盖
        File.Copy(sourcePath, destPath, true);
        return true;
    } catch (e) {
        QuernAPI.Log("[File Error] Copy failed: " + e.message);
        return false;
    }
});
