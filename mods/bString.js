// binput.js
function setVar(name, value, type) {
    try {
        if (!QuernAPI.Runtime) {
            QuernAPI.Log("[Error] QuernAPI.Runtime is not available.");
            return;
        }
        QuernAPI.Runtime.SetVariable(name, type, value.toString());
    } catch (e) {
        QuernAPI.Log("[Error] Failed to set variable '" + name + "': " + e);
    }
}

QuernAPI.Register("Input", function(line, tokens) {
    if (tokens.length >= 4 && tokens[2] === "=") {
        var prompt = tokens[1].replace(/^"|"$/g, '');
        var varName = tokens[3].replace(/^"|"$/g, '');
        
        if (prompt) QuernAPI.Log(prompt);
        
        var input = Console.ReadLine();
        setVar(varName, input, "String");
        return true;
    } 
    else if (tokens.length === 3 && tokens[1] === "=") {
         var varName = tokens[2].replace(/^"|"$/g, '');
         QuernAPI.Log("Enter value for " + varName + ":");
         var input = Console.ReadLine();
         setVar(varName, input, "String");
         return true;
    }
    
    return false;
});

QuernAPI.Register("InputNumber", function(line, tokens) {
    var prompt = "";
    var varName = "";

    if (tokens.length >= 4 && tokens[2] === "=") {
        prompt = tokens[1].replace(/^"|"$/g, '');
        varName = tokens[3].replace(/^"|"$/g, '');
    } 
    else if (tokens.length === 3 && tokens[1] === "=") {
        varName = tokens[2].replace(/^"|"$/g, '');
    }
    else if (tokens.length === 2) {
        varName = tokens[1].replace(/^"|"$/g, '');
    }

    if (!varName) {
        QuernAPI.Log("[Error] InputNumber syntax error. Could not find variable name.");
        return false; 
    }

    if (prompt) QuernAPI.Log(prompt + " (Number only):");
    else QuernAPI.Log("Enter number for " + varName + ":");

    while (true) {
        var input = Console.ReadLine();
        if (input === null) break; 
        
        var trimmedInput = input.trim();
        var isStrictInt = /^-?\d+$/.test(trimmedInput);
        
        if (isStrictInt) {
             var intVal = parseInt(trimmedInput);
             setVar(varName, intVal, "Number");
             break;
        } else {
            QuernAPI.Log("Invalid integer '" + trimmedInput + "'. Please try again:");
        }
    }
    
    return true;
});


QuernAPI.Register("Wait", function(line, tokens) {
    var match = line.match(/Wait\s*\(\s*(\d+)\s*\)/i);
    
    if (match) {
        var ms = parseInt(match[1]);
        
        if (!isNaN(ms) && ms >= 0) {
            try {
                var threadType = System.Threading.Thread;
                if (threadType && threadType.Sleep) {
                    threadType.Sleep(ms);
                } else {
                    throw new Error("Thread type not found");
                }
            } catch (e) {
                var start = new Date().getTime();
                var current = new Date().getTime();
                while (current - start < ms) {
                    current = new Date().getTime();
                }
            }
            return true;
        } else {
            QuernAPI.Log("[Error] Wait requires a valid positive number.");
            return false;
        }
    }
    
    return false;
});
QuernAPI.Register("WaitInput", function(line, tokens) {
    QuernAPI.Log("Press any key to continue...");
    Console.ReadKey(true);
    return true;
});

