// bdebug.js
if (typeof console === 'undefined') {
    var console = {
        log: function(msg) { QuernAPI.Log("[DEBUG] " + msg); },
        error: function(msg) { QuernAPI.Log("[ERROR] " + msg); },
        warn: function(msg) { QuernAPI.Log("[WARN] " + msg); }
    };
}

QuernAPI.Register("Assert", function(line, tokens) {
    
    try {
        let errorMessage = "Assertion Failed";
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

        const startParen = line.indexOf('(');
        const endParen = line.lastIndexOf(')'); 
        
        if (startParen === -1 || endParen === -1 || endParen <= startParen) {
            QuernAPI.Log("[ASSERT ERROR] Malformed syntax. Use: Assert (Condition), \"Message\"");
            return false;
        }

        const conditionStr = line.substring(startParen + 1, endParen).trim();
        
        const runtime = QuernAPI.Runtime;
        let isTrue = false;
        
        if (runtime) {
             isTrue = evaluateSimpleConditionJS(conditionStr, runtime);
        } else {
             QuernAPI.Log("[ASSERT WARN] Runtime not accessible.");
             isTrue = true; 
        }

        if (!isTrue) {
            QuernAPI.Log("[FATAL ASSERTION] " + errorMessage);
            throw { name: "QuernFatalError", message: errorMessage };
        }
        
        return true;
    } catch (e) {
        if (e.name === "QuernFatalError") {
            throw e;
        }
        QuernAPI.Log("[ASSERT EXCEPTION] " + e.message);
        throw e;
    }
});

QuernAPI.Register("DebugLog", function(line, tokens) {
    const match = line.match(/DebugLog\s+"(.*)"/);
    if (match) {
        console.log(match[1]);
    } else {
        const parts = line.split(/\s+/);
        if (parts.length > 1) {
            console.log(parts.slice(1).join(" "));
        }
    }
    return true;
});

function evaluateSimpleConditionJS(cond, runtime) {
    cond = cond.trim();
    
    const inRegex = /^(.+?)\s+In\s+(\d+)\s*-\s*(\d+)$/i;
    const inMatch = cond.match(inRegex);
    if (inMatch) {
        const rawVal = inMatch[1].trim();
        const min = parseInt(inMatch[2]);
        const max = parseInt(inMatch[3]);
        
        const val = resolveValue(rawVal, runtime);
        const numVal = parseFloat(val);
        
        if (isNaN(numVal)) {
            return false;
        }
        return numVal >= min && numVal <= max;
    }

    if (/IsNumber$/i.test(cond)) {
        const varNameRaw = cond.replace(/IsNumber$/i, "").trim();
        const val = resolveValue(varNameRaw, runtime);
        return !isNaN(parseFloat(val)) && isFinite(val);
    }
    
    if (/IsString$/i.test(cond)) {
        const varNameRaw = cond.replace(/IsString$/i, "").trim();
        const val = resolveValue(varNameRaw, runtime);
        const isNum = !isNaN(parseFloat(val)) && isFinite(val);
        return !isNum; 
    }

    const operators = ['>=', '<=', '!=', '==', '>', '<'];
    for (let op of operators) {
        const idx = cond.indexOf(op);
        if (idx !== -1) {
            const leftRaw = cond.substring(0, idx).trim();
            const rightRaw = cond.substring(idx + op.length).trim();
            
            if (leftRaw === "" || rightRaw === "") continue;

            const left = resolveValue(leftRaw, runtime);
            const right = resolveValue(rightRaw, runtime);
            
            const leftNum = parseFloat(left);
            const rightNum = parseFloat(right);
            if (!isNaN(leftNum) && !isNaN(rightNum)) {
                 switch (op) {
                    case '>': return leftNum > rightNum;
                    case '<': return leftNum < rightNum;
                    case '>=': return leftNum >= rightNum;
                    case '<=': return leftNum <= rightNum;
                    case '==': return leftNum === rightNum;
                    case '!=': return leftNum !== rightNum;
                }
            } 
            
            switch (op) {
                case '==': return left === right;
                case '!=': return left !== right;
                default: return false; 
            }
        }
    }
    
    const val = resolveValue(cond, runtime);
    if (val === "0" || val === "" || val.toLowerCase() === "false") return false;
    return true;
}

function resolveValue(token, runtime) {
    if (!token) return "";
    token = token.trim();
    
    if (token.startsWith('"') && token.endsWith('"')) {
        return token.slice(1, -1);
    }
    
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

    if (runtime && runtime.GetVariableString) {
        const val = runtime.GetVariableString(token);
        if (val !== token) {
            return val;
        }
    }
    
    return token;
}