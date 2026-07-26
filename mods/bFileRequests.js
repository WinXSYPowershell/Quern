// bFileRequests.js - Fixed System Namespace Issues

// 辅助函数：解析参数（去除引号或获取变量值）
function parseArg(arg) {
    if (!arg) return "";
    // 去除可能的引号
    if (arg.startsWith('"') && arg.endsWith('"')) {
        return arg.substring(1, arg.length - 1);
    }
    // 尝试获取变量
    var val = QuernAPI.GetVariable(arg);
    if (val !== arg) {
        return val;
    }
    return arg;
}

// 辅助函数：处理换行符
function processNewlines(str) {
    if (typeof str !== 'string') return str;
    return str.replace(/\\n/g, '\n');
}

// 核心解析器：从 tokens 中提取信息
function parseFileCommand(tokens, commandName) {
    var result = {
        chunkSize: 0,
        hasChunk: false,
        args: []
    };

    // 1. 检测并移除 ChunkMode("Size")
    var lastToken = tokens[tokens.length - 1];
    var chunkMatch = /^ChunkMode\("(\d+)"\)$/.exec(lastToken);
    
    if (chunkMatch) {
        result.hasChunk = true;
        result.chunkSize = parseInt(chunkMatch[1], 10);
        if (result.chunkSize <= 0) result.chunkSize = 4096;
        tokens = tokens.slice(0, tokens.length - 1);
    }

    // 2. 寻找重定向符号 ">"
    var redirectIndex = -1;
    for (var i = 0; i < tokens.length; i++) {
        if (tokens[i] === '>') {
            redirectIndex = i;
            break;
        }
    }

    if (redirectIndex === -1) {
        return null; 
    }

    // 3. 根据命令类型提取参数
    if (commandName === "ReadFile") {
        if (redirectIndex < 2) return null; 
        
        var filePath = parseArg(tokens[1]);
        var targetVar = parseArg(tokens[redirectIndex + 1]);
        
        return {
            type: 'read',
            filePath: filePath,
            targetVar: targetVar,
            chunkSize: result.chunkSize,
            hasChunk: result.hasChunk
        };
    } 
    else if (commandName === "EditFile" || commandName === "NewFile") {
        if (redirectIndex < 2) return null;
        
        var pathToken = tokens[redirectIndex + 1];
        var filePath = parseArg(pathToken);
        
        // 提取内容：从 index 1 到 redirectIndex - 1
        var contentTokens = [];
        for (var k = 1; k < redirectIndex; k++) {
            contentTokens.push(tokens[k]);
        }
        
        var rawContent = contentTokens.join(" ");
        
        var content = "";
        if (rawContent.startsWith('"') && rawContent.endsWith('"')) {
             content = rawContent.substring(1, rawContent.length - 1);
        } else {
            var varVal = QuernAPI.GetVariable(rawContent);
            if (varVal !== rawContent) {
                content = varVal;
            } else {
                // 处理被 split 破坏的字符串
                var cleanedParts = contentTokens.map(function(t) {
                    if (t.startsWith('"')) t = t.substring(1);
                    if (t.endsWith('"')) t = t.substring(0, t.length - 1);
                    return t;
                });
                content = cleanedParts.join(" ");
            }
        }
        
        return {
            type: 'write',
            filePath: filePath,
            content: content,
            chunkSize: result.chunkSize,
            hasChunk: result.hasChunk
        };
    }
    
    return null;
}

// --- ReadFile Command ---
QuernAPI.Register("ReadFile", function(line, tokens) {
    var params = parseFileCommand(tokens, "ReadFile");
    if (!params) {
        QuernAPI.Log("[File Error] Usage: ReadFile \"FilePath\" > \"VarName\" [ChunkMode(\"Size\")]");
        return false;
    }

    try {
        if (!File.Exists(params.filePath)) {
            QuernAPI.Log("[File Error] File not found: " + params.filePath);
            return false;
        }

        if (!params.hasChunk) {
            var content = File.ReadAllText(params.filePath);
            QuernAPI.Runtime.ApiSetVariable(params.targetVar, "String", content);
            QuernAPI.Log("[File Info] Read completed (One-shot): " + params.filePath);
        } else {
            QuernAPI.Runtime.ApiSetVariable(params.targetVar, "String", ""); 
            
            var reader = File.OpenText(params.filePath);
            var bufferSize = params.chunkSize; 
            var buffer = new Array(bufferSize);
            var currentContent = "";
            var readCount = 0;

            while ((readCount = reader.ReadBlock(buffer, 0, bufferSize)) > 0) {
                var chunk = new String(buffer, 0, readCount);
                currentContent += chunk;
                QuernAPI.Runtime.ApiSetVariable(params.targetVar, "String", currentContent);
            }
            
            reader.Close();
            QuernAPI.Log("[File Info] Read completed (Chunked " + bufferSize + "): " + params.filePath);
        }
        return true;
    } catch (e) {
        QuernAPI.Log("[File Error] Read failed: " + e.message);
        return false;
    }
});

