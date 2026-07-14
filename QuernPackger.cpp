using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.RegularExpressions;
using Jint;

namespace QuernEngine
{
    public class Variable
    {
        public string Name { get; set; }
        public string Type { get; set; }
        public string StrVal { get; set; }
        public int NumVal { get; set; }
        public bool Used { get; set; }
        public Variable() { Used = false; StrVal = ""; NumVal = 0; }
    }

    // New Data Structures for Lists
    public class QuernList
    {
        public string Name { get; set; }
        public List<string> Items { get; set; }
        
        public QuernList()
        {
            Items = new List<string>();
        }
    }

    public class FunctionDef
    {
        public string Name { get; set; }
        public string Body { get; set; }
        public bool IsMainFn { get; set; }
        public bool UsedInExecution { get; set; }
        public FunctionDef() { UsedInExecution = false; }
    }

    public class ModInfo
    {
        public string Name { get; set; }
        public string Version { get; set; }
        public string Author { get; set; }
    }

    public interface IModInterface
    {
        ModInfo GetInfo();
        void OnLoad(QuernRuntime runtime);
        void OnUnload();
    }

    public delegate bool SyntaxHandler(string line, List<string> tokens);

    public class ModManager
    {
        private static Dictionary<string, SyntaxHandler> _globalSyntaxMap = new Dictionary<string, SyntaxHandler>();
        private List<IModInterface> _loadedMods = new List<IModInterface>();
        private Engine _jsEngine;

        public ModManager() { _jsEngine = new Engine(); }

        public static bool RegisterSyntax(string pattern, SyntaxHandler handler)
        {
            if (string.IsNullOrEmpty(pattern) || handler == null) return false;
            _globalSyntaxMap[pattern] = handler;
            return true;
        }

        public static bool ProcessSyntax(string line, QuernRuntime runtimeContext)
        {
            if (string.IsNullOrEmpty(line)) return false;
            var tokens = SplitString(line, ' ');
            if (tokens.Count == 0) return false;
            string pattern = tokens[0];

            if (_globalSyntaxMap.TryGetValue(pattern, out var handler))
            {
                try
                {
                    bool result = handler(line, tokens);
                    if (result) Console.WriteLine($"");
                    return result;
                }
                catch (Exception ex)
                {
                    Console.WriteLine($"[MOD] Error in handler for '{pattern}': {ex.Message}");
                    return false;
                }
            }
            return false;
        }

        // Modified: Load specific JS mod by filename
        public void LoadJsMod(string filePath, QuernRuntime runtime)
        {
            if (!File.Exists(filePath))
            {
                Console.WriteLine($"[MOD] File not found: {filePath}");
                return;
            }

            try 
            {
                // 1. 读取 JS 脚本内容
                string scriptContent = File.ReadAllText(filePath);
                
                // 2. 【核心修改】直接将 C# 的原生命名空间暴露给 JS 环境
                _jsEngine.SetValue("File", typeof(File));
                _jsEngine.SetValue("Directory", typeof(Directory));
                _jsEngine.SetValue("Path", typeof(Path));
                _jsEngine.SetValue("Console", typeof(Console));
                
                // 3. 创建 API 包装器，保留原有的语法注册和 Runtime 功能
                var apiWrapper = new
                {
                    Register = new Action<string, Func<object, object, bool>>((pattern, jsHandler) =>
                    {
                        SyntaxHandler csharpHandler = (line, tokens) =>
                        {
                            object[] jsTokens = tokens.Cast<object>().ToArray();
                            try { return Convert.ToBoolean(jsHandler.Invoke(line, jsTokens)); }
                            catch (Exception ex) { Console.WriteLine($"[JS Error] in handler for '{pattern}': {ex.Message}"); return false; }
                        };
                        RegisterSyntax(pattern, csharpHandler);
                    }),
                    Log = new Action<string>(msg => Console.WriteLine($"{msg}")),
                    Runtime = runtime,
                    GetVariable = new Func<string, string>((name) => runtime.GetVariableString(name)),
                    GetList = new Func<string, List<string>>((name) => 
                    {
                        var l = runtime.GetList(name);
                        return l != null ? l.Items : new List<string>();
                    })
                };
        
                // 4. 将 API 注入 JS 引擎并执行脚本
                _jsEngine.SetValue("QuernAPI", apiWrapper);
                _jsEngine.Execute(scriptContent);
                Console.WriteLine($"[MOD] Loaded JS Module: {Path.GetFileName(filePath)}");
            }
            catch (Exception ex) { 
                Console.WriteLine($"[MOD] Failed to load {filePath}: {ex.Message}"); 
            }
        }

        public void UnloadAllMods()
        {
            _loadedMods.Clear();
            _globalSyntaxMap.Clear();
            // Note: Jint Engine doesn't have a simple "unload" for executed scripts, 
            // but clearing the syntax map stops them from being called.
            // For a full reset, one might need to recreate the Engine instance.
            _jsEngine = new Engine(); 
        }

        private static List<string> SplitString(string s, char delimiter)
        {
            var tokens = new List<string>();
            if (string.IsNullOrEmpty(s)) return tokens;
            var parts = s.Split(new[] { delimiter }, StringSplitOptions.RemoveEmptyEntries);
            tokens.AddRange(parts);
            return tokens;
        }
    }

    // --- Logic Structures ---

    public enum BlockType
    {
        If,
        ElseIf,
        Else,
        Cycle,
        Code
    }

    public class LogicBlock
    {
        public BlockType Type { get; set; }
        public string Condition { get; set; }
        public int CycleCount { get; set; }
        public List<string> BodyLines { get; set; }
        public List<LogicBlock> SubBlocks { get; set; }
        
        public LogicBlock()
        {
            BodyLines = new List<string>();
            SubBlocks = new List<LogicBlock>();
            CycleCount = 1;
        }
    }

    public class QuernRuntime
    {
        private const int MAX_VARIABLES = 100;
        private const int MAX_FUNCTIONS = 100;
        private const int MAX_LIST_ITEMS = 10000;

        private Variable[] _variables = new Variable[MAX_VARIABLES];
        private List<FunctionDef> _functions = new List<FunctionDef>();
        private Stack<string> _callStack = new Stack<string>();
        
        // New Storage
        private Dictionary<string, QuernList> _lists = new Dictionary<string, QuernList>();
        
