package main

import (
	"fmt"
	"io/ioutil"
	"os"
	"os/exec"
	"path/filepath"
	"reflect"
	"regexp"
	"strings"

	"github.com/dop251/goja"
)

// --- AST Definitions ---

type Node interface {
	Type() string
}

type Program struct {
	Imports []string
	Nodes   []Node
}

func (p *Program) Type() string { return "Program" }

type FunctionDef struct {
	Name   string
	Body   []Node
	IsMain bool
}

func (f *FunctionDef) Type() string { return "FunctionDef" }

type VarDef struct {
	Name      string
	Value     string
	IsPrivate bool
	VarType   string
}

func (v *VarDef) Type() string { return "VarDef" }

type ConsoleInfo struct {
	Content string
}

func (c *ConsoleInfo) Type() string { return "ConsoleInfo" }

type ClassDef struct {
	Name    string
	Members []Node
}

func (c *ClassDef) Type() string { return "ClassDef" }

type TemplateUse struct {
	ClassName string
}

func (t *TemplateUse) Type() string { return "TemplateUse" }

type AliasDef struct {
	Name  string
	Value string
}

func (a *AliasDef) Type() string { return "AliasDef" }

type LoopBlock struct {
	Count string
	Body  []Node
}

func (l *LoopBlock) Type() string { return "LoopBlock" }

type IfBlock struct {
	Condition string
	Body      []Node
}

func (i *IfBlock) Type() string { return "IfBlock" }

// New Entrust Structure
type EntrustBlock struct {
	Condition string
	Body      []Node
	ID        int // Unique ID for generating function names
}

func (e *EntrustBlock) Type() string { return "EntrustBlock" }

type CustomNode struct {
	RawLine string
	Keyword string
}

func (c *CustomNode) Type() string { return "CustomNode" }

type ListDef struct {
	Name  string
	Items []string
}

func (l *ListDef) Type() string { return "ListDef" }

type DictDef struct {
	Name   string
	Pairs  map[string]string
}

func (d *DictDef) Type() string { return "DictDef" }

// --- Parser ---

type Parser struct {
	lines []string
	pos   int
}

func NewParser(source string) *Parser {
	source = strings.ReplaceAll(source, "\r\n", "\n")
	lines := strings.Split(source, "\n")
	return &Parser{lines: lines, pos: 0}
}

func (p *Parser) Parse() (*Program, error) {
	prog := &Program{}
	for p.pos < len(p.lines) {
		line := strings.TrimSpace(p.lines[p.pos])
		if line == "" || strings.HasPrefix(line, "//") {
			p.pos++
			continue
		}

		oldPos := p.pos
		node, err := p.parseLine(line)
		if err != nil {
			return nil, fmt.Errorf("Line %d: %v", p.pos+1, err)
		}

		if node != nil {
			prog.Nodes = append(prog.Nodes, node)
		}

		if p.pos == oldPos {
			p.pos++
		}
	}
	return prog, nil
}

func (p *Parser) parseLine(line string) (Node, error) {
	if strings.HasPrefix(line, "Import") {
		parts := strings.Fields(line)
		if len(parts) >= 2 {
			fmt.Printf("[Info] Importing: %s\n", strings.Trim(parts[1], "\""))
			return nil, nil
		}
	}

	if strings.HasPrefix(line, "Function") {
		return p.parseFunction(line)
	}

	if strings.HasPrefix(line, "Class") {
		return p.parseClass(line)
	}

	// Add Entrust Parsing
	if strings.HasPrefix(line, "Entrust") {
		return p.parseEntrust(line)
	}

	return nil, fmt.Errorf("Unknown top-level statement: %s", line)
}

func (p *Parser) parseFunction(line string) (*FunctionDef, error) {
	name := ""
	isMain := false

	rest := strings.TrimPrefix(line, "Function")
	rest = strings.TrimSpace(rest)

	if strings.HasPrefix(rest, "\"") {
		endQuote := strings.Index(rest[1:], "\"")
		if endQuote != -1 {
			name = rest[1 : endQuote+1]
			rest = rest[endQuote+2:]
			rest = strings.TrimSpace(rest)

			if strings.HasPrefix(rest, "(Main)") {
				isMain = true
				rest = rest[6:]
				rest = strings.TrimSpace(rest)
			} else if strings.HasPrefix(rest, "(") {
				closeParen := strings.Index(rest, ")")
				if closeParen != -1 {
					rest = rest[closeParen+1:]
					rest = strings.TrimSpace(rest)
				}
			}
		}
	}

	if name == "" {
		parts := strings.Fields(line)
		if len(parts) < 2 {
			return nil, fmt.Errorf("Invalid function definition")
		}
		name = strings.Trim(parts[1], "\"")
	}

	body, err := p.parseBlock()
	if err != nil {
		return nil, err
	}
	return &FunctionDef{Name: name, Body: body, IsMain: isMain}, nil
}

