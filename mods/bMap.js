// bMap.js
function getInternalMap(mapName) {
    var jsonStr = QuernAPI.GetVariable(mapName);
    if (!jsonStr || jsonStr === mapName) { 
        return {};
    }
    try {
        var parsed = JSON.parse(jsonStr);
        if (typeof parsed === 'object' && parsed !== null && !Array.isArray(parsed)) {
            return parsed;
        }
        return {};
    } catch (e) {
        return {};
    }
}

function saveInternalMap(mapName, mapObj) {
    var jsonString = JSON.stringify(mapObj);
    if (QuernAPI.Runtime && QuernAPI.Runtime.ApiSetVariable) {
        QuernAPI.Runtime.ApiSetVariable(mapName, "String", jsonString);
    } else {
        QuernAPI.Log("[Map Error] Runtime API not available for saving map.");
    }
}

function parseArg(token) {
    if (token.startsWith('"') && token.endsWith('"')) {
        return token.slice(1, -1);
    }
    return token;
}

QuernAPI.Register("MapInit", function(line, tokens) {
    if (tokens.length < 2) {
        QuernAPI.Log("[Map Error] Usage: MapInit \"MapName\"");
        return false;
    }
    var mapName = parseArg(tokens[1]);
    saveInternalMap(mapName, {});
    return true;
});

QuernAPI.Register("MapSet", function(line, tokens) {
    if (tokens.length < 4 || (tokens.length - 2) % 2 !== 0) {
        QuernAPI.Log("[Map Error] Usage: MapSet \"MapName\" \"Key1\" \"Val1\" [\"Key2\" \"Val2\"...]");
        return false;
    }
    
    var mapName = parseArg(tokens[1]);
    var mapObj = getInternalMap(mapName);
    
    for (var i = 2; i < tokens.length; i += 2) {
        var key = parseArg(tokens[i]);
        var value = parseArg(tokens[i+1]);
        
        if (!isNaN(value) && value.trim() !== "") {
            value = Number(value);
        }
        
        mapObj[key] = value;
    }
    
    saveInternalMap(mapName, mapObj);
    return true;
});

QuernAPI.Register("PrintMap", function(line, tokens) {
    if (tokens.length < 2) {
        QuernAPI.Log("[Map Error] Usage: PrintMap \"MapName\" [\"Key\"] > CommandLine");
        return false;
    }
    
    var mapName = parseArg(tokens[1]);
    var mapObj = getInternalMap(mapName);
    
    if (tokens.length >= 3) {
        var key = parseArg(tokens[2]);
        if (mapObj.hasOwnProperty(key)) {
            QuernAPI.Log(String(mapObj[key]));
        } else {
            QuernAPI.Log("[Map Error] Key '" + key + "' not found in map '" + mapName + "'.");
        }
    } else {
        QuernAPI.Log(JSON.stringify(mapObj, null, 2));
    }
    return true;
});

QuernAPI.Register("MapDel", function(line, tokens) {
    if (tokens.length < 3) {
        QuernAPI.Log("[Map Error] Usage: MapDel \"MapName\" \"Key\"");
        return false;
    }
    
    var mapName = parseArg(tokens[1]);
    var key = parseArg(tokens[2]);
    var mapObj = getInternalMap(mapName);
    
    if (mapObj.hasOwnProperty(key)) {
        delete mapObj[key];
        saveInternalMap(mapName, mapObj);
        return true;
    } else {
        QuernAPI.Log("[Map Warning] Key '" + key + "' does not exist.");
        return false;
    }
});

QuernAPI.Register("MapHas", function(line, tokens) {
    if (tokens.length < 3) {
        QuernAPI.Log("[Map Error] Usage: MapHas \"MapName\" \"Key\"");
        return false;
    }
    
    var mapName = parseArg(tokens[1]);
    var key = parseArg(tokens[2]);
    var mapObj = getInternalMap(mapName);
    
    var exists = mapObj.hasOwnProperty(key);
    var resultStr = exists ? "True" : "False";
    
    if (QuernAPI.Runtime && QuernAPI.Runtime.ApiSetVariable) {
        QuernAPI.Runtime.ApiSetVariable("MapCheckResult", "String", resultStr);
    }
    return true;
});

QuernAPI.Register("MapMerge", function(line, tokens) {
    if (tokens.length < 3) {
        QuernAPI.Log("[Map Error] Usage: MapMerge \"TargetMap\" \"SourceMap\"");
        return false;
    }
    
    var targetName = parseArg(tokens[1]);
    var sourceName = parseArg(tokens[2]);
    
    var targetObj = getInternalMap(targetName);
    var sourceObj = getInternalMap(sourceName);
    
    for (var key in sourceObj) {
        if (sourceObj.hasOwnProperty(key)) {
            targetObj[key] = sourceObj[key];
        }
    }
    
    saveInternalMap(targetName, targetObj);
    return true;
});
QuernAPI.Register("MapGet", function(line, tokens) {
    if (tokens.length < 4) {
        QuernAPI.Log("[Map Error] Usage: MapGet \"MapName\" \"Key\" \"ResultVar\"");
        return false;
    }
    
    var mapName = parseArg(tokens[1]);
    var key = parseArg(tokens[2]);
    var varName = parseArg(tokens[3]);
    
    var mapObj = getInternalMap(mapName);
    
    if (mapObj.hasOwnProperty(key)) {
        var val = mapObj[key];
        var type = typeof val === 'number' ? "Number" : "String";
        var strVal = String(val);
        
        if (QuernAPI.Runtime && QuernAPI.Runtime.ApiSetVariable) {
            QuernAPI.Runtime.ApiSetVariable(varName, type, strVal);
            return true;
        }
    }
    
    QuernAPI.Log("[Map Warning] Key '" + key + "' not found or API error.");
    return false;
});
