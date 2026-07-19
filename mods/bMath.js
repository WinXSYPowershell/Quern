// bmath.js
function getNumericValue(tokens, index) {
    if (index >= tokens.length) return null;
    var raw = tokens[index];
    
    var num = parseFloat(raw);
    if (!isNaN(num)) return num;

    try {
        var varVal = QuernAPI.GetVariable(raw);
        if (varVal !== raw) {
            var varNum = parseFloat(varVal);
            if (!isNaN(varNum)) return varNum;
        }
    } catch (e) {
        
    }
    
    return null;
}

function seededRandom(seed) {
    var m = 0x80000000;
    var a = 1103515245;
    var c = 12345;
    seed = Math.abs(seed); 
    seed = (a * seed + c) % m;
    return seed / m;
}


QuernAPI.Register("Pow", function(line, tokens) {
    try {
        if (tokens.length < 3) throw "Pow requires 2 arguments (base, exp)";
        var base = getNumericValue(tokens, 1);
        var exp = getNumericValue(tokens, 2);
        
        if (base === null || exp === null) throw "Invalid number or variable";
        
        QuernAPI.Log(Math.pow(base, exp));
        return true;
    } catch (e) {
        QuernAPI.Log("[BMath Error] in 'Pow': " + e);
        return false;
    }
});

QuernAPI.Register("Sqrt", function(line, tokens) {
    try {
        if (tokens.length < 2) throw "Sqrt requires 1 argument";
        var num = getNumericValue(tokens, 1);
        
        if (num === null) throw "Invalid number or variable";
        if (num < 0) throw "Cannot calculate square root of negative number";
        
        QuernAPI.Log(Math.sqrt(num));
        return true;
    } catch (e) {
        QuernAPI.Log("[BMath Error] in 'Sqrt': " + e);
        return false;
    }
});

QuernAPI.Register("Abs", function(line, tokens) {
    try {
        if (tokens.length < 2) throw "Abs requires 1 argument";
        var num = getNumericValue(tokens, 1);
        
        if (num === null) throw "Invalid number or variable";
        
        QuernAPI.Log(Math.abs(num));
        return true;
    } catch (e) {
        QuernAPI.Log("[BMath Error] in 'Abs': " + e);
        return false;
    }
});


QuernAPI.Register("Ceiling", function(line, tokens) {
    try {
        if (tokens.length < 2) throw "Ceiling requires 1 argument";
        var num = getNumericValue(tokens, 1);
        if (num === null) throw "Invalid number or variable";
        
        QuernAPI.Log(Math.Ceiling(num));
        return true;
    } catch (e) {
        QuernAPI.Log("[BMath Error] in 'Ceiling': " + e);
        return false;
    }
});

QuernAPI.Register("Floor", function(line, tokens) {
    try {
        if (tokens.length < 2) throw "Floor requires 1 argument";
        var num = getNumericValue(tokens, 1);
        if (num === null) throw "Invalid number or variable";
        
        QuernAPI.Log(Math.floor(num));
        return true;
    } catch (e) {
        QuernAPI.Log("[BMath Error] in 'Floor': " + e);
        return false;
    }
});

QuernAPI.Register("Round", function(line, tokens) {
    try {
        if (tokens.length < 2) throw "Round requires 1 argument";
        var num = getNumericValue(tokens, 1);
        if (num === null) throw "Invalid number or variable";
        
        QuernAPI.Log(Math.round(num));
        return true;
    } catch (e) {
        QuernAPI.Log("[BMath Error] in 'Round': " + e);
        return false;
    }
});


QuernAPI.Register("Random", function(line, tokens) {
    try {
        if (tokens.length < 3) throw "Random requires min and max";
        
        var min = getNumericValue(tokens, 1);
        var max = getNumericValue(tokens, 2);
        
        if (min === null || max === null) throw "Invalid number or variable";
        
        if (min > max) { var temp = min; min = max; max = temp; }
        
        var seed = Date.now();
        var rand = seededRandom(seed);
        
        var result = Math.floor(rand * (max - min + 1)) + min;
        
        QuernAPI.Log(result);
        return true;
    } catch (e) {
        QuernAPI.Log("[BMath Error] in 'Random': " + e);
        return false;
    }
});

QuernAPI.Register("RandomFloat", function(line, tokens) {
    try {
        if (tokens.length < 3) throw "RandomFloat requires min and max";
        
        var min = getNumericValue(tokens, 1);
        var max = getNumericValue(tokens, 2);
        
        if (min === null || max === null) throw "Invalid number or variable";
        
        if (min > max) { var temp = min; min = max; max = temp; }
        
        var seed = Date.now();
        var rand = seededRandom(seed);
        
        var result = rand * (max - min) + min;
        
        QuernAPI.Log(result.toFixed(4));
        return true;
    } catch (e) {
        QuernAPI.Log("[BMath Error] in 'RandomFloat': " + e);
        return false;
    }
});


QuernAPI.Register("Sin", function(line, tokens) {
    try {
        if (tokens.length < 2) throw "Sin requires 1 argument (radians)";
        var rad = getNumericValue(tokens, 1);
        if (rad === null) throw "Invalid number or variable";
        
        QuernAPI.Log(Math.sin(rad).toFixed(4));
        return true;
    } catch (e) {
        QuernAPI.Log("[BMath Error] in 'Sin': " + e);
        return false;
    }
});

QuernAPI.Register("Cos", function(line, tokens) {
    try {
        if (tokens.length < 2) throw "Cos requires 1 argument (radians)";
        var rad = getNumericValue(tokens, 1);
        if (rad === null) throw "Invalid number or variable";
        
        QuernAPI.Log(Math.cos(rad).toFixed(4));
        return true;
    } catch (e) {
        QuernAPI.Log("[BMath Error] in 'Cos': " + e);
        return false;
    }
});

QuernAPI.Register("Tan", function(line, tokens) {
    try {
        if (tokens.length < 2) throw "Tan requires 1 argument (radians)";
        var rad = getNumericValue(tokens, 1);
        if (rad === null) throw "Invalid number or variable";
        
        QuernAPI.Log(Math.tan(rad).toFixed(4));
        return true;
    } catch (e) {
        QuernAPI.Log("[BMath Error] in 'Tan': " + e);
        return false;
    }
});