func (p *Parser) parseClass(line string) (*ClassDef, error) {
	parts := strings.Fields(line)
	if len(parts) < 2 {
		return nil, fmt.Errorf("Invalid class definition")
	}
	name := strings.Trim(parts[1], "\"")

	body, err := p.parseBlock()
	if err != nil {
		return nil, err
	}
	return &ClassDef{Name: name, Members: body}, nil
}

// parseEntrust handles: Entrust (Condition) { ... }
func (p *Parser) parseEntrust(line string) (*EntrustBlock, error) {
	start := strings.Index(line, "(")
	end := strings.Index(line, ")")
	
	if start == -1 || end == -1 || end < start {
		return nil, fmt.Errorf("Invalid Entrust syntax: missing parentheses")
	}

	condition := strings.TrimSpace(line[start+1 : end])
	
	// Check if there is a block following
	// The parser state is currently at the line with "Entrust (...)"
	// We need to move to the next line to find "{" or assume it's on the same line
	
	// Advance pos to look for the block
	// Note: parseBlock expects p.pos to be at the line containing "{" or the line after the header
	
	// Let's check if "{" is on the current line after ")"
	restOfLine := strings.TrimSpace(line[end+1:])
	if !strings.Contains(restOfLine, "{") {
		p.pos++ // Move to next line to find "{"
	}
	
	body, err := p.parseBlock()
	if err != nil {
		return nil, err
	}

	return &EntrustBlock{
		Condition: condition,
		Body:      body,
		ID:        0, // Will be assigned by translator
	}, nil
}

func (p *Parser) parseBlock() ([]Node, error) {
	var nodes []Node

	currentLine := p.lines[p.pos]
	if !strings.Contains(currentLine, "{") {
		p.pos++
		if p.pos >= len(p.lines) {
			return nil, fmt.Errorf("Unexpected end of file, expected '{'")
		}
		if strings.TrimSpace(p.lines[p.pos]) == "{" {
			p.pos++
		}
	} else {
		p.pos++
	}

	for p.pos < len(p.lines) {
		line := strings.TrimSpace(p.lines[p.pos])

		if line == "}" {
			p.pos++
			return nodes, nil
		}

		if line == "" || strings.HasPrefix(line, "//") {
			p.pos++
			continue
		}

		posBeforeStmt := p.pos
		node, err := p.parseStatement(line)
		if err != nil {
			return nil, err
		}
		if node != nil {
			nodes = append(nodes, node)
		}

		if p.pos == posBeforeStmt {
			p.pos++
		}
	}
	return nil, fmt.Errorf("Unmatched braces, expected '}'")
}

func (p *Parser) parseStatement(line string) (Node, error) {
	if strings.HasPrefix(line, "Data.Var") {
		return p.parseVarDef(line)
	}

	if strings.HasPrefix(line, "Console.Info") {
		return p.parseConsoleInfo(line)
	}

	if strings.HasPrefix(line, "Template") {
		parts := strings.Fields(line)
		if len(parts) >= 2 {
			return &TemplateUse{ClassName: strings.Trim(parts[1], "\"")}, nil
		}
	}

	if strings.HasPrefix(line, "Alias") {
		return p.parseAlias(line)
	}

	if strings.HasPrefix(line, "DataStruct.List") {
		return p.parseList(line)
	}

	if strings.HasPrefix(line, "DataStruct.Dict") {
		return p.parseDict(line)
	}

	if strings.HasPrefix(line, "Loop") {
		return p.parseLoop(line)
	}

	if strings.HasPrefix(line, "If") {
		return p.parseIf(line)
	}

	fields := strings.Fields(line)
	if len(fields) > 0 {
		keyword := fields[0]
		blacklist := []string{"Import", "Function", "Class", "Entrust"}
		isBlacklisted := false
		for _, b := range blacklist {
			if keyword == b {
				isBlacklisted = true
				break
			}
		}
		if !isBlacklisted {
			return &CustomNode{RawLine: line, Keyword: keyword}, nil
		}
	}

	return nil, fmt.Errorf("Unknown statement: %s", line)
}