        // New: Track requested imports for this execution context
        private List<string> _requestedJsImports = new List<string>();
        
        public bool DebugMode { get; set; } = false;
        
        // Track current execution context for error reporting
        private string _currentSourceFile = "Unknown";
        private int _currentLineNumber = 0;

        // Reference to ModManager to trigger specific loads
        private ModManager _modManager;

        public QuernRuntime(ModManager modManager = null)
        {
            _modManager = modManager;
            for (int i = 0; i < MAX_VARIABLES; i++) _variables[i] = new Variable();
        }

        // --- Error Reporting Helpers ---

        private void ReportError(string message)
        {
            Console.WriteLine($"ERROR!{message}:(Line{_currentLineNumber})");
        }

        private void ReportWarning(string message)
        {
            Console.WriteLine($"WARN!{message}:(Line{_currentLineNumber})");
        }

        // --- Variable Accessors ---

        private Variable FindVariable(string name)
        {
            return _variables.FirstOrDefault(v => v.Used && v.Name == name);
        }

        public int SetVariable(string name, string type, string value)
        {
            var var = FindVariable(name);
            if (var != null)
            {
                var.Type = type;
                if (type == "String") var.StrVal = value;
                else if (type == "Number" && int.TryParse(value, out int num)) var.NumVal = num;
                return 0;
            }
            for (int i = 0; i < MAX_VARIABLES; i++)
            {
                if (!_variables[i].Used)
                {
                    _variables[i].Name = name;
                    _variables[i].Type = type;
                    _variables[i].Used = true;
                    if (type == "String") _variables[i].StrVal = value;
                    else if (type == "Number" && int.TryParse(value, out int num)) _variables[i].NumVal = num;
                    return 0;
                }
            }
            ReportError("TooManyVariablesError");
            return -1;
        }

        public string GetVariableString(string name)
        {
            var var = FindVariable(name);
            if (var != null)
            {
                if (var.Type == "String") return var.StrVal;
                if (var.Type == "Number") return var.NumVal.ToString();
            }
            return name;
        }

        public int GetVariableNumber(string name)
        {
            var var = FindVariable(name);
            if (var != null)
            {
                if (var.Type == "Number") return var.NumVal;
                if (var.Type == "String") { int.TryParse(var.StrVal, out int res); return res; }
            }
            return 0;
        }

        // --- List Accessors (For Hooks and Internal Use) ---

        public QuernList GetList(string name)
        {
            _lists.TryGetValue(name, out var list);
            return list;
        }

        // --- Function Management ---

        private FunctionDef FindFunction(string name)
        {
            return _functions.FirstOrDefault(f => f.Name == name);
        }

        public void AddFunction(string name, string body, bool isMainFn)
        {
            if (_functions.Count >= MAX_FUNCTIONS)
            {
                ReportError("TooManyFunctionsError");
                return;
            }
            _functions.Add(new FunctionDef { Name = name, Body = body, IsMainFn = isMainFn, UsedInExecution = false });
        }

        private void MarkFunctionUsed(string name)
        {
            var func = FindFunction(name);
            if (func != null) func.UsedInExecution = true;
        }

        private bool IsInCallStack(string name)
        {
            return _callStack.Contains(name);
        }

        // --- Expression & Condition Evaluation ---

        private int EvaluateExpression(string expr)
        {
            expr = expr.Trim();
            if (string.IsNullOrEmpty(expr)) return 0;

            // Handle parentheses recursively
            if (expr.StartsWith("(") && expr.EndsWith(")"))
            {
                int depth = 0;
                bool match = true;
                for(int i=0; i<expr.Length; i++) {
                    if(expr[i]=='(') depth++;
                    if(expr[i]==')') depth--;
                    if(depth==0 && i < expr.Length-1) { match = false; break; }
                }
                if(match) return EvaluateExpression(expr.Substring(1, expr.Length-2));
            }

            // Check for List Reference: L+Name+Index
            if (expr.Contains("L+"))
            {
                 string resolvedVal = ResolveComplexReference(expr);
                 if(resolvedVal != null) {
                     if(int.TryParse(resolvedVal, out int rVal)) return rVal;
                     return 0;
                 }
            }

            // Operator precedence: We'll do a simple left-to-right scan for +, -, *, /, //, %
            // To keep it simple and robust for this engine, we look for the LAST occurrence of low-precedence ops (+, -)
            // then higher (*, /, //, %) if no +/- found.
            
            char op = '\0';
            int opIndex = -1;
            int opLength = 1; // Default length for single char ops
            
            int parenDepth = 0;
            
            // First pass: Look for + and - (lowest precedence)
            // We scan from right to left to handle left-associativity correctly if we were doing recursive descent, 
            // but here we just split once. Let's find the RIGHTMOST + or - outside parentheses.
            for (int i = expr.Length - 1; i >= 0; i--) 
            {
                char c = expr[i];
                if (c == ')') parenDepth++;
                else if (c == '(') parenDepth--;
                else if (parenDepth == 0)
                {
                    if (c == '+' || c == '-')
                    {
                        // Avoid treating L+ as operator
                        if (i > 0 && expr[i-1] == 'L') continue;
                        
                        // Avoid unary minus at start
                        if (c == '-' && i == 0) continue;
                        
                        op = c;
                        opIndex = i;
                        break;
                    }
                }
            }

            // If no + or -, look for *, /, //, %
            if (op == '\0')
            {
                parenDepth = 0;
                for (int i = expr.Length - 1; i >= 0; i--)
                {
                    char c = expr[i];
                    if (c == ')') parenDepth++;
                    else if (c == '(') parenDepth--;
                    else if (parenDepth == 0)
                    {
                        if (c == '*' || c == '%')
                        {
                            op = c;
                            opIndex = i;
                            break;
                        }
                        else if (c == '/')
                        {
                            // Check for //
                            if (i > 0 && expr[i-1] == '/')
                            {
                                op = '#'; // Use # to represent // internally
                                opIndex = i;
                                opLength = 2;
                                break;
                            }
                            else
                            {
                                op = '/';
                                opIndex = i;
                                break;
                            }
                        }
                    }
                }
            }

            if (op == '\0')
            {
                if (int.TryParse(expr, out int val)) return val;
                
                // Check if it's a variable name
                string varVal = GetVariableString(expr);
                if (varVal != expr) { // It was a variable
                     if(int.TryParse(varVal, out int vVal)) return vVal;
                     return 0;
                }
                
                // Check if it's a direct List ref like "L+MyList+0" without spaces
                string complexVal = ResolveComplexReference(expr);
                if(complexVal != null) {
                    if(int.TryParse(complexVal, out int cVal)) return cVal;
                    return 0;
                }

                return 0;
            }

            string leftStr = expr.Substring(0, opIndex);
            string rightStr = expr.Substring(opIndex + opLength);

            int leftVal = EvaluateExpression(leftStr);
            int rightVal = EvaluateExpression(rightStr);

            switch (op)
            {
                case '+': return leftVal + rightVal;
                case '-': return leftVal - rightVal;
                case '*': return leftVal * rightVal;
                case '/': 
                    if (rightVal == 0) { ReportWarning("DivisionByZeroWarning"); return 0; }
                    return leftVal / rightVal;
                case '#': // Integer Division //
                    if (rightVal == 0) { ReportWarning("DivisionByZeroWarning"); return 0; }
                    return leftVal / rightVal;
                case '%': 
                    if (rightVal == 0) { ReportWarning("DivisionByZeroWarning"); return 0; }
                    return leftVal % rightVal;
                default: return 0;
            }
        }