// --- EditFile Command ---
QuernAPI.Register("EditFile", function(line, tokens) {
    var params = parseFileCommand(tokens, "EditFile");
    if (!params) {
        QuernAPI.Log("[File Error] Usage: EditFile \"Content\" > \"FilePath\" [ChunkMode(\"Size\")]");
        return false;
    }

    var finalContent = processNewlines(params.content);

    try {
        var dir = Path.GetDirectoryName(params.filePath);
        if (dir && !Directory.Exists(dir)) {
            Directory.CreateDirectory(dir);
        }

        if (!params.hasChunk) {
            File.WriteAllText(params.filePath, finalContent);
            QuernAPI.Log("[File Info] Write completed (One-shot): " + params.filePath);
        } else {
            // [FIXED] 使用全局暴露的 StreamWriter 和 Encoding
            var writer = new StreamWriter(params.filePath, false, Encoding.UTF8);
            var chunkSize = params.chunkSize;
            var totalLength = finalContent.length;
            
            for (var i = 0; i < totalLength; i += chunkSize) {
                var end = Math.min(i + chunkSize, totalLength);
                var chunk = finalContent.substring(i, end);
                writer.Write(chunk);
            }
            
            writer.Flush();
            writer.Close();
            QuernAPI.Log("[File Info] Write completed (Chunked " + chunkSize + "): " + params.filePath);
        }
        return true;
    } catch (e) {
        QuernAPI.Log("[File Error] Write failed: " + e.message);
        return false;
    }
});

// --- NewFile Command ---
QuernAPI.Register("NewFile", function(line, tokens) {
    var lastToken = tokens[tokens.length - 1];
    var chunkMatch = /^ChunkMode\("(\d+)"\)$/.exec(lastToken);
    var hasChunk = false;
    var chunkSize = 4096;
    var cmdTokens = tokens;
    
    if (chunkMatch) {
        hasChunk = true;
        chunkSize = parseInt(chunkMatch[1], 10);
        if (chunkSize <= 0) chunkSize = 4096;
        cmdTokens = tokens.slice(0, tokens.length - 1);
    }

    var redirectIndex = -1;
    for (var i = 0; i < cmdTokens.length; i++) {
        if (cmdTokens[i] === '>') {
            redirectIndex = i;
            break;
        }
    }

    var doCreate = function(fp, content, useChunk, cSize) {
        try {
            var dir = Path.GetDirectoryName(fp);
            if (dir && !Directory.Exists(dir)) {
                Directory.CreateDirectory(dir);
            }
            
            if (content === null) {
                if (!File.Exists(fp)) {
                    var fs = File.Create(fp);
                    fs.Close();
                }
            } else {
                if (File.Exists(fp)) {
                    QuernAPI.Log("[File Warning] File exists, overwriting: " + fp);
                }
                
                if (!useChunk) {
                    File.WriteAllText(fp, content);
                } else {
                    // [FIXED] 使用全局暴露的 StreamWriter 和 Encoding
                    var writer = new StreamWriter(fp, false, Encoding.UTF8);
                    var totalLength = content.length;
                    for (var i = 0; i < totalLength; i += cSize) {
                        var end = Math.min(i + cSize, totalLength);
                        writer.Write(content.substring(i, end));
                    }
                    writer.Flush();
                    writer.Close();
                }
            }
            return true;
        } catch (e) {
            QuernAPI.Log("[File Error] Create failed: " + e.message);
            return false;
        }
    };

    if (redirectIndex === -1) {
        if (cmdTokens.length < 2) {
            QuernAPI.Log("[File Error] Usage: NewFile \"FilePath\"");
            return false;
        }
        var filePath = parseArg(cmdTokens[1]);
        return doCreate(filePath, null, false, 0);
    } else {
        if (redirectIndex < 2) { 
             QuernAPI.Log("[File Error] Usage: NewFile \"Content\" > \"FilePath\"");
             return false;
        }
        
        var pathToken = cmdTokens[redirectIndex + 1];
        var filePath = parseArg(pathToken);
        
        var contentTokens = [];
        for (var k = 1; k < redirectIndex; k++) {
            contentTokens.push(cmdTokens[k]);
        }
        var rawContent = contentTokens.join(" ");
        
        var content = "";
        if (rawContent.startsWith('"') && rawContent.endsWith('"')) {
             content = rawContent.substring(1, rawContent.length - 1);
        } else {
            var varVal = QuernAPI.GetVariable(rawContent);
            if (varVal !== rawContent) {
                content = varVal;
            } else {
                var cleanedParts = contentTokens.map(function(t) {
                    if (t.startsWith('"')) t = t.substring(1);
                    if (t.endsWith('"')) t = t.substring(0, t.length - 1);
                    return t;
                });
                content = cleanedParts.join(" ");
            }
        }
        
        var finalContent = processNewlines(content);
        return doCreate(filePath, finalContent, hasChunk, chunkSize);
    }
});

