// bdebug.js
// 为 Quern Engine 提供断言、调试和高级条件检查功能

QuernAPI.Register("Assert", function(line, tokens) {
    // 语法: Assert (Condition), "Error Message"
    
    try {
        // 1. 提取错误消息
        let errorMessage = "Assertion Failed";
        // 查找最后一个逗号之后的内容作为消息
        const lastCommaIndex = line.lastIndexOf(',');
        if (lastCommaIndex !== -1) {
            const msgPart = line.substring(lastCommaIndex + 1).trim();
            // 去除引号
            if (msgPart.startsWith('"') && msgPart.endsWith('"')) {
                errorMessage = msgPart.slice(1, -1);
            } else {
                errorMessage = msgPart;
            }
        }

        // 2. 提取条件部分
        // 格式: Assert (Condition), ...
        const startParen = line.indexOf('(');
        const endParen = line.lastIndexOf(')'); // 使用 lastIndexOf 以防条件中包含函数调用等复杂情况
        
        if (startParen === -1 || endParen === -1 || endParen <= startParen) {
            QuernAPI.Log("[ASSERT ERROR] Malformed syntax. Use: Assert (Condition), \"Message\"");
            return false;
        }

        const conditionStr = line.substring(startParen + 1, endParen).trim();
        
        // 3. 评估条件
        const runtime = QuernAPI.Runtime;
        let isTrue = false;
        
        if (runtime) {
             isTrue = evaluateSimpleConditionJS(conditionStr, runtime);
        } else {
             QuernAPI.Log("[ASSERT WARN] Runtime not accessible.");
             isTrue = true; // Fail safe? Or false? Let's say true to avoid crash if no runtime
        }

        if (!isTrue) {
            QuernAPI.Log("[FATAL ASSERTION] " + errorMessage);
            // 抛出异常以试图中断执行
            // 注意：为了让 C# 能够识别这是“致命错误”，我们抛出一个特定的错误对象
            throw { name: "QuernFatalError", message: errorMessage };
        }
        
        return true;
    } catch (e) {
        // 如果是我们的致命错误，重新抛出以便 C# 捕获并处理（如果 C# 支持）
        if (e.name === "QuernFatalError") {
            throw e;
        }
        QuernAPI.Log("[ASSERT EXCEPTION] " + e.message);
        throw e;
    }
});

// 辅助函数：在 JS 端简单模拟条件评估
function evaluateSimpleConditionJS(cond, runtime) {
    cond = cond.trim();
    
    // 1. 检查 In 范围: "Val" In Min-Max 或 Val In Min-Max
    // 正则：匹配 (任意字符) In (数字)-(数字)
    const inRegex = /^(.+?)\s+In\s+(\d+)\s*-\s*(\d+)$/i;
    const inMatch = cond.match(inRegex);
    if (inMatch) {
        const rawVal = inMatch[1].trim();
        const min = parseInt(inMatch[2]);
        const max = parseInt(inMatch[3]);
        
        const val = resolveValue(rawVal, runtime);
        const numVal = parseFloat(val);
        
        if (isNaN(numVal)) {
            QuernAPI.Log("[DEBUG] In Check Failed: '" + val + "' is not a number.");
            return false;
        }
        return numVal >= min && numVal <= max;
    }

    // 2. 检查 IsNumber / IsString
    // 匹配: "VarName" IsNumber 或 VarName IsNumber
    if (/IsNumber$/i.test(cond)) {
        const varNameRaw = cond.replace(/IsNumber$/i, "").trim();
        const val = resolveValue(varNameRaw, runtime);
        // 判断是否为有效数字
        return !isNaN(parseFloat(val)) && isFinite(val);
    }
    
    if (/IsString$/i.test(cond)) {
        const varNameRaw = cond.replace(/IsString$/i, "").trim();
        const val = resolveValue(varNameRaw, runtime);
        // 如果它不能被解析为数字，或者它是空串，我们认为是字符串
        // 注意：在 Quern 中，数字也是以字符串形式存储的，所以这里主要看内容
        // 更严格的检查需要 C# 暴露类型，这里仅做内容启发式判断
        const isNum = !isNaN(parseFloat(val)) && isFinite(val);
        return !isNum; 
    }

    // 3. 基本比较: A > B, A == B, etc.
    // 操作符优先级：>=, <=, !=, ==, >, <
    const operators = ['>=', '<=', '!=', '==', '>', '<'];
    for (let op of operators) {
        // 使用 split 限制为 2 部分，防止表达式中有多个相同操作符
        const parts = cond.split(op);
        if (parts.length === 2) {
            const leftRaw = parts[0].trim();
            const rightRaw = parts[1].trim();
            
            const left = resolveValue(leftRaw, runtime);
            const right = resolveValue(rightRaw, runtime);
            
            // 尝试转为数字比较
            const leftNum = parseFloat(left);
            const rightNum = parseFloat(right);
            
            // 如果两边都是有效数字，进行数字比较
            if (!isNaN(leftNum) && !isNaN(rightNum) && left.trim() !== "" && right.trim() !== "") {
                 // 额外检查：防止将 "12abc" 解析为 12
                 if (String(leftNum) === left.trim() && String(rightNum) === right.trim()) {
                    switch (op) {
                        case '>': return leftNum > rightNum;
                        case '<': return leftNum < rightNum;
                        case '>=': return leftNum >= rightNum;
                        case '<=': return leftNum <= rightNum;
                        case '==': return leftNum === rightNum;
                        case '!=': return leftNum !== rightNum;
                    }
                 }
            } 
            
            // 否则进行字符串比较
            switch (op) {
                case '==': return left === right;
                case '!=': return left !== right;
                // 字符串不支持 > < >= <= 在此简化版中，返回 false
                default: return false; 
            }
        }
    }
    
    // 4. 单个变量/值：非零即真
    const val = resolveValue(cond, runtime);
    if (val === "0" || val === "" || val.toLowerCase() === "false") return false;
    return true;
}

function resolveValue(token, runtime) {
    if (!token) return "";
    token = token.trim();
    
    // 移除包裹的引号 "Var" -> Var
    if (token.startsWith('"') && token.endsWith('"')) {
        token = token.slice(1, -1);
    }
    
    // 检查是否是列表引用 L+Name+Idx
    if (token.startsWith("L+")) {
        const parts = token.split('+');
        if (parts.length === 3) {
            const listName = parts[1];
            const idx = parseInt(parts[2]);
            const list = QuernAPI.GetList(listName);
            if (list && idx >= 0 && idx < list.length) {
                return list[idx];
            }
        }
    }

    // 尝试作为变量获取
    if (runtime && runtime.GetVariableString) {
        // GetVariableString 如果找不到变量，通常返回变量名本身
        const val = runtime.GetVariableString(token);
        if (val !== token) {
            return val;
        }
    }
    
    // 返回原值（可能是数字字面量或字符串字面量）
    return token;
}

// 注册额外的调试助手
QuernAPI.Register("DebugLog", function(line, tokens) {
    const match = line.match(/DebugLog\s+"(.*)"/);
    if (match) {
        console.log("[DEBUG] " + match[1]);
    }
    return true;
});