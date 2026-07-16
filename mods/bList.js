// blist.js

QuernAPI.Register("ListSort", function(line, tokens) {
    if (tokens.length < 2) return false;
    
    var listName = tokens[1].replace(/"/g, '');
    var order = tokens[2] ? tokens[2].toUpperCase() : "ASC";
    
    var items = QuernAPI.GetList(listName);
    if (!items) {
        QuernAPI.Log("Error: List '" + listName + "' not found.");
        return true;
    }

    var sortedItems = items.slice();

    var allNumbers = sortedItems.every(function(item) {
        return !isNaN(parseFloat(item)) && isFinite(item);
    });

    sortedItems.sort(function(a, b) {
        if (allNumbers) {
            return order === "DESC" ? parseFloat(b) - parseFloat(a) : parseFloat(a) - parseFloat(b);
        } else {
            if (a < b) return order === "DESC" ? 1 : -1;
            if (a > b) return order === "DESC" ? -1 : 1;
            return 0;
        }
    });
    
    QuernAPI.Log("[" + listName + "]: " + JSON.stringify(sortedItems));
    
    return true;
});

QuernAPI.Register("ListFind", function(line, tokens) {
    if (tokens.length < 3) return false;
    
    var listName = tokens[1].replace(/"/g, '');
    var target = tokens[2].replace(/"/g, '');
    
    var items = QuernAPI.GetList(listName);
    if (!items) {
        QuernAPI.Log("-1");
        return true;
    }
    
    var index = items.indexOf(target);
    
    QuernAPI.Log(index.toString());
    return true;
});

QuernAPI.Register("ListJoin", function(line, tokens) {
    if (tokens.length < 4) return false;
    
    var list1Name = tokens[1].replace(/"/g, '');
    var list2Name = tokens[2].replace(/"/g, '');
    var newListName = tokens[3].replace(/"/g, '');
    
    var items1 = QuernAPI.GetList(list1Name);
    var items2 = QuernAPI.GetList(list2Name);
    
    if (!items1 || !items2) {
        QuernAPI.Log("Error: One of the lists not found.");
        return true;
    }
    
    var merged = items1.concat(items2);
    
    QuernAPI.Log("Merged Content (" + newListName + "): " + JSON.stringify(merged));
    return true;
});

QuernAPI.Register("ListReverse", function(line, tokens) {
    if (tokens.length < 2) return false;
    
    var listName = tokens[1].replace(/"/g, '');
    var items = QuernAPI.GetList(listName);
    
    if (!items) {
        QuernAPI.Log("Error: List '" + listName + "' not found.");
        return true;
    }
    
    var reversed = items.slice().reverse();
    QuernAPI.Log("[" + listName + "]: " + JSON.stringify(reversed));
    return true;
});

QuernAPI.Register("ListDistinct", function(line, tokens) {
    if (tokens.length < 2) return false;
    
    var listName = tokens[1].replace(/"/g, '');
    var items = QuernAPI.GetList(listName);
    
    if (!items) {
        QuernAPI.Log("Error: List '" + listName + "' not found.");
        return true;
    }
    
    var unique = Array.from(new Set(items));
    
    QuernAPI.Log("[" + listName + "]: " + JSON.stringify(unique));
    return true;
});