        private string ResolveComplexReference(string expr)
        {
            expr = expr.Trim();
            // Pattern: L+Name+Index
            if (expr.StartsWith("L+"))
            {
                var parts = expr.Split(new[] { '+' }, 3);
                if (parts.Length == 3)
                {
                    string listName = parts[1];
                    string indexStr = parts[2];
                    int index = EvaluateExpression(indexStr); // Index can be an expression
                    
                    var list = GetList(listName);
                    if (list != null)
                    {
                        if (index >= 0 && index < list.Items.Count)
                        {
                            return list.Items[index];
                        }
                        else
                        {
                            ReportWarning($"ListIndexOutOfRangeError List='{listName}' Index={index}");
                        }
                    }
                    else
                    {
                        ReportWarning($"ListNotFoundError Name='{listName}'");
                    }
                }
                else
                {
                    ReportWarning($"InvalidListReferenceFormatError Expr='{expr}'");
                }
            }
            return null;
        }

        private bool EvaluateSimpleCondition(string cond) {
            cond = cond.Trim();
            // Order matters: check two-char operators first
            string[] ops = { "<=", ">=", "==", "!=", "<", ">" };
            
            foreach (string op in ops) {
                int idx = cond.IndexOf(op);
                if (idx != -1) {
                    // Ensure we don't match part of a longer operator incorrectly, 
                    // though simple IndexOf is usually fine for these specific sets.
                    // For robustness, we assume standard formatting.
                    
                    string leftPart = cond.Substring(0, idx).Trim();
                    string rightPart = cond.Substring(idx + op.Length).Trim();

                    int? leftNumericVal = null;
                    int? rightNumericVal = null;

                    // --- 处理左侧值 ---
                    string leftResolved = ResolveComplexReference(leftPart);
                    if (leftResolved != null) {
                        if (int.TryParse(leftResolved, out int lVal)) {
                            leftNumericVal = lVal;
                        }
                    } else {
                        string leftVarVal = GetVariableString(leftPart);
                        if (leftVarVal != leftPart) {
                            if (int.TryParse(leftVarVal, out int lVal)) {
                                leftNumericVal = lVal;
                            }
                        }
                    }

                    // --- 处理右侧值 (逻辑同上) ---
                    string rightResolved = ResolveComplexReference(rightPart);
                    if (rightResolved != null) {
                        if (int.TryParse(rightResolved, out int rVal)) {
                            rightNumericVal = rVal;
                        }
                    } else {
                        string rightVarVal = GetVariableString(rightPart);
                        if (rightVarVal != rightPart) {
                            if (int.TryParse(rightVarVal, out int rVal)) {
                                rightNumericVal = rVal;
                            }
                        }
                    }

                    // --- 执行比较 ---
                    if (leftNumericVal.HasValue && rightNumericVal.HasValue) {
                        int l = leftNumericVal.Value;
                        int r = rightNumericVal.Value;
                        switch (op) {
                            case "<": return l < r;
                            case ">": return l > r;
                            case "<=": return l <= r;
                            case ">=": return l >= r;
                            case "==": return l == r;
                            case "!=": return l != r;
                        }
                    } else {
                        // Fallback to string comparison if not numeric
                        switch (op) {
                            case "==": return leftPart == rightPart;
                            case "!=": return leftPart != rightPart;
                            default:
                                ReportWarning($"NonNumericComparisonWarning Op='{op}' Left='{leftPart}' Right='{rightPart}'");
                                return false;
                        }
                    }
                }
            }
            // Fallback: Evaluate as expression (non-zero is true)
            int val = EvaluateExpression(cond);
            return val != 0;
        }

        private bool CheckCondition(string condition)
        {
            if (string.IsNullOrEmpty(condition)) return false;
            
            if (condition.Contains(",A,"))
            {
                var parts = condition.Split(new string[] { ",A," }, 2, StringSplitOptions.None);
                return CheckCondition(parts[0]) && CheckCondition(parts[1]);
            }
            if (condition.Contains(",O,"))
            {
                var parts = condition.Split(new string[] { ",O," }, 2, StringSplitOptions.None);
                return CheckCondition(parts[0]) || CheckCondition(parts[1]);
            }

            return EvaluateSimpleCondition(condition);
        }

        // --- REVISED ROBUST PARSER ---
        
        private class Statement { }
        
        private class CodeStatement : Statement 
        { 
            public string Line { get; set; } 
            public int LineNumber { get; set; } // Store original line number
        }
        
        private class IfStatement : Statement
        {
            public string Condition { get; set; }
            public int CycleCount { get; set; }
            public List<Statement> TrueBody { get; set; }
            public IfStatement ElseChain { get; set; }
            public int LineNumber { get; set; }
            
            public IfStatement() { TrueBody = new List<Statement>(); CycleCount = 1; }
        }
        
        private class CycleStatement : Statement
        {
            public int Count { get; set; }
            public List<Statement> Body { get; set; }
            public int LineNumber { get; set; }
            public CycleStatement() { Body = new List<Statement>(); }
        }

