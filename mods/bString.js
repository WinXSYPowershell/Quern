// BString.js

/**
 * Helper function to resolve variable values from the runtime.
 * If the token is a quoted string, it returns the content.
 * If it's a variable name, it fetches the value from QuernAPI.
 */
function resolveValue(token) {
    if (!token) return "";
    
    if (token.startsWith('"') && token.endsWith('"')) {
        return token.slice(1, -1);
    }
    
    try {
        var val = QuernAPI.GetVariable(token);
        
        if (val !== token) {
            return val;
        }
    } catch (e) {
        
    }
    
    return token;
}

/**
 * Syntax: String "VarName" Split "Delimiter"
 * Example: String "MyStr" Split ","
 * Action: Splits the variable 'MyStr' by delimiter and stores result in a List named 'MyStr_Split'
 */
QuernAPI.Register("String", function(line, tokens) {
    
    if (tokens.length < 3) {
        QuernAPI.Log("Error: String command requires at least a variable name and an operation.");
        return false;
    }

    var varNameToken = tokens[1];
    var operation = tokens[2];
    
    var varName = resolveValue(varNameToken);
    
    var strVal = QuernAPI.GetVariable(varName);
    
    if (!strVal || strVal === varName) {
         QuernAPI.Log("Warning: Variable '" + varName + "' might be undefined or empty.");
         strVal = "";
    }

    switch (operation) {
        case "Split":
            handleSplit(varName, strVal, tokens);
            break;
        case "SuString":
            handleSuString(varName, strVal, tokens);
            break;
        case "Trim":
            handleTrim(varName, tokens);
            break;
        case "Length":
            handleLength(varName, strVal, tokens);
            break;
        default:
            QuernAPI.Log("Error: Unknown String operation '" + operation + "'. Supported: Split, SuString, Trim, Length");
            return false;
    }

    return true;
});

function handleSplit(varName, strVal, tokens) {
    if (tokens.length < 4) {
        QuernAPI.Log("Error: Split requires a delimiter. Usage: String \"Var\" Split \",\"");
        return;
    }
    
    var delimiter = resolveValue(tokens[3]);
    var parts = strVal.split(delimiter);
    
    var listName = varName + "_List";
    
    var itemsStr = parts.map(function(item) {
        return '"' + item.replace(/"/g, '\\"') + '"';
    }).join(", ");
    
    var listDef = 'List = "' + listName + '" = [' + itemsStr + ']';
    
    if (QuernAPI.Runtime) {
        try {
            QuernAPI.Runtime.SetVariable(varName + "_Count", "Number", parts.length.toString());
            QuernAPI.Log("Split '" + varName + "' into " + parts.length + " parts. Stored count in '" + varName + "_Count'.");
            
            for(var i=0; i<parts.length; i++) {
                QuernAPI.Log("  Part[" + i + "]: " + parts[i]);
            }
            
        } catch (e) {
            QuernAPI.Log("Error accessing Runtime for Split: " + e.message);
        }
    } else {
        QuernAPI.Log("P: " + parts.join(" | "));
    }
}

function handleSuString(varName, strVal, tokens) {
    
    if (tokens.length < 5) {
        QuernAPI.Log("Error: SuString requires start index and length/count. Usage: String \"Var\" SuString 0 5");
        return;
    }
    
    var startIdx = parseInt(resolveValue(tokens[3]));
    var count = parseInt(resolveValue(tokens[4]));
    
    if (isNaN(startIdx) || isNaN(count)) {
        QuernAPI.Log("Error: SuString indices must be numbers.");
        return;
    }
    
    var result = strVal.substring(startIdx, startIdx + count);
    
    var targetVar = varName + "_Sub";
    
    if (QuernAPI.Runtime) {
        try {
            QuernAPI.Runtime.SetVariable(targetVar, "String", result);
            QuernAPI.Log("SuString of '" + varName + "' stored in '" + targetVar + "': " + result);
        } catch (e) {
            QuernAPI.Log("Error setting variable: " + e.message);
        }
    } else {
        QuernAPI.Log("SuString result: " + result);
    }
}

function handleTrim(varName, tokens) {
    
    var currentVal = QuernAPI.GetVariable(varName);
    if (currentVal === varName) currentVal = "";
    
    var trimmedVal = currentVal.trim();
    
    if (QuernAPI.Runtime) {
        try {
            QuernAPI.Runtime.SetVariable(varName, "String", trimmedVal);
            QuernAPI.Log("''" + trimmedVal);
        } catch (e) {
            QuernAPI.Log("Error trimming variable: " + e.message);
        }
    } else {
        QuernAPI.Log("R: '" + trimmedVal + "'");
    }
}

function handleLength(varName, strVal, tokens) {
    
    var len = strVal.length;
    var targetVar = varName + "_Len";
    
    if (QuernAPI.Runtime) {
        try {
            QuernAPI.Runtime.SetVariable(targetVar, "Number", len.toString());
            QuernAPI.Log("Length of '" + varName + "' is " + len + ". Stored in '" + targetVar + "'.");
        } catch (e) {
            QuernAPI.Log("Error setting length variable: " + e.message);
        }
    } else {
        QuernAPI.Log("'" + len);
    }
}
