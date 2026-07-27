// Profiler.js
(function() {
    var _profilerData = {};
    var _callStack = [];
    var _isLoggingEnabled = false;
    var _isEnabled = false;
    
    function getTimestamp() {
        return Date.now();
    }

    function formatTime(ms) {
        if (ms < 1) return ms.toFixed(3) + "ms";
        return ms.toFixed(2) + "ms";
    }

    function startProfile(funcName) {
        if (!_isEnabled) return;

        var now = getTimestamp();
        
        if (!_profilerData[funcName]) {
            _profilerData[funcName] = {
                name: funcName,
                count: 0,
                totalTime: 0,
                minTime: Infinity,
                maxTime: 0,
                lastTime: 0
            };
        }

        _callStack.push({
            name: funcName,
            startTime: now
        });
    }

    function endProfile(funcName) {
        if (!_isEnabled || _callStack.length === 0) return;

        var now = getTimestamp();
        var stackFrame = _callStack.pop();
        
        if (stackFrame.name !== funcName) {
            if (_isLoggingEnabled) {
                Console.WriteLine("[Profiler Warning] Stack mismatch: expected " + funcName + " but found " + stackFrame.name);
            }
        }

        var duration = now - stackFrame.startTime;
        var data = _profilerData[funcName];

        if (data) {
            data.count++;
            data.totalTime += duration;
            data.lastTime = duration;
            if (duration < data.minTime) data.minTime = duration;
            if (duration > data.maxTime) data.maxTime = duration;
        }

        if (_isLoggingEnabled) {
            var indent = "  ".repeat(_callStack.length);
            Console.WriteLine(indent + "[Profiler] " + funcName + " took " + formatTime(duration));
        }
    }

    function generateReport() {
        var report = "=== Quern Profiler Report ===\n";
        report += "Generated at: " + new Date().toLocaleTimeString() + "\n";
        report += "--------------------------------\n";
        
        var sortedKeys = Object.keys(_profilerData).sort(function(a, b) {
            return _profilerData[b].totalTime - _profilerData[a].totalTime;
        });

        if (sortedKeys.length === 0) {
            report += "No data collected.\n";
            return report;
        }

        report += padRight("Function Name", 30) + 
                  padRight("Calls", 10) + 
                  padRight("Total", 12) + 
                  padRight("Avg", 12) + 
                  padRight("Min", 10) + 
                  padRight("Max", 10) + "\n";
        report += "--------------------------------\n";

        for (var i = 0; i < sortedKeys.length; i++) {
            var key = sortedKeys[i];
            var d = _profilerData[key];
            var avg = d.count > 0 ? d.totalTime / d.count : 0;
            
            report += padRight(d.name, 30) + 
                      padRight(d.count.toString(), 10) + 
                      padRight(formatTime(d.totalTime), 12) + 
                      padRight(formatTime(avg), 12) + 
                      padRight(formatTime(d.minTime === Infinity ? 0 : d.minTime), 10) + 
                      padRight(formatTime(d.maxTime), 10) + "\n";
        }
        
        report += "--------------------------------\n";
        report += "End of Report\n";
        return report;
    }

    function padRight(str, len) {
        str = String(str);
        while (str.length < len) {
            str += " ";
        }
        return str.substring(0, len);
    }

    QuernAPI.Register("DebugOn", function(line, tokens) {
        _isEnabled = true;
        _isLoggingEnabled = true;
        Console.WriteLine("[Profiler] Debugging ON. Real-time logging enabled.");
        return true;
    });

    QuernAPI.Register("DebugOff", function(line, tokens) {
        _isLoggingEnabled = false;
        Console.WriteLine("[Profiler] Real-time logging OFF. Data collection continues.");
        return true;
    });

    QuernAPI.Register("DebugStop", function(line, tokens) {
        _isEnabled = false;
        _isLoggingEnabled = false;
        Console.WriteLine("[Profiler] Profiler STOPPED.");
        return true;
    });

    QuernAPI.Register("DebugReset", function(line, tokens) {
        _profilerData = {};
        _callStack = [];
        Console.WriteLine("[Profiler] Data reset.");
        return true;
    });

    QuernAPI.Register("DebugReport", function(line, tokens) {
        var report = generateReport();
        var lines = report.split('\n');
        for (var i = 0; i < lines.length; i++) {
            if (lines[i].trim() !== "") {
                Console.WriteLine(lines[i]);
            }
        }
        return true;
    });

    QuernAPI.Register("Debug", function(line, tokens) {
        if (tokens.length >= 3 && tokens[1] === ">") {
            var varName = tokens[2];
            var report = generateReport();
            
            if (QuernAPI.Runtime && QuernAPI.Runtime.ApiSetVariable) {
                QuernAPI.Runtime.ApiSetVariable(varName, "String", report);
            } else {
                Console.WriteLine("[Profiler Error] Cannot save to variable: Runtime API missing.");
            }
            return true;
        }
        
        Console.WriteLine("[Profiler] Status: " + (_isEnabled ? "Active" : "Inactive") + 
                          ", Logging: " + (_isLoggingEnabled ? "ON" : "OFF") + 
                          ", Functions Tracked: " + Object.keys(_profilerData).length);
        return true;
    });

    var _originalRegister = QuernAPI.Register;

    QuernAPI.Register = function(pattern, handler) {
        var wrappedHandler = function(line, tokens) {
            if (!_isEnabled) {
                return handler(line, tokens);
            }

            startProfile(pattern);
            
            var result = false;
            try {
                result = handler(line, tokens);
            } catch (e) {
                endProfile(pattern);
                throw e;
            }
            
            endProfile(pattern);
            
            return result;
        };

        return _originalRegister(pattern, wrappedHandler);
    };
    this.ProfileStart = function(name) { startProfile(name); };
    this.ProfileEnd = function(name) { endProfile(name); };

})();