        private List<Statement> ParseStatements(List<string> lines, ref int index, bool stopAtElseOrEndBrace = false)
        {
            List<Statement> statements = new List<Statement>();

            while (index < lines.Count)
            {
                string rawLine = lines[index];
                string line = rawLine.Trim();
                int currentLineNum = index + 1; // 1-based line number

                if (string.IsNullOrEmpty(line))
                {
                    index++;
                    continue;
                }

                // Stop conditions
                if (line == "}")
                {
                    index++;
                    break;
                }
                
                if (stopAtElseOrEndBrace)
                {
                    if (line.StartsWith("Else:") || line.StartsWith("Else if"))
                    {
                        break;
                    }
                }

                // 1. If Statement
                Match ifMatch = Regex.Match(line, @"^If\s+\((.+?)\)\s*(?:cycle\s*\(\s*(\d+)\s*\))?\s*\{:?\s*$");
                if (ifMatch.Success)
                {
                    IfStatement ifStmt = new IfStatement();
                    ifStmt.LineNumber = currentLineNum;
                    ifStmt.Condition = ifMatch.Groups[1].Value.Trim();
                    
                    if (!string.IsNullOrEmpty(ifMatch.Groups[2].Value))
                    {
                        int tempCycle;
                        if (int.TryParse(ifMatch.Groups[2].Value, out tempCycle))
                        {
                            ifStmt.CycleCount = tempCycle;
                        }
                    }

                    index++; 
                    ifStmt.TrueBody = ParseStatements(lines, ref index, true);
                    
                    if (index < lines.Count)
                    {
                        string nextLine = lines[index].Trim();
                        if (nextLine.StartsWith("Else if"))
                        {
                            Match elseIfMatch = Regex.Match(nextLine, @"^Else\s+if\s+\((.+?)\)\s*(?:cycle\s*\(\s*(\d+)\s*\))?\s*\{:?\s*$");
                            if (elseIfMatch.Success)
                            {
                                IfStatement elseIfStmt = new IfStatement();
                                elseIfStmt.LineNumber = index + 1;
                                elseIfStmt.Condition = elseIfMatch.Groups[1].Value.Trim();
                                
                                if (!string.IsNullOrEmpty(elseIfMatch.Groups[2].Value))
                                {
                                    int tempCycle;
                                    if (int.TryParse(elseIfMatch.Groups[2].Value, out tempCycle))
                                    {
                                        elseIfStmt.CycleCount = tempCycle;
                                    }
                                }
                                
                                index++; 
                                elseIfStmt.TrueBody = ParseStatements(lines, ref index, true);
                                 
                                if (index < lines.Count)
                                {
                                     string subsequentLine = lines[index].Trim();
                                     if (subsequentLine.StartsWith("Else:"))
                                     {
                                         IfStatement pureElseStmt = new IfStatement();
                                         pureElseStmt.LineNumber = index + 1;
                                         pureElseStmt.Condition = "TRUE";
                                         
                                         Match elseCycleMatch = Regex.Match(subsequentLine, @"^Else:\s*cycle\s*\(\s*(\d+)\s*\)\s*\{:?\s*$");
                                         if(elseCycleMatch.Success) {
                                             int tempCycle;
                                             if(int.TryParse(elseCycleMatch.Groups[1].Value, out tempCycle)) {
                                                 pureElseStmt.CycleCount = tempCycle;
                                             }
                                         }
                                         
                                         index++; 
                                         pureElseStmt.TrueBody = ParseStatements(lines, ref index, false);
                                         elseIfStmt.ElseChain = pureElseStmt;
                                     }
                                }
                                
                                ifStmt.ElseChain = elseIfStmt;
                            }
                        }
                        else if (nextLine.StartsWith("Else:"))
                        {
                            IfStatement elseStmt = new IfStatement();
                            elseStmt.LineNumber = index + 1;
                            elseStmt.Condition = "TRUE";
                            
                            Match elseCycleMatch = Regex.Match(nextLine, @"^Else:\s*cycle\s*\(\s*(\d+)\s*\)\s*\{:?\s*$");
                            if(elseCycleMatch.Success) {
                                int tempCycle;
                                if(int.TryParse(elseCycleMatch.Groups[1].Value, out tempCycle)) {
                                    elseStmt.CycleCount = tempCycle;
                                }
                            }
                            
                            index++; 
                            elseStmt.TrueBody = ParseStatements(lines, ref index, false);
                            ifStmt.ElseChain = elseStmt;
                        }
                    }
                    
                    statements.Add(ifStmt);
                    continue;
                }

                // 2. Standalone Cycle
                Match cycMatch = Regex.Match(line, @"^Cycle\s*\(\s*(\d+)\s*\)\s*\{:?\s*$");
                if (cycMatch.Success)
                {
                    CycleStatement cycStmt = new CycleStatement();
                    cycStmt.LineNumber = currentLineNum;
                    int tempCount;
                    if(int.TryParse(cycMatch.Groups[1].Value, out tempCount)) {
                        cycStmt.Count = tempCount;
                    }
                    
                    index++; 
                    cycStmt.Body = ParseStatements(lines, ref index, false);
                    statements.Add(cycStmt);
                    continue;
                }
                
                // 3. Code Line
                statements.Add(new CodeStatement { Line = rawLine, LineNumber = currentLineNum });
                index++;
            }
            
            return statements;
        }

        private void ExecuteStatements(List<Statement> statements)
        {
            if (statements == null) return;

            foreach (var stmt in statements)
            {
                // Update current line context before execution
                if (stmt is CodeStatement cs) _currentLineNumber = cs.LineNumber;
                else if (stmt is IfStatement ifs) _currentLineNumber = ifs.LineNumber;
                else if (stmt is CycleStatement cycs) _currentLineNumber = cycs.LineNumber;

                if (stmt is CodeStatement codeStmt)
                {
                    ExecuteSingleLine(codeStmt.Line);
                }
                else if (stmt is IfStatement ifStmt)
                {
                    ExecuteIfStatement(ifStmt);
                }
                else if (stmt is CycleStatement cycStmt)
                {
                    ExecuteCycleStatement(cycStmt);
                }
            }
        }

