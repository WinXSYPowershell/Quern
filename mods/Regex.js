// -----------------------------------------------------------------------
// <copyright file="Regex.js" company="Quern Project">
//     Copyright 2026 WinXSYPowershell
//
//     Licensed under the Apache License, Version 2.0 (the "License");
//     you may not use this file except in compliance with the License.
//     You may obtain a copy of the License at
//
//         http://www.apache.org/licenses/LICENSE-2.0
//
//     Unless required by applicable law or agreed to in writing, software
//     distributed under the License is distributed on an "AS IS" BASIS,
//     WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//     See the License for the specific language governing permissions and
//     limitations under the License.
// </copyright>
// -----------------------------------------------------------------------
// Regex.js

QuernAPI.Register("RegexString", function(line, tokens) {
    if (!line.trim().startsWith("RegexString")) {
        return false;
    }

        
        var sourceRaw = "";
        var pattern = "";
        var targetVarName = "";
        var firstOpen = line.indexOf('(');
        var firstClose = line.indexOf(')', firstOpen);
        
        if (firstOpen !== -1 && firstClose !== -1) {
            sourceRaw = line.substring(firstOpen + 1, firstClose).trim();
        } else {
            QuernAPI.Log("[Regex Error] Could not find Source argument in parentheses.");
            return true;
        }

        var regexKeywordIndex = line.indexOf("Regex");
        if (regexKeywordIndex === -1) {
             QuernAPI.Log("[Regex Error] Missing 'Regex' keyword.");
             return true;
        }

        var afterRegex = line.substring(regexKeywordIndex + 5); // skip "Regex"
        var quoteStart = afterRegex.indexOf('"');
        var quoteEnd = -1;
        
        if (quoteStart !== -1) {
            quoteEnd = afterRegex.indexOf('"', quoteStart + 1);
        }

        if (quoteStart !== -1 && quoteEnd !== -1) {
            pattern = afterRegex.substring(quoteStart + 1, quoteEnd);
        } else {
            QuernAPI.Log("[Regex Error] Pattern must be enclosed in double quotes after 'Regex'.");
            return true;
        }

        var arrowIndex = line.indexOf('>');
        if (arrowIndex === -1) {
            QuernAPI.Log("[Regex Error] Missing output redirect '>'");
            return true;
        }

        var afterArrow = line.substring(arrowIndex + 1);
        var targetOpen = afterArrow.indexOf('(');
        var targetClose = afterArrow.indexOf(')', targetOpen);

        if (targetOpen !== -1 && targetClose !== -1) {
            targetVarName = afterArrow.substring(targetOpen + 1, targetClose).trim();
        } else {
            QuernAPI.Log("[Regex Error] Could not find Target Variable in parentheses after '>'.");
            return true;
        }

        if (!sourceRaw || !pattern || !targetVarName) {
            QuernAPI.Log("[Regex Error] Failed to parse all arguments.");
            return true;
        }
        var inputValue = "";
        
        if (sourceRaw.startsWith("\"") && sourceRaw.endsWith("\"")) {
            inputValue = sourceRaw.substring(1, sourceRaw.length - 1);
        } else if (sourceRaw.startsWith("L+")) {
            var parts = sourceRaw.split("+");
            if (parts.length >= 2) {
                var listName = parts[1];
                var items = QuernAPI.GetList(listName);
                
                if (items && items.length > 0) {
                    var idx = 0;
                    if (parts.length === 3) {
                        idx = parseInt(parts[2]);
                    }
                    
                    if (!isNaN(idx) && idx >= 0 && idx < items.length) {
                        inputValue = items[idx];
                    } else {
                        QuernAPI.Log("[Regex Warning] List index out of range.");
                        return true;
                    }
                } else {
                    QuernAPI.Log("[Regex Warning] List '" + listName + "' is empty or not found.");
                    return true;
                }
            }
        } else {
            var varVal = QuernAPI.GetVariable(sourceRaw);
            if (varVal !== sourceRaw) {
                inputValue = varVal;
            } else {
                
                inputValue = varVal; 
            }
        }

        // --- 执行正则匹配 ---
try {
    var strInput = String(inputValue);
    var result = "";
    
    // 直接使用 JavaScript 的正则引擎
    var jsRegex = new RegExp(pattern);
    var jsMatch = jsRegex.exec(strInput);
    if (jsMatch) {
        result = jsMatch[0];
        QuernAPI.Log("[Regex Debug] Match found: '" + result + "'");
    } else {
        QuernAPI.Log("[Regex Debug] No match found.");
    }

    QuernAPI.Runtime.ApiSetVariable(targetVarName, "String", result ? result : "");
    return true;
} catch (e) {
    QuernAPI.Log("[Regex Error] Critical execution error: " + e.message);
    return true;
}

            QuernAPI.Runtime.ApiSetVariable(targetVarName, "String", result ? result : "");
            return true;
});