func (p *Parser) parseVarDef(line string) (*VarDef, error) {
	parts := strings.Fields(line)
	if len(parts) < 4 {
		return nil, fmt.Errorf("Invalid VarDef: %s", line)
	}

	idx := 1
	isPrivate := false
	varType := "Any"

	if parts[idx] == "Private" {
		isPrivate = true
		idx++
	}

	if idx < len(parts) && (parts[idx] == "Int" || parts[idx] == "String" || parts[idx] == "Bool") {
		varType = parts[idx]
		idx++
	}

	if idx >= len(parts) {
		return nil, fmt.Errorf("Missing variable name")
	}
	name := parts[idx]
	idx++

	if idx >= len(parts) || parts[idx] != "=" {
		return nil, fmt.Errorf("Expected '='")
	}
	idx++

	value := strings.Join(parts[idx:], " ")
	value = strings.Trim(value, "\"")

	return &VarDef{Name: name, Value: value, IsPrivate: isPrivate, VarType: varType}, nil
}

func (p *Parser) parseConsoleInfo(line string) (*ConsoleInfo, error) {
	start := strings.Index(line, "(")
	end := strings.LastIndex(line, ")")
	if start == -1 || end == -1 {
		return nil, fmt.Errorf("Invalid Console.Info syntax")
	}
	content := line[start+1 : end]
	content = strings.Trim(content, "\"")
	return &ConsoleInfo{Content: content}, nil
}

func (p *Parser) parseAlias(line string) (*AliasDef, error) {
	parts := strings.Fields(line)
	if len(parts) < 4 {
		return nil, fmt.Errorf("Invalid Alias")
	}
	name := strings.Trim(parts[1], "\"")
	value := strings.Trim(parts[3], "\"")
	return &AliasDef{Name: name, Value: value}, nil
}

func (p *Parser) parseList(line string) (*ListDef, error) {
	parts := strings.Fields(line)
	if len(parts) < 4 {
		return nil, fmt.Errorf("Invalid List definition: %s", line)
	}

	name := strings.Trim(parts[1], "\"")
	
	if parts[2] != "=" {
		return nil, fmt.Errorf("Expected '=' in List definition")
	}

	valuePart := strings.Join(parts[3:], " ")
	items := make([]string, 0)

	if strings.HasPrefix(valuePart, "[") && strings.HasSuffix(valuePart, "]") {
		inner := strings.Trim(valuePart, "[]")
		if inner != "" {
			splitItems := strings.Split(inner, ",")
			for _, item := range splitItems {
				item = strings.TrimSpace(item)
				item = strings.Trim(item, "\"")
				if item != "" {
					items = append(items, item)
				}
			}
		}
	} else {
		val := strings.Trim(valuePart, "\"")
		if val != "" {
			items = append(items, val)
		}
	}

	return &ListDef{Name: name, Items: items}, nil
}

func (p *Parser) parseDict(line string) (*DictDef, error) {
	parts := strings.Fields(line)
	if len(parts) < 4 {
		return nil, fmt.Errorf("Invalid Dict definition: %s", line)
	}

	name := strings.Trim(parts[1], "\"")
	
	if parts[2] != "=" {
		return nil, fmt.Errorf("Expected '=' in Dict definition")
	}

	valuePart := strings.Join(parts[3:], " ")
	pairs := make(map[string]string)

	if strings.HasPrefix(valuePart, "{") && strings.HasSuffix(valuePart, "}") {
		inner := strings.Trim(valuePart, "{}")
		if inner != "" {
			kvPairs := strings.Split(inner, ",")
			for _, kv := range kvPairs {
				kv = strings.TrimSpace(kv)
				colonIdx := strings.Index(kv, ":")
				if colonIdx != -1 {
					k := strings.TrimSpace(kv[:colonIdx])
					v := strings.TrimSpace(kv[colonIdx+1:])
					k = strings.Trim(k, "\"")
					v = strings.Trim(v, "\"")
					if k != "" {
						pairs[k] = v
					}
				}
			}
		}
	}

	return &DictDef{Name: name, Pairs: pairs}, nil
}