        private void ExecuteIfStatement(IfStatement ifStmt)
        {
            bool conditionMet = CheckCondition(ifStmt.Condition);
            
            if (DebugMode) Console.WriteLine($"[If] Condition '{ifStmt.Condition}' is {(conditionMet ? "TRUE" : "FALSE")}");

            if (conditionMet)
            {
                int loops = ifStmt.CycleCount;
                for (int i = 0; i < loops; i++)
                {
                    if (DebugMode && loops > 1) Console.WriteLine($"[If-Cycle] Iteration {i+1}/{loops}");
                    ExecuteStatements(ifStmt.TrueBody);
                }
            }
            else
            {
                if (ifStmt.ElseChain != null)
                {
                    ExecuteIfStatement(ifStmt.ElseChain);
                }
            }
        }
        
        private void ExecuteCycleStatement(CycleStatement cycStmt)
        {
            for (int i = 0; i < cycStmt.Count; i++)
            {
                if (DebugMode) Console.WriteLine($"[Cycle] Iteration {i+1}/{cycStmt.Count}");
                ExecuteStatements(cycStmt.Body);
            }
        }

        // --- NEW: List Parsing Helpers (Dict Removed) ---

        private void ParseAndRegisterList(string name, string content)
        {
            // content is inside [ ... ]
            var list = new QuernList { Name = name };
            
            // Simple split by comma, handling quotes
            List<string> items = new List<string>();
            string currentToken = "";
            bool inQuotes = false;
            
            for (int i = 0; i < content.Length; i++)
            {
                char c = content[i];
                if (c == '"')
                {
                    inQuotes = !inQuotes;
                    currentToken += c;
                }
                else if (c == ',' && !inQuotes)
                {
                    items.Add(currentToken.Trim());
                    currentToken = "";
                }
                else
                {
                    currentToken += c;
                }
            }
            if (!string.IsNullOrEmpty(currentToken)) items.Add(currentToken.Trim());

            // Process items: Resolve variables if they are not quoted strings
            foreach (var item in items)
            {
                string cleanItem = item.Trim();
                if (cleanItem.StartsWith("\"") && cleanItem.EndsWith("\""))
                {
                    // String literal
                    list.Items.Add(cleanItem.Substring(1, cleanItem.Length - 2));
                }
                else if (cleanItem.StartsWith("L+"))
                {
                    // Complex Reference in List Definition
                    string resolved = ResolveComplexReference(cleanItem);
                    if (resolved != null) list.Items.Add(resolved);
                    else list.Items.Add(cleanItem); // Keep original if resolution fails
                }
                else
                {
                    // Variable or Number
                    Variable v = FindVariable(cleanItem);
                    if (v != null)
                    {
                        if (v.Type == "String") list.Items.Add(v.StrVal);
                        else list.Items.Add(v.NumVal.ToString());
                    }
                    else if (int.TryParse(cleanItem, out int num))
                    {
                        list.Items.Add(num.ToString());
                    }
                    else
                    {
                        list.Items.Add(cleanItem); // Keep as is if unknown
                    }
                }
            }

            if (list.Items.Count > MAX_LIST_ITEMS)
            {
                ReportWarning($"ListMaxItemsExceededWarning Name='{name}' Max={MAX_LIST_ITEMS}");
                list.Items = list.Items.Take(MAX_LIST_ITEMS).ToList();
            }

            _lists[name] = list;
            if (DebugMode) Console.WriteLine($"[List] Registered '{name}' with {list.Items.Count} items.");
        }

