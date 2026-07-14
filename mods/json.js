// json.js
// 增强版：支持写入变量和动态创建列表

// 辅助函数：安全地获取列表内容
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

// 辅助函数：将 JS 值转换为适合 Quern List 定义的字符串
function toQuernString(val) {
    if (typeof val === 'object' && val !== null) {
        return '"' + JSON.stringify(val).replace(/"/g, '\\"') + '"';
    }
    return '"' + String(val).replace(/"/g, '\\"') + '"';
}

// 1. JsonPrint: 打印列表为 JSON
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
        // 尝试转换数字类型
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

// 2. JsonWriteVar: 将列表转换为 JSON 字符串并写入变量
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
        
        // 检查是否有 SetVariable API
        if (QuernAPI.SetVariable) {
            QuernAPI.SetVariable(varName, "String", jsonString);
            QuernAPI.Log("[JSON] Written JSON to variable '" + varName + "'");
        } else {
            QuernAPI.Log("[JSON Warning] SetVariable API not available. Printing instead:");
            QuernAPI.Log("Set \"" + varName + "\" = \"" + jsonString.replace(/"/g, '\\"') + "\"");
        }
        return true;
    } catch (e) {
        QuernAPI.Log("[JSON Error] " + e.message);
        return false;
    }
});

// 3. JsonToList: 解析 JSON 并动态创建 Quern 列表
QuernAPI.Register("JsonToList", function(line, tokens) {
    if (tokens.length < 3) {
        QuernAPI.Log("[JSON Error] Usage: JsonToList \"NewListName\" \"JsonString\"");
        return false;
    }
    
    var newListName = tokens[1].replace(/^"|"$/g, '');
    var jsonSource = tokens.slice(2).join(" ");
    
    // 处理变量引用
    if (!jsonSource.includes("{") && !jsonSource.includes("[") && !jsonSource.startsWith("\"")) {
        var varVal = QuernAPI.GetVariable(jsonSource);
        if (varVal && varVal !== jsonSource) jsonSource = varVal;
    } else if (jsonSource.startsWith("\"") && jsonSource.endsWith("\"")) {
        jsonSource = jsonSource.substring(1, jsonSource.length - 1);
    }
    
    try {
        var parsedObj = JSON.parse(jsonSource);
        var items = [];
        
        if (Array.isArray(parsedObj)) {
            for (var i = 0; i < parsedObj.length; i++) {
                items.push(toQuernString(parsedObj[i]));
            }
        } else if (typeof parsedObj === 'object') {
            items.push(toQuernString(parsedObj));
        } else {
            items.push(toQuernString(parsedObj));
        }
        
        var definition = 'List = "' + newListName + '" = [' + items.join(", ") + ']';
        
        // 尝试动态执行
        if (QuernAPI.ExecuteLine) {
            QuernAPI.ExecuteLine(definition);
            QuernAPI.Log("[JSON] Created list '" + newListName + "' with " + items.length + " items.");
        } else {
            QuernAPI.Log("[JSON Warning] ExecuteLine API not available. Please copy this definition:");
            QuernAPI.Log(definition);
        }
        
        return true;
    } catch (e) {
        QuernAPI.Log("[JSON Error] Invalid JSON: " + e.message);
        return false;
    }
});