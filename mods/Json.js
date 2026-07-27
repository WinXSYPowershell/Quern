// -----------------------------------------------------------------------
// <copyright file="main.cs" company="Quern Project">
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
// json.js
function getListItems(listName) {
    var quernList = QuernAPI.GetList(listName);
    var jsArray = [];
    
    if (quernList) {
        if (typeof quernList.length !== 'undefined') {
            for (var i = 0; i < quernList.length; i++) jsArray.push(quernList[i]);
        } else if (typeof quernList.Count !== 'undefined') {
            for (var i = 0; i < quernList.Count; i++) jsArray.push(quernList[i]);
        } else if (typeof quernList.GetEnumerator === 'function') {
             var enumerator = quernList.GetEnumerator();
             while (enumerator.MoveNext()) jsArray.push(enumerator.Current);
        }
    }
    return jsArray;
}

function toQuernString(val) {
    if (typeof val === 'object' && val !== null) {
        return '"' + JSON.stringify(val).replace(/"/g, '\\"') + '"';
    }
    return '"' + String(val).replace(/"/g, '\\"') + '"';
}

QuernAPI.Register("JsonPrint", function(line, tokens) {
    if (tokens.length < 2) {
        QuernAPI.Log("[JSON Error] Usage: JsonPrint L+ListName");
        return false;
    }
    
    var listRef = tokens[1];
    if (!listRef.startsWith("L+")) {
        QuernAPI.Log("[JSON Error] Invalid ref: " + listRef);
        return false;
    }
    
    var parts = listRef.split("+");
    var listName = parts[1];
    
    try {
        var items = getListItems(listName);
        var finalArray = items.map(function(item) {
            if (item !== null && !isNaN(item) && item.trim() !== "") return Number(item);
            return item;
        });
        
        QuernAPI.Log(JSON.stringify(finalArray, null, 2));
        return true;
    } catch (e) {
        QuernAPI.Log("[JSON Error] " + e.message);
        return false;
    }
});

QuernAPI.Register("JsonWriteVar", function(line, tokens) {
    if (tokens.length < 3) {
        QuernAPI.Log("[JSON Error] Usage: JsonWriteVar \"VarName\" L+ListName");
        return false;
    }
    
    var varName = tokens[1].replace(/^"|"$/g, '');
    var listRef = tokens[2];
    
    if (!listRef.startsWith("L+")) {
        QuernAPI.Log("[JSON Error] Invalid list ref: " + listRef);
        return false;
    }
    
    var parts = listRef.split("+");
    var listName = parts[1];
    
    try {
        var items = getListItems(listName);
        var finalArray = items.map(function(item) {
            if (item !== null && !isNaN(item) && item.trim() !== "") return Number(item);
            return item;
        });
        
        var jsonString = JSON.stringify(finalArray);
        
        // Modified: Use Runtime.ApiSetVariable
        if (QuernAPI.Runtime && QuernAPI.Runtime.ApiSetVariable) {
            QuernAPI.Runtime.ApiSetVariable(varName, "String", jsonString);
        } else {
            QuernAPI.Log("[JSON Error] Runtime API not available for setting variables.");
        }
        return true;
    } catch (e) {
        QuernAPI.Log("[JSON Error] " + e.message);
        return false;
    }
});

// QuernAPI.Register("JsonToList", function(line, tokens) {
//     if (tokens.length < 3) {
//         QuernAPI.Log("[JSON Error] Usage: JsonToList \"NewListName\" \"JsonString\"");
//         return false;
//     }
//     
//     var newListName = tokens[1].replace(/^"|"$/g, '');
//     var jsonSource = tokens.slice(2).join(" ");
//     
//     if (!jsonSource.includes("{") && !jsonSource.includes("[") && !jsonSource.startsWith("\"")) {
//         var varVal = QuernAPI.GetVariable(jsonSource);
//         if (varVal && varVal !== jsonSource) jsonSource = varVal;
//     } else if (jsonSource.startsWith("\"") && jsonSource.endsWith("\"")) {
//         jsonSource = jsonSource.substring(1, jsonSource.length - 1);
//     }
//     
//     try {
//         var parsedObj = JSON.parse(jsonSource);
//         var items = [];
//         
//         if (Array.isArray(parsedObj)) {
//             for (var i = 0; i < parsedObj.length; i++) {
//                 items.push(toQuernString(parsedObj[i]));
//             }
//         } else if (typeof parsedObj === 'object') {
//             items.push(toQuernString(parsedObj));
//         } else {
//             items.push(toQuernString(parsedObj));
//         }
//         
//         var definition = 'List = "' + newListName + '" = [' + items.join(", ") + ']';
//         
//         if (QuernAPI.ExecuteLine) {
//             QuernAPI.ExecuteLine(definition);
//         } else {
//             QuernAPI.Log("[JSON Warning] ExecuteLine API not available. Please copy this definition:");
//             QuernAPI.Log(definition);
//         }
//         
//         return true;
//     } catch (e) {
//         QuernAPI.Log("[JSON Error] Invalid JSON: " + e.message);
//         return false;
//     }
// });