        private void ExecuteSingleLine(string line)
        {
            string trimmed = line.Trim();
            if (string.IsNullOrEmpty(trimmed)) return;

            if (ModManager.ProcessSyntax(trimmed, this))
            {
                return;
            }

            // --- NEW: List Operations ---
            // Init: List = "Name" = ["item", ...]
            Match listInitMatch = Regex.Match(trimmed, @"^List\s*=\s*""([^""]+)""\s*=\s*\[(.*)\]\s*$");
            if (listInitMatch.Success)
            {
                ParseAndRegisterList(listInitMatch.Groups[1].Value, listInitMatch.Groups[2].Value);
                return;
            }

            // Op: List = "Name" Add Index "Value"/Var
            Match listAddMatch = Regex.Match(trimmed, @"^List\s*=\s*""([^""]+)""\s*Add\s+(\d+)\s+(.+)$");
            if (listAddMatch.Success)
            {
                string name = listAddMatch.Groups[1].Value;
                // Index is ignored for Add (Append), but parsed for consistency
                string valRaw = listAddMatch.Groups[3].Value.Trim();
                string val = ResolveValue(valRaw);

                var list = GetList(name);
                if (list != null)
                {
                    if (list.Items.Count < MAX_LIST_ITEMS)
                    {
                        list.Items.Add(val);
                    }
                    else
                    {
                        ReportError($"ListFullError Name='{name}'");
                    }
                }
                else
                {
                    ReportError($"ListNotFoundError Name='{name}'");
                }
                return;
            }

            // Op: List = "Name" Delete Index
            Match listDelMatch = Regex.Match(trimmed, @"^List\s*=\s*""([^""]+)""\s*Delete\s+(\d+)\s*$");
            if (listDelMatch.Success)
            {
                string name = listDelMatch.Groups[1].Value;
                int index = int.Parse(listDelMatch.Groups[2].Value);
                
                var list = GetList(name);
                if (list != null && index >= 0 && index < list.Items.Count)
                {
                    list.Items.RemoveAt(index);
                }
                else
                {
                    ReportError($"ListDeleteError Name='{name}' Index={index}");
                }
                return;
            }

            // Op: List = "Name" Replace Index "Value"/Var
            Match listRepMatch = Regex.Match(trimmed, @"^List\s*=\s*""([^""]+)""\s*Replace\s+(\d+)\s+(.+)$");
            if (listRepMatch.Success)
            {
                string name = listRepMatch.Groups[1].Value;
                int index = int.Parse(listRepMatch.Groups[2].Value);
                string valRaw = listRepMatch.Groups[3].Value.Trim();
                string val = ResolveValue(valRaw);

                var list = GetList(name);
                if (list != null && index >= 0 && index < list.Items.Count)
                {
                    list.Items[index] = val;
                }
                else
                {
                    ReportError($"ListReplaceError Name='{name}' Index={index}");
                }
                return;
            }

            // --- Enhanced Set Logic ---
            
            // Pattern 1: Compound Assignment (e.g., Set "Var" += 1)
            // Matches: Set "Name" OP Value
            Match compoundSetMatch = Regex.Match(trimmed, @"^Set\s+""([^""]+)""\s*(\+|-|\*|/|//|%)=\s*(.+)$");
            if (compoundSetMatch.Success)
            {
                string varName = compoundSetMatch.Groups[1].Value;
                string op = compoundSetMatch.Groups[2].Value;
                string valRaw = compoundSetMatch.Groups[3].Value.Trim();
                
                Variable currentVar = FindVariable(varName);
                if (currentVar == null)
                {
                    ReportError($"UndefinedVariableError Name='{varName}'");
                    return;
                }

                int currentVal = GetVariableNumber(varName);
                int rightVal = EvaluateExpression(valRaw); // Evaluate expression including L+ refs
                
                int result = 0;
                switch (op)
                {
                    case "+": result = currentVal + rightVal; break;
                    case "-": result = currentVal - rightVal; break;
                    case "*": result = currentVal * rightVal; break;
                    case "/": 
                        if (rightVal == 0) { ReportWarning("DivisionByZeroWarning"); result = 0; }
                        else result = currentVal / rightVal; 
                        break;
                    case "//": 
                        if (rightVal == 0) { ReportWarning("DivisionByZeroWarning"); result = 0; }
                        else result = currentVal / rightVal; 
                        break;
                    case "%": 
                        if (rightVal == 0) { ReportWarning("DivisionByZeroWarning"); result = 0; }
                        else result = currentVal % rightVal; 
                        break;
                }
                
                // Update variable as Number
                SetVariable(varName, "Number", result.ToString());
                return;
            }

            // Pattern 2: Standard Set with optional Type
            // Matches: Set [Type] "Name" = Value
            // Type is optional. If present, it's String or Number.
            // Fixed Regex: Removed extra spaces in group definition
            Match stdSetMatch = Regex.Match(trimmed, @"^Set\s+(?:(String|Number)\s+)?""([^""]+)""\s*=\s*(.+)$");
            if (stdSetMatch.Success)
            {
                string type = stdSetMatch.Groups[1].Value; // Can be empty
                string varName = stdSetMatch.Groups[2].Value;
                string valRaw = stdSetMatch.Groups[3].Value.Trim();

                // Determine effective type
                string effectiveType = type;
                if (string.IsNullOrEmpty(effectiveType))
                {
                    // If type not specified, check if var exists. If so, keep type. If not, default to String? 
                    // Or try to infer. Let's default to String for safety unless it looks like a number expression?
                    // The prompt implies Set "Var" = "Content" (String) or Set "Var" = 1+1 (Number).
                    // Let's check if the raw value is wrapped in quotes.
                    if (valRaw.StartsWith("\"") && valRaw.EndsWith("\""))
                    {
                        effectiveType = "String";
                    }
                    else
                    {
                        effectiveType = "Number"; // Assume number if expression or bare word
                    }
                    
                    // If variable already exists, preserve its type unless explicitly overridden?
                    // For simplicity, if type is omitted, we update the value keeping existing type if possible, 
                    // or use inferred type if new.
                    Variable existing = FindVariable(varName);
                    if (existing != null)
                    {
                        effectiveType = existing.Type;
                    }
                }

                string finalValue = "";
                
                if (effectiveType == "String")
                {
                    // If it's a string assignment, we expect quotes.
                    if (valRaw.StartsWith("\"") && valRaw.EndsWith("\""))
                    {
                        finalValue = valRaw.Substring(1, valRaw.Length - 2);
                    }
                    else
                    {
                        // Maybe it's a variable reference being assigned to a string var?
                        // Or just a raw string without quotes (less likely per spec but possible)
                        // Let's resolve it as a value just in case.
                        finalValue = ResolveValue(valRaw);
                    }
                }
                else // Number
                {
                    // Evaluate the expression
                    int calcVal = EvaluateExpression(valRaw);
                    finalValue = calcVal.ToString();
                }

                SetVariable(varName, effectiveType, finalValue);
                return;
            }


            // --- Existing Logic ---

            if (trimmed.StartsWith("Print ("))
            {
                // Enforce "> CommandLine"
                if (!trimmed.Contains("> CommandLine"))
                {
                    ReportError("MissingCommandLineTagError");
                    return;
                }

                string contentToPrint = null;
                int cmdIndex = trimmed.IndexOf("> CommandLine");
                string relevantPart = trimmed.Substring(0, cmdIndex);

                int startParen = relevantPart.IndexOf('(');
                int endParen = relevantPart.LastIndexOf(')');

                if (startParen >= 0 && endParen > startParen)
                {
                    string inner = relevantPart.Substring(startParen + 1, endParen - startParen - 1).Trim();
                    
                    // Check for List Print Syntax: L+Name+Index
                    if (inner.StartsWith("L+"))
                    {
                        string resolved = ResolveComplexReference(inner);
                        if (resolved != null) 
                        {
                            contentToPrint = resolved;
                        }
                        else
                        {
                            ReportError($"UnresolvedPrintReferenceError Ref='{inner}'");
                            contentToPrint = $"[Unresolved: {inner}]";
                        }
                    }
                    else if (inner.StartsWith("\"") && inner.EndsWith("\""))
                    {
                        contentToPrint = inner.Substring(1, inner.Length - 2);
                    }
                    else
                    {
                        Variable v = FindVariable(inner);
                        if (v != null) contentToPrint = GetVariableString(inner);
                        else 
                        {
                            // Could be a literal number or undefined variable
                            if(int.TryParse(inner, out int litNum)) contentToPrint = inner;
                            else ReportWarning($"UndefinedVariableInPrintWarning Var='{inner}'");
                            contentToPrint = inner;
                        }
                    }
                }
                else
                {
                    ReportError("MalformedPrintStatementError");
                }

                if (contentToPrint != null) Console.WriteLine(contentToPrint);
            }
            else if (trimmed.StartsWith("Return "))
            {
                 if (DebugMode) Console.WriteLine($"[Return] {trimmed.Substring(7).Trim()}");
            }
            else
            {
                var match = Regex.Match(trimmed, @"^(\w+)\(\)$");
                if (match.Success)
                {
                    string funcName = match.Groups[1].Value;
                    FunctionDef func = FindFunction(funcName);
                    if (func != null)
                    {
                        if (IsInCallStack(funcName))
                        {
                            ReportError($"RecursiveCallDetectedError Func='{funcName}'");
                        }
                        else
                        {
                            if (DebugMode) Console.WriteLine($"Calling function: {funcName}");
                            MarkFunctionUsed(funcName);
                            _callStack.Push(funcName);
                            
                            var funcLines = Regex.Split(func.Body, @"\r\n|\n").ToList();
                            int idx = 0;
                            var stmts = ParseStatements(funcLines, ref idx, false);
                            ExecuteStatements(stmts);
                            
                            _callStack.Pop();
                        }
                    }
                    else
                    {
                        ReportError($"FunctionNotFoundError Name='{funcName}'");
                    }
                }
                else if (!string.IsNullOrEmpty(trimmed))
                {
                    // If it doesn't match any known command structure
                    ReportWarning($"UnknownCommandOrSyntaxWarning Line='{trimmed}'");
                }
            }
        }

