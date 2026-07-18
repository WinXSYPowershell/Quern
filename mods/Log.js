// Log.js
function getTimestamp() {
    var now = new Date();
    var pad = function(n) { return n < 10 ? '0' + n : n; };
    
    var year = now.getFullYear();
    var month = pad(now.getMonth() + 1);
    var day = pad(now.getDate());
    var hour = pad(now.getHours());
    var minute = pad(now.getMinutes());
    var second = pad(now.getSeconds());
    
    return "[" + year + "-" + month + "-" + day + " " + hour + ":" + minute + ":" + second + "]";
}

function normalizeLevel(level) {
    if (!level) return "INFO";
    var upper = level.toUpperCase();
    if (upper === "INFO" || upper === "WARN" || upper === "ERROR") {
        return upper;
    }
    return "INFO";
}

QuernAPI.Register("Log", function(line, tokens) {
    
    if (tokens.length < 3) {
        QuernAPI.Log("[Log Error] Usage: Log Level(<Lvl>) Content(\"<Msg>\") > <Action>(\"<Path>\")");
        return false;
    }

    try {
        var levelToken = tokens[1];
        var levelMatch = levelToken.match(/^Level\((.+)\)$/i);
        if (!levelMatch) {
            QuernAPI.Log("[Log Error] Invalid Level format. Use Level(INFO).");
            return false;
        }
        var level = normalizeLevel(levelMatch[1]);
        var timestamp = getTimestamp();
        var prefix = timestamp + "[" + level + "]";

        var arrowIndex = -1;
        for (var i = 2; i < tokens.length; i++) {
            if (tokens[i] === ">") {
                arrowIndex = i;
                break;
            }
        }

        if (arrowIndex === -1) {
            QuernAPI.Log("[Log Error] Missing output direction '>'. Use > CommandLine, > Save(...), or > Add(...)");
            return false;
        }
        
        var contentRawParts = tokens.slice(2, arrowIndex);
        var contentRawStr = contentRawParts.join(" ");
        
        var contentMatch = contentRawStr.match(/^Content\(["'](.*)["']\)$/);
        if (!contentMatch) {
             var looseMatch = contentRawStr.match(/^Content\((.*)\)$/);
             if (looseMatch) {
                 contentMatch = looseMatch;
             } else {
                 QuernAPI.Log("[Log Error] Invalid Content format. Use Content(\"Message\").");
                 return false;
             }
        }
        
        var message = contentMatch[1];
        var fullLogLine = prefix + message;
        if (arrowIndex + 1 >= tokens.length) {
            QuernAPI.Log("[Log Error] Missing action after '>'.");
            return false;
        }
        var actionParts = tokens.slice(arrowIndex + 1);
        var actionStr = actionParts.join(" ");
        if (actionStr === "CommandLine") {
            QuernAPI.Log(fullLogLine);
            return true;
        }
        
        var saveMatch = actionStr.match(/^Save\(["'](.+)["']\)$/);
        var addMatch = actionStr.match(/^Add\(["'](.+)["']\)$/);
        
        if (saveMatch) {
            var filePath = saveMatch[1];
            try {
                File.WriteAllText(filePath, fullLogLine);
            } catch (e) {
                QuernAPI.Log("[Log Error] Failed to write file: " + e.message);
                return false;
            }
            return true;
        }
        
        if (addMatch) {
            var filePath = addMatch[1];
            try {
                File.AppendAllText(filePath, fullLogLine + "\n");
            } catch (e) {
                QuernAPI.Log("[Log Error] Failed to append file: " + e.message);
                return false;
            }
            return true;
        }

        QuernAPI.Log("[Log Error] Unknown action: " + actionStr);
        return false;

    } catch (e) {
        QuernAPI.Log("[Log Error] Exception: " + e.message);
        return false;
    }
});