func (p *Parser) parseLoop(line string) (*LoopBlock, error) {
	start := strings.Index(line, "(")
	end := strings.Index(line, ")")
	if start == -1 || end == -1 {
		return nil, fmt.Errorf("Invalid Loop syntax")
	}
	count := strings.TrimSpace(line[start+1 : end])

	body, err := p.parseBlock()
	if err != nil {
		return nil, err
	}
	return &LoopBlock{Count: count, Body: body}, nil
}

func (p *Parser) parseIf(line string) (*IfBlock, error) {
	start := strings.Index(line, "(")
	end := strings.Index(line, ")")
	if start == -1 || end == -1 {
		return nil, fmt.Errorf("Invalid If syntax")
	}
	cond := strings.TrimSpace(line[start+1 : end])

	body, err := p.parseBlock()
	if err != nil {
		return nil, err
	}
	return &IfBlock{Condition: cond, Body: body}, nil
}

// --- Mod Loader ---

type SyntaxHandler func(input string) (string, error)

type ModLoader struct {
	vm       *goja.Runtime
	handlers map[string]SyntaxHandler
}

func NewModLoader() *ModLoader {
	vm := goja.New()
	loader := &ModLoader{
		vm:       vm,
		handlers: make(map[string]SyntaxHandler),
	}
	loader.injectAPIs()
	return loader
}

func (ml *ModLoader) injectAPIs() {
	vm := ml.vm
	quernObj := vm.NewObject()

	quernObj.Set("Log", func(call goja.FunctionCall) goja.Value {
		if len(call.Arguments) > 0 {
			msg := call.Argument(0).String()
			fmt.Printf("[Mod Log] %s\n", msg)
		}
		return goja.Undefined()
	})

	quernObj.Set("Reg", func(call goja.FunctionCall) goja.Value {
		if len(call.Arguments) < 2 {
			return goja.Undefined()
		}

		keyword := call.Argument(0).String()
		handlerVal := call.Argument(1)

		if handlerVal.ExportType().Kind() != reflect.Func {
			fmt.Printf("[Warn] Quern.Reg: Second argument for '%s' is not a function\n", keyword)
			return goja.Undefined()
		}

		goHandler := func(input string) (string, error) {
			jsInput := vm.ToValue(input)
			callable, ok := goja.AssertFunction(handlerVal)
			if !ok {
				return "", fmt.Errorf("Handler for '%s' is not a function", keyword)
			}

			resultVal, err := callable(goja.Undefined(), jsInput)
			if err != nil {
				return "", fmt.Errorf("JS handler error for '%s': %v", keyword, err)
			}

			return resultVal.String(), nil
		}

		ml.handlers[keyword] = goHandler
		fmt.Printf("[Mod] Registered syntax handler for: %s\n", keyword)
		return goja.Undefined()
	})

	vm.Set("Quern", quernObj)
}

func (ml *ModLoader) LoadModsFromDirectory(modDir string) error {
	if _, err := os.Stat(modDir); os.IsNotExist(err) {
		return nil
	}

	files, err := ioutil.ReadDir(modDir)
	if err != nil {
		return err
	}

	for _, f := range files {
		if !strings.HasSuffix(f.Name(), ".js") {
			continue
		}

		fullPath := filepath.Join(modDir, f.Name())
		fmt.Printf("[Info] Processing mod: %s\n", f.Name())

		content, err := ioutil.ReadFile(fullPath)
		if err != nil {
			fmt.Printf("[Error] Failed to read %s: %v\n", f.Name(), err)
			continue
		}

		visited := make(map[string]bool)
		visited[fullPath] = true
		mergedCode, err := ProcessIncludes(modDir, string(content), visited)
		if err != nil {
			fmt.Printf("[Error] Failed to process includes for %s: %v\n", f.Name(), err)
			continue
		}

		_, err = ml.vm.RunString(mergedCode)
		if err != nil {
			fmt.Printf("[Error] Failed to execute mod %s: %v\n", f.Name(), err)
			continue
		}
	}

	return nil
}

func (ml *ModLoader) GetHandler(keyword string) (SyntaxHandler, bool) {
	handler, ok := ml.handlers[keyword]
	return handler, ok
}

func ProcessIncludes(baseDir string, content string, visited map[string]bool) (string, error) {
	re := regexp.MustCompile(`Include\s+["']([^"']+)["'];?`)

	var result strings.Builder
	lastIndex := 0

	for _, match := range re.FindAllStringSubmatchIndex(content, -1) {
		result.WriteString(content[lastIndex:match[0]])

		pathStart := match[2]
		pathEnd := match[3]
		includePath := content[pathStart:pathEnd]

		absPath := filepath.Join(baseDir, includePath)
		absPath, err := filepath.Abs(absPath)
		if err != nil {
			return "", fmt.Errorf("invalid include path: %s", includePath)
		}

		cleanBase, _ := filepath.Abs(baseDir)
		if !strings.HasPrefix(absPath, cleanBase) {
			return "", fmt.Errorf("security violation: include path escapes base directory: %s", includePath)
		}

		if visited[absPath] {
			fmt.Printf("[Warn] Circular include detected: %s, skipping.\n", absPath)
			lastIndex = match[1]
			continue
		}
		visited[absPath] = true

		subContent, err := ioutil.ReadFile(absPath)
		if err != nil {
			return "", fmt.Errorf("failed to read included file %s: %v", absPath, err)
		}

		subDir := filepath.Dir(absPath)
		processedSub, err := ProcessIncludes(subDir, string(subContent), visited)
		if err != nil {
			return "", err
		}

		result.WriteString(processedSub)
		lastIndex = match[1]
	}

	result.WriteString(content[lastIndex:])

	return result.String(), nil
}

// --- Translator ---

type Translator struct {
	Instructions []string
	Classes      map[string]*ClassDef
	Aliases      map[string]string
	LabelCounter int
	ModLoader    *ModLoader
	MainFuncName string
	
	// Entrust specific
	Entrusts []*EntrustBlock
}

func NewTranslatorWithMods(loader *ModLoader) *Translator {
	return &Translator{
		Instructions: make([]string, 0),
		Classes:      make(map[string]*ClassDef),
		Aliases:      make(map[string]string),
		ModLoader:    loader,
		MainFuncName: "",
		Entrusts:     make([]*EntrustBlock, 0),
	}
}

func (t *Translator) Translate(prog *Program) string {
	// First pass: Register Classes, Aliases, and Collect Entrusts
	for _, node := range prog.Nodes {
		switch n := node.(type) {
		case *ClassDef:
			t.Classes[n.Name] = n
		case *AliasDef:
			t.Aliases[n.Name] = n.Value
		case *EntrustBlock:
			n.ID = t.LabelCounter
			t.LabelCounter++
			t.Entrusts = append(t.Entrusts, n)
		}
	}

	// Second pass: Translate Functions and detect Main
	for _, node := range prog.Nodes {
		if fn, ok := node.(*FunctionDef); ok {
			t.emit(fmt.Sprintf("fnc %s {", fn.Name))
			t.translateBody(fn.Body, make(map[string]string))
			t.emit("}")

			if fn.IsMain {
				t.MainFuncName = fn.Name
			}
		}
	}

	// Generate Entrust Functions and Loop
	t.generateEntrustLogic()

	// Append call to main function if found
	if t.MainFuncName != "" {
		t.emit(fmt.Sprintf("cal %s", t.MainFuncName))
	}

	return strings.Join(t.Instructions, "\n")
}

func (t *Translator) generateEntrustLogic() {
	if len(t.Entrusts) == 0 {
		return
	}

	// 1. Generate individual Entrust functions and their Runners
	for _, entrust := range t.Entrusts {
		funcName := fmt.Sprintf("Entrust_%d", entrust.ID)
		runFuncName := fmt.Sprintf("RunEntrust_%d", entrust.ID)

		// Define the action function
		t.emit(fmt.Sprintf("fnc %s {", funcName))
		t.translateBody(entrust.Body, make(map[string]string))
		t.emit("}")

		// Define the runner function (checks condition)
		t.emit(fmt.Sprintf("fnc %s {", runFuncName))
		
		// Parse condition: "left op right"
		parts := strings.Fields(entrust.Condition)
		if len(parts) == 3 {
			left := parts[0]
			op := parts[1]
			right := parts[2]
			// jmp left right op cal target
			t.emit(fmt.Sprintf("jmp %s %s %s cal %s", left, right, op, funcName))
		} else {
			t.emit(fmt.Sprintf("# Invalid condition in Entrust %d: %s", entrust.ID, entrust.Condition))
		}
		
		t.emit("}")
	}

	// 2. Generate the Global Loop that checks all entrusts
	loopFuncName := "_EntrustLoop"
	t.emit(fmt.Sprintf("fnc %s {", loopFuncName))
	
	for _, entrust := range t.Entrusts {
		runFuncName := fmt.Sprintf("RunEntrust_%d", entrust.ID)
		// Use jmp 1 = 1 cal RunEntrust_ID to always trigger the check
		t.emit(fmt.Sprintf("jmp 1 = 1 cal %s", runFuncName))
	}
	
	// Recursive call to keep the loop running forever
	// Note: This will cause stack overflow eventually in a real VM without TCO, 
	// but for this simple VM implementation, it's the standard way to loop.
	t.emit(fmt.Sprintf("cal %s", loopFuncName))
	
	t.emit("}")

	// Call the loop at the very beginning of execution logic (appended after main call usually, 
	// but since we want it to run "always", we call it. 
	// However, if Main returns, the program ends. 
	// To make it truly "background", we should call it BEFORE Main or ensure Main doesn't return quickly.
	// Given the structure, we'll append it after Main call, implying Main might block or the loop is the last thing.
	// Actually, better to put it before Main if Main is long-running, or just append it.
	// Let's append it after Main call as per standard script flow.
	t.emit(fmt.Sprintf("cal %s", loopFuncName))
}

func (t *Translator) translateBody(nodes []Node, localAliases map[string]string) {
	allAliases := make(map[string]string)
	for k, v := range t.Aliases {
		allAliases[k] = v
	}
	for k, v := range localAliases {
		allAliases[k] = v
	}

	for _, node := range nodes {
		switch n := node.(type) {
		case *VarDef:
			t.emit(fmt.Sprintf("crt %s", n.Name))
			val := n.Value
			if v, ok := allAliases[val]; ok {
				val = v
			}
			t.emit(fmt.Sprintf("psh %s %s", n.Name, val))

		case *ListDef:
			t.emit(fmt.Sprintf("crt %s_len", n.Name))
			t.emit(fmt.Sprintf("psh %s_len %d", n.Name, len(n.Items)))
			
			for i, item := range n.Items {
				varName := fmt.Sprintf("%s_%d", n.Name, i)
				t.emit(fmt.Sprintf("crt %s", varName))
				
				val := item
				if v, ok := allAliases[item]; ok {
					val = v
				}
				t.emit(fmt.Sprintf("psh %s %s", varName, val))
			}

		case *DictDef:
			for k, v := range n.Pairs {
				keyVar := fmt.Sprintf("%s_key_%s", n.Name, k)
				valVar := fmt.Sprintf("%s_val_%s", n.Name, k)
				
				t.emit(fmt.Sprintf("crt %s", keyVar))
				t.emit(fmt.Sprintf("psh %s %s", keyVar, k))
				
				t.emit(fmt.Sprintf("crt %s", valVar))
				
				resolvedVal := v
				if rv, ok := allAliases[v]; ok {
					resolvedVal = rv
				}
				t.emit(fmt.Sprintf("psh %s %s", valVar, resolvedVal))
			}

		case *ConsoleInfo:
			if t.isLiteral(n.Content) {
				tempStack := fmt.Sprintf("_lit_%d", t.LabelCounter)
				t.LabelCounter++
				t.emit(fmt.Sprintf("crt %s", tempStack))
				t.emit(fmt.Sprintf("psh %s %s", tempStack, n.Content))
				t.emit(fmt.Sprintf("out %s", tempStack))
				t.emit(fmt.Sprintf("del %s", tempStack))
			} else {
				content := n.Content
				if v, ok := allAliases[content]; ok {
					content = v
				}
				t.emit(fmt.Sprintf("out %s", content))
			}
			t.emit("otn")

		case *TemplateUse:
			if cls, ok := t.Classes[n.ClassName]; ok {
				t.translateBody(cls.Members, localAliases)
			} else {
				fmt.Printf("[Warn] Class %s not found\n", n.ClassName)
			}

		case *AliasDef:
			localAliases[n.Name] = n.Value

		case *LoopBlock:
			t.translateLoop(n, localAliases)

		case *IfBlock:
			t.translateIf(n, localAliases)

		case *CustomNode:
			if t.ModLoader != nil {
				if handler, ok := t.ModLoader.GetHandler(n.Keyword); ok {
					result, err := handler(n.RawLine)
					if err != nil {
						t.emit(fmt.Sprintf("# Error in custom syntax '%s': %v", n.Keyword, err))
					} else {
						lines := strings.Split(result, "\n")
						for _, l := range lines {
							l = strings.TrimSpace(l)
							if l != "" {
								t.emit(l)
							}
						}
					}
				} else {
					t.emit(fmt.Sprintf("# Unknown custom syntax: %s", n.RawLine))
				}
			} else {
				t.emit(fmt.Sprintf("# Mod system not initialized for: %s", n.RawLine))
			}
		}
	}
}