        private string ResolveValue(string raw)
        {
            if (raw.StartsWith("\"") && raw.EndsWith("\""))
            {
                return raw.Substring(1, raw.Length - 2);
            }
            
            // Check for Complex References in values (e.g. Add 0 L+Other+1)
            if (raw.StartsWith("L+"))
            {
                string resolved = ResolveComplexReference(raw);
                if (resolved != null) return resolved;
            }

            Variable v = FindVariable(raw);
            if (v != null)
            {
                if (v.Type == "String") return v.StrVal;
                return v.NumVal.ToString();
            }
            return raw;
        }

        public void ExecuteFunctionBody(string body)
        {
            if (string.IsNullOrEmpty(body)) return;
            
            // Pre-process to handle multi-line List definitions AND Comments
            string processedBody = PreProcessCode(body);

            var lines = Regex.Split(processedBody, @"\r\n|\n").ToList();
            int index = 0;
            var statements = ParseStatements(lines, ref index, false);
            ExecuteStatements(statements);
        }

        /// <summary>
        /// Pre-processes code to handle:
        /// 1. Multi-line List definitions (joining lines)
        /// 2. Single-line comments (#)
        /// 3. Multi-line comments (### ... ###)
        /// </summary>
        private string PreProcessCode(string body)
        {
            var lines = body.Split(new[] { "\r\n", "\n" }, StringSplitOptions.None).ToList();
            var result = new List<string>();
            string buffer = null;
            bool inDefinition = false;
            bool inMultiLineComment = false;

            foreach (var line in lines)
            {
                string trimmed = line.Trim();

                // 1. Handle Multi-line Comments (### ... ###)
                if (inMultiLineComment)
                {
                    if (trimmed.Contains("###"))
                    {
                        // End of multi-line comment. 
                        // Note: If there is code after ### on the same line, it would need more complex parsing.
                        // For simplicity, we assume ### ends the comment block.
                        inMultiLineComment = false;
                        
                        // Check if there is content after the closing ###
                        int endIdx = trimmed.IndexOf("###");
                        if (endIdx + 3 < trimmed.Length)
                        {
                            string remaining = trimmed.Substring(endIdx + 3).Trim();
                            if (!string.IsNullOrEmpty(remaining))
                            {
                                // Process the remaining part as normal code (could be start of definition or code)
                                // We fall through to the normal processing logic below, but we need to treat 'remaining' as the line.
                                // To keep it simple, we'll just add it to result if it's not empty, or handle definition logic.
                                // However, mixing comment end and code start on same line is rare in this simple parser.
                                // Let's just ignore rest of line for safety or process it.
                                // For robustness, let's process 'remaining' as a new line.
                                ProcessLineForDefinition(remaining, ref inDefinition, ref buffer, result);
                            }
                        }
                    }
                    // Skip everything inside multi-line comment
                    continue;
                }
                else
                {
                    // Check for start of multi-line comment
                    if (trimmed.StartsWith("###"))
                    {
                        inMultiLineComment = true;
                        // Check if it also closes on the same line: ### comment ###
                        if (trimmed.Length > 3 && trimmed.Substring(3).Contains("###"))
                        {
                            inMultiLineComment = false;
                            // Process rest of line if needed, but usually ### starts a block.
                        }
                        continue;
                    }
                }

                // 2. Handle Single-line Comments (#)
                // If line contains #, truncate everything after it.
                // Be careful not to truncate if # is inside a string? 
                // The spec says "use # as comment", implying simple truncation.
                if (trimmed.Contains("#"))
                {
                    int hashIndex = trimmed.IndexOf('#');
                    trimmed = trimmed.Substring(0, hashIndex).Trim();
                }

                if (string.IsNullOrEmpty(trimmed))
                {
                    continue;
                }

                // 3. Handle Multi-line Definitions (List = ... [ ... ])
                ProcessLineForDefinition(trimmed, ref inDefinition, ref buffer, result);
            }

            if (buffer != null) result.Add(buffer);

            return string.Join(Environment.NewLine, result);
        }

        private void ProcessLineForDefinition(string trimmed, ref bool inDefinition, ref string buffer, List<string> result)
        {
            if (inDefinition)
            {
                buffer += " " + trimmed;
                if (trimmed.Contains("]"))
                {
                    inDefinition = false;
                    result.Add(buffer);
                    buffer = null;
                }
            }
            else
            {
                // Check for start of multi-line definition: List = "Name" = [ ... (no closing ])
                if (trimmed.StartsWith("List") && trimmed.Contains("=") && trimmed.Contains("[") && !trimmed.Contains("]"))
                {
                    inDefinition = true;
                    buffer = trimmed;
                }
                else
                {
                    result.Add(trimmed);
                }
            }
        }

        // --- NEW: Import and Include Handling ---

