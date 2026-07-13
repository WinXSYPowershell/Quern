// time.js
QuernAPI.Register("Time", function(line, tokens) {
    var now = new Date();
    QuernAPI.Log("" + now.toLocaleTimeString());
    return true;
});
QuernAPI.Register("Date", function(line, tokens) {
    var now = new Date();
    QuernAPI.Log("Current System Date: " + now.toLocaleDateString());
    return true;
});