func (t *Translator) isLiteral(s string) bool {
	if len(s) == 0 {
		return true
	}
	first := s[0]
	if (first >= 'a' && first <= 'z') || (first >= 'A' && first <= 'Z') || first == '_' {
		return false
	}
	return true
}

func (t *Translator) translateLoop(loop *LoopBlock, localAliases map[string]string) {
	fmt.Printf("[Warn] QVM lacks math operations. Loop(%s) cannot be dynamically implemented.\n", loop.Count)

	var count int
	_, err := fmt.Sscanf(loop.Count, "%d", &count)

	if err == nil && count > 0 && count <= 10 {
		fmt.Printf("[Info] Static unrolling Loop(%d)\n", count)
		for i := 0; i < count; i++ {
			t.translateBody(loop.Body, localAliases)
		}
	} else {
		t.emit(fmt.Sprintf("# Loop(%s) skipped or simulated once", loop.Count))
		t.translateBody(loop.Body, localAliases)
	}
}

func (t *Translator) translateIf(ifBlock *IfBlock, localAliases map[string]string) {
	funcName := fmt.Sprintf("_if_body_%d", t.LabelCounter)
	t.LabelCounter++

	parts := strings.Fields(ifBlock.Condition)
	if len(parts) != 3 {
		fmt.Printf("[Warn] Complex condition '%s' not supported. Skipping.\n", ifBlock.Condition)
		return
	}
	left := parts[0]
	op := parts[1]
	right := parts[2]

	t.emit(fmt.Sprintf("jmp %s %s %s cal %s", left, right, op, funcName))

	t.emit(fmt.Sprintf("fnc %s {", funcName))
	t.translateBody(ifBlock.Body, localAliases)
	t.emit("}")
}

func (t *Translator) emit(instr string) {
	t.Instructions = append(t.Instructions, instr)
}

// --- Main Execution ---

func main() {
	if len(os.Args) < 2 {
		fmt.Println("Usage: quern-translator --Run <file.q>")
		os.Exit(1)
	}

	if os.Args[1] != "--Run" || len(os.Args) < 3 {
		fmt.Println("Usage: quern-translator --Run <file.q>")
		os.Exit(1)
	}

	sourceFile := os.Args[2]
	modDir := "mods"

	// 1. Initialize Mod Loader
	loader := NewModLoader()

	// 2. Load Mods
	if _, err := os.Stat(modDir); err == nil {
		if err := loader.LoadModsFromDirectory(modDir); err != nil {
			fmt.Printf("[Error] Failed to load mods: %v\n", err)
		}
	} else {
		fmt.Println("[Info] No mods directory found.")
	}

	// 3. Read Source
	content, err := ioutil.ReadFile(sourceFile)
	if err != nil {
		fmt.Printf("[Error] Cannot read file: %v\n", err)
		os.Exit(1)
	}

	// 4. Parse
	parser := NewParser(string(content))
	prog, err := parser.Parse()
	if err != nil {
		fmt.Printf("[Parse Error] %v\n", err)
		os.Exit(1)
	}

	// 5. Translate
	translator := NewTranslatorWithMods(loader)
	qbCode := translator.Translate(prog)

	// 6. Save Cache
	baseName := filepath.Base(sourceFile)
	nameWithoutExt := strings.TrimSuffix(baseName, filepath.Ext(baseName))
	cacheDir := "cache"
	os.MkdirAll(cacheDir, os.ModePerm)

	outputFile := filepath.Join(cacheDir, nameWithoutExt+".qb")
	err = ioutil.WriteFile(outputFile, []byte(qbCode), 0644)
	if err != nil {
		fmt.Printf("[Error] Cannot write cache file: %v\n", err)
		os.Exit(1)
	}

	fmt.Printf("[Info] Translated to: %s\n", outputFile)

	// 7. Run QVM
	cmd := exec.Command("./Qvm.exe", "--Run", outputFile)
	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr

	fmt.Println("--- Running QVM ---")
	err = cmd.Run()
	if err != nil {
		fmt.Printf("[Runtime Error] QVM execution failed: %v\n", err)
	}
}