        private string ResolveIncludedCode(string code, string baseDirectory, HashSet<string> includedFiles)
        {
            // Regex to find Include "filename.q"
            // We process includes recursively.
            var includeRegex = new Regex(@"^Include\s+""([^""]+)""\s*$", RegexOptions.Multiline);
            
            // We need to loop because replacing text changes indices/structure, 
            // but simpler is to split by lines and process.
            // However, regex replace is easier if we do it carefully.
            // Let's use a MatchEvaluator.

            string result = includeRegex.Replace(code, match =>
            {
                string fileName = match.Groups[1].Value;
                string fullPath = Path.Combine(baseDirectory, fileName);
                
                // Normalize path to prevent duplicates via different paths (e.g. ./a.q vs a.q)
                string normalizedPath = Path.GetFullPath(fullPath);

                if (includedFiles.Contains(normalizedPath))
                {
                    Console.WriteLine($"[WARN] Circular Include Detected or Duplicate: {fileName}");
                    return ""; // Return empty to avoid infinite recursion/duplication
                }

                if (!File.Exists(fullPath))
                {
                    Console.WriteLine($"[ERROR] Include file not found: {fullPath}");
                    return "";
                }

                includedFiles.Add(normalizedPath);
                string content = File.ReadAllText(fullPath);
                
                // Recursively resolve includes in the included file
                // The base directory for the included file should be its own directory
                string subDir = Path.GetDirectoryName(fullPath);
                return ResolveIncludedCode(content, subDir, includedFiles);
            });

            return result;
        }

        public int ParseAndExecuteCode(string codeContent, string sourceFile = "Unknown")
        {
            _currentSourceFile = sourceFile;
            
            // Reset state
            foreach (var v in _variables) v.Used = false;
            foreach (var f in _functions) f.UsedInExecution = false;
            _functions.Clear();
            _lists.Clear();
            _requestedJsImports.Clear();

            string baseDirectory = Path.GetDirectoryName(sourceFile);
            if (string.IsNullOrEmpty(baseDirectory)) baseDirectory = Directory.GetCurrentDirectory();

            // 1. Handle Includes (Pre-processing)
            // We use a HashSet to track included files to prevent circular dependencies
            HashSet<string> includedFiles = new HashSet<string>();
            // Add the main file to the set so it doesn't include itself if referenced relatively
            includedFiles.Add(Path.GetFullPath(sourceFile));
            
            string expandedCode = ResolveIncludedCode(codeContent, baseDirectory, includedFiles);

            // 2. Handle Imports (Scan for Import statements)
            // Import "file.js"
            var importRegex = new Regex(@"^Import\s+""([^""]+)""\s*$", RegexOptions.Multiline);
            var importMatches = importRegex.Matches(expandedCode);
            
            foreach (Match match in importMatches)
            {
                string jsFile = match.Groups[1].Value;
                _requestedJsImports.Add(jsFile);
            }

            // Remove Import lines from code so they don't cause syntax errors during parsing
            string cleanCode = importRegex.Replace(expandedCode, "");

            // 3. Load Requested JS Mods
            if (_modManager != null && _requestedJsImports.Any())
            {
                string modsDir = Path.Combine(baseDirectory, "mods");
                // Also check current directory if mods folder doesn't exist? 
                // Spec says "mods下的.js文件", so we stick to mods folder relative to script or exe?
                // Usually relative to script is better for portability, but let's check script dir/mods first.
                if (!Directory.Exists(modsDir))
                {
                     // Fallback to exe directory mods folder if script dir doesn't have it
                     modsDir = Path.Combine(Directory.GetCurrentDirectory(), "mods");
                }

                foreach (var jsFile in _requestedJsImports.Distinct())
                {
                    string fullPath = Path.Combine(modsDir, jsFile);
                    _modManager.LoadJsMod(fullPath, this);
                }
            }

            // 4. Parse Functions from Cleaned Code
            var fnRegex = new Regex(@"Fn\s+""([^""]+)""\s*\(([^)]*)\)\s*\{([\s\S]*?)\}", RegexOptions.Singleline);
            var matches = fnRegex.Matches(cleanCode);

            if (matches.Count == 0 && !string.IsNullOrEmpty(cleanCode.Trim()))
            {
                 // If there is code but no functions defined at all
                 ReportError("NoFunctionsDefinedError");
                 return -1;
            }

            foreach (Match match in matches)
            {
                string name = match.Groups[1].Value;
                string paramsStr = match.Groups[2].Value;
                string body = match.Groups[3].Value;
                
                bool isMain = paramsStr.Contains("MainFn");
                AddFunction(name, body, isMain);
            }

            var mainFunc = _functions.FirstOrDefault(f => f.IsMainFn);
            if (mainFunc != null)
            {
                if (DebugMode) Console.WriteLine($"Main function: {mainFunc.Name}");
                MarkFunctionUsed(mainFunc.Name);
                _callStack.Push(mainFunc.Name);
                
                ExecuteFunctionBody(mainFunc.Body);
                
                _callStack.Pop();

                if (DebugMode)
                {
                    ;
                    foreach (var f in _functions)
                    {
                        ;
                    }
                }
                return 0;
            }
            else
            {
                ReportError("MissingMainFunctionError");
                return -1;
            }
        }

        public int ExecuteFile(string filename)
        {
            if (!File.Exists(filename))
            {
                Console.WriteLine($"Error: Could not open file {filename}");
                return 1;
            }

            string content = File.ReadAllText(filename);
            return ParseAndExecuteCode(content, filename);
        }
    }

    class Program
    {
        static void Main(string[] args)
        {
            // Set console encoding to UTF-8 to handle Chinese characters correctly
            Console.OutputEncoding = System.Text.Encoding.UTF8;

            if (args.Length < 2)
            {
                Console.WriteLine("Quern Lang C# Edition");
                Console.WriteLine("Usage: Quern.exe <Command> <Script_Name.q>");
                Console.WriteLine("Commands:");
                Console.WriteLine(" --Run       Run the Quern script");
                Console.WriteLine(" --Help      Show this help");
                return;
            }

            string command = args[0];
            string inputFile = args[1];

            ModManager modManager = new ModManager();
            QuernRuntime runtime = new QuernRuntime(modManager); // Pass modManager to runtime
            runtime.DebugMode = true;
            
            // Note: We no longer load ALL mods automatically here.
            // Mods are loaded on-demand via Import statements in the script.
            // modManager.LoadAllMods("mods", runtime); 

            if (command == "--Run")
            {
                Console.WriteLine($"Running {inputFile}...");
                runtime.ExecuteFile(inputFile);
            }
            else
            {
                Console.WriteLine($"Unknown command: {command}");
            }
            
            modManager.UnloadAllMods();
        }
    }
}
