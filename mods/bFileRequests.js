// bFileRequests.js
// 提供文件读写、新建、删除、移动和复制功能
function parseArg(arg) {
    if (!arg) return "";
    if (arg.startsWith('"') && arg.endsWith('"')) {
        return arg.substring(1, arg.length - 1);
    }
    var val = QuernAPI.GetVariable(arg);
    if (val !== arg) {
        return val;
    }
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
        var escapedContent = escapeNewlines(content);
        
        QuernAPI.Runtime.ApiSetVariable(targetName, "String", escapedContent);
        return true;
    } catch (e) {
        QuernAPI.Log("[File Error] Read failed: " + e.message);
        return false;
    }
});

QuernAPI.Register("EditFile", function(line, tokens) {
    
    if (tokens.length < 4 || tokens[2] !== '>') {
        QuernAPI.Log("[File Error] Usage: EditFile \"ContentOrVar\" > \"FilePath\"");
        return false;
    }

    var sourceRaw = tokens[1];
    var filePath = parseArg(tokens[3]);

    var content = "";
    
    if (sourceRaw.startsWith('"') && sourceRaw.endsWith('"')) {
        content = parseArg(sourceRaw);
    } else {
        content = QuernAPI.GetVariable(sourceRaw);
        if (content === sourceRaw) {
             QuernAPI.Log("[File Warning] Variable '" + sourceRaw + "' not found or invalid.");
             content = sourceRaw;
        }
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
        QuernAPI.Log("[File Error] Write failed: " + e.message);
        return false;
    }
});

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

QuernAPI.Register("NewFile", function(line, tokens) {
    
    var hasRedirect = tokens.indexOf('>') !== -1;
    
    if (!hasRedirect) {
        if (tokens.length < 2) {
            QuernAPI.Log("[File Error] Usage: NewFile \"FilePath\"");
            return false;
        }
        var filePath = parseArg(tokens[1]);
        try {
            var dir = Path.GetDirectoryName(filePath);
            if (dir && !Directory.Exists(dir)) {
                Directory.CreateDirectory(dir);
            }
            
            if (!File.Exists(filePath)) {
                File.Create(filePath).Close();
            } else {
                QuernAPI.Log("[File Warning] File already exists: " + filePath);
            }
            return true;
        } catch (e) {
            QuernAPI.Log("[File Error] Create failed: " + e.message);
            return false;
        }
    } else {
        
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
        
        var destDir = Path.GetDirectoryName(destPath);
        if (destDir && !Directory.Exists(destDir)) {
            Directory.CreateDirectory(destDir);
        }

        File.Copy(sourcePath, destPath, true);
        return true;
    } catch (e) {
        QuernAPI.Log("[File Error] Copy failed: " + e.message);
        return false;
    }
});