// --- DeleteFile Command ---
QuernAPI.Register("DeleteFile", function(line, tokens) {
    var lastToken = tokens[tokens.length - 1];
    var chunkMatch = /^ChunkMode\("(\d+)"\)$/.exec(lastToken);
    var cmdTokens = tokens;
    if (chunkMatch) {
        cmdTokens = tokens.slice(0, tokens.length - 1);
    }

    if (cmdTokens.length < 2) {
        QuernAPI.Log("[File Error] Usage: DeleteFile \"FilePath\"");
        return false;
    }

    var filePath = parseArg(cmdTokens[1]);

    try {
        if (File.Exists(filePath)) {
            File.Delete(filePath);
            QuernAPI.Log("[File Info] Deleted: " + filePath);
            return true;
        } else {
            QuernAPI.Log("[File Warning] Not found: " + filePath);
            return false;
        }
    } catch (e) {
        QuernAPI.Log("[File Error] Delete failed: " + e.message);
        return false;
    }
});

// --- MoveFile Command ---
QuernAPI.Register("MoveFile", function(line, tokens) {
    var lastToken = tokens[tokens.length - 1];
    var chunkMatch = /^ChunkMode\("(\d+)"\)$/.exec(lastToken);
    var cmdTokens = tokens;
    if (chunkMatch) {
        cmdTokens = tokens.slice(0, tokens.length - 1);
    }

    var redirectIndex = -1;
    for(var i=0; i<cmdTokens.length; i++) { if(cmdTokens[i]=='>') { redirectIndex=i; break; } }
    
    if(redirectIndex === -1) {
         QuernAPI.Log("[File Error] Usage: MoveFile \"Src\" > \"Dest\"");
         return false;
    }
    
    var srcTokens = [];
    for(var k=1; k<redirectIndex; k++) srcTokens.push(cmdTokens[k]);
    var srcRaw = srcTokens.join(" ");
    if(srcRaw.startsWith('"') && srcRaw.endsWith('"')) srcRaw = srcRaw.substring(1, srcRaw.length-1);
    var srcVal = QuernAPI.GetVariable(srcRaw);
    var src = (srcVal !== srcRaw) ? srcVal : srcRaw;

    var dest = parseArg(cmdTokens[redirectIndex + 1]);

    try {
        if (!File.Exists(src)) {
            QuernAPI.Log("[File Error] Source not found: " + src);
            return false;
        }
        var dir = Path.GetDirectoryName(dest);
        if (dir && !Directory.Exists(dir)) Directory.CreateDirectory(dir);
        
        File.Move(src, dest);
        QuernAPI.Log("[File Info] Moved: " + src + " to " + dest);
        return true;
    } catch (e) {
        QuernAPI.Log("[File Error] Move failed: " + e.message);
        return false;
    }
});

// --- CopyFile Command ---
QuernAPI.Register("CopyFile", function(line, tokens) {
    var lastToken = tokens[tokens.length - 1];
    var chunkMatch = /^ChunkMode\("(\d+)"\)$/.exec(lastToken);
    var cmdTokens = tokens;
    if (chunkMatch) {
        cmdTokens = tokens.slice(0, tokens.length - 1);
    }

    var redirectIndex = -1;
    for(var i=0; i<cmdTokens.length; i++) { if(cmdTokens[i]=='>') { redirectIndex=i; break; } }
    
    if(redirectIndex === -1) {
         QuernAPI.Log("[File Error] Usage: CopyFile \"Src\" > \"Dest\"");
         return false;
    }
    
    var srcTokens = [];
    for(var k=1; k<redirectIndex; k++) srcTokens.push(cmdTokens[k]);
    var srcRaw = srcTokens.join(" ");
    if(srcRaw.startsWith('"') && srcRaw.endsWith('"')) srcRaw = srcRaw.substring(1, srcRaw.length-1);
    var srcVal = QuernAPI.GetVariable(srcRaw);
    var src = (srcVal !== srcRaw) ? srcVal : srcRaw;

    var dest = parseArg(cmdTokens[redirectIndex + 1]);

    try {
        if (!File.Exists(src)) {
            QuernAPI.Log("[File Error] Source not found: " + src);
            return false;
        }
        var dir = Path.GetDirectoryName(dest);
        if (dir && !Directory.Exists(dir)) Directory.CreateDirectory(dir);
        
        File.Copy(src, dest, true);
        QuernAPI.Log("[File Info] Copied: " + src + " to " + dest);
        return true;
    } catch (e) {
        QuernAPI.Log("[File Error] Copy failed: " + e.message);
        return false;
    }
});
