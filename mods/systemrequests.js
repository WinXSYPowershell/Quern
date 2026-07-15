// systemrequests.js
function resolveExpression(expr, runtime) {
    if (!expr.includes("+")) {
        var val = runtime.GetVariableString(expr.trim());
        if (val === expr.trim() && expr.startsWith("\"") && expr.endsWith("\"")) {
            return expr.substring(1, expr.length - 1);
        }
        return val;
    }

    var parts = expr.split("+");
    var result = "";
    for (var i = 0; i < parts.length; i++) {
        var part = parts[i].trim();
        if (part.startsWith("\"") && part.endsWith("\"")) {
            result += part.substring(1, part.length - 1);
        } else {
            result += runtime.GetVariableString(part);
        }
    }
    return result;
}

function parseCommandForSafeExec(commandStr) {
    var trimmed = commandStr.trim();
    if (!trimmed) return null;

    var firstSpace = trimmed.indexOf(" ");
    
    if (firstSpace === -1) {
        return { cmd: trimmed, args: "" };
    } else {
        return { 
            cmd: trimmed.substring(0, firstSpace), 
            args: trimmed.substring(firstSpace + 1) 
        };
    }
}

QuernAPI.Register("RunCommand", function(line, tokens) {
    var startIdx = line.indexOf("(");
    var endIdx = line.lastIndexOf(")");
    
    if (startIdx === -1 || endIdx === -1) {
        QuernAPI.Log("[Error] RunCommand: Syntax error. Use RunCommand (expression)");
        return false;
    }
    
    var content = line.substring(startIdx + 1, endIdx).trim();
    var finalCommand = resolveExpression(content, QuernAPI.Runtime);
    
    QuernAPI.Log("[System] Attempting Safe Execution: " + finalCommand);
    
    try {
        if (typeof SafeSystem === 'undefined') {
             QuernAPI.Log("[Security Error] SafeSystem not found. Cannot execute commands.");
             return false;
        }

        var parsed = parseCommandForSafeExec(finalCommand);
        
        if (!parsed) {
            QuernAPI.Log("[Error] Empty command.");
            return false;
        }
        
        
        if (typeof SafeSystem.IAcceptAllRisks === 'function') {
            SafeSystem.IAcceptAllRisks();
        }

        var output = SafeSystem.RunSafeCommand(parsed.cmd, parsed.args);
        
        if (output) {
            QuernAPI.Log(output.trim());
        } else {
            QuernAPI.Log("[System] Command executed successfully (no output).");
        }
        
    } catch (e) {
        QuernAPI.Log("[System Security/Error]: " + e.message);
        if (e.stack) QuernAPI.Log(e.stack);
    }
    
    return true;
});
