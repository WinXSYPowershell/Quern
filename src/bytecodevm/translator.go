package main

import (
	"crypto/md5"
	"encoding/hex"
	"fmt"
	"io/ioutil"
	"os"
	"os/exec"
	"path/filepath"
	"reflect"
	"regexp"
	"strconv"
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

// Modified IfBlock to support Else, ElseIf, and Loop sugar
type IfBlock struct {
	LeftCond      string // e.g., "a = 1"
	RightCond     string // e.g., "b = 2" (empty if no logic op)
	LogicOp       string // "", "and", "or"
	IsNot         bool   // true if 'not' keyword is present
	Body          []Node
	ElseBody      []Node    // Code block for Else
	ElseIf        *IfBlock  // Linked list for Else If
	ElseLoopCount string    // If not empty, the Else body acts as a loop body. "-1" means infinite.
	LoopCount     string    // If not empty, the If/Else body acts as a loop body
}

func (i *IfBlock) Type() string { return "IfBlock" }

type EntrustBlock struct {
	Condition string
	Body      []Node
	ID        int
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

// --- New AST Nodes for CRUD Operations ---

type ListOpAdd struct {
	ListName string
	Value    string
}

func (l *ListOpAdd) Type() string { return "ListOpAdd" }

type ListOpDelete struct {
	ListName string
	Item     string // Item content to delete
}

func (l *ListOpDelete) Type() string { return "ListOpDelete" }

type ListOpEdit struct {
	ListName string
	Index    int    // Index to edit
	NewValue string // New value
}

func (l *ListOpEdit) Type() string { return "ListOpEdit" }

type ListOpFindBool struct {
	ListName string
	Target   string
	VarName  string
}

func (l *ListOpFindBool) Type() string { return "ListOpFindBool" }

type ListOpFindIndex struct {
	ListName string
	Target   string
	VarName  string
}

func (l *ListOpFindIndex) Type() string { return "ListOpFindIndex" }

type DictOpAdd struct {
	DictName string
	Key      string
	Value    string
}

func (d *DictOpAdd) Type() string { return "DictOpAdd" }

type DictOpDelete struct {
	DictName string
	Key      string
}

func (d *DictOpDelete) Type() string { return "DictOpDelete" }

type DictOpEdit struct {
	DictName string
	Key      string
	NewValue string
}

func (d *DictOpEdit) Type() string { return "DictOpEdit" }

type DictOpFindBool struct {
	DictName    string
	TargetValue string // Searching for this value
	VarName     string
}

func (d *DictOpFindBool) Type() string { return "DictOpFindBool" }

type DictOpFindKey struct {
	DictName    string
	TargetValue string // Searching for this value
	VarName     string
}

func (d *DictOpFindKey) Type() string { return "DictOpFindKey" }

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

func (p *Parser) parseEntrust(line string) (*EntrustBlock, error) {
	start := strings.Index(line, "(")
	end := strings.Index(line, ")")

	if start == -1 || end == -1 || end < start {
		return nil, fmt.Errorf("Invalid Entrust syntax: missing parentheses")
	}

	condition := strings.TrimSpace(line[start+1 : end])

	restOfLine := strings.TrimSpace(line[end+1:])
	if !strings.Contains(restOfLine, "{") {
		p.pos++
	}

	body, err := p.parseBlock()
	if err != nil {
		return nil, err
	}

	return &EntrustBlock{
		Condition: condition,
		Body:      body,
		ID:        0,
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

	// --- New CRUD Parsers ---
	if strings.HasPrefix(line, "DataStruct.ListAdd") {
		return p.parseListAdd(line)
	}
	if strings.HasPrefix(line, "DataStruct.ListDelete") {
		return p.parseListDelete(line)
	}
	if strings.HasPrefix(line, "DataStruct.ListEdit") {
		return p.parseListEdit(line)
	}
	if strings.HasPrefix(line, "DataStruct.ListFind.Bool") {
		return p.parseListFindBool(line)
	}
	if strings.HasPrefix(line, "DataStruct.ListFind.Index") {
		return p.parseListFindIndex(line)
	}

	if strings.HasPrefix(line, "DataStruct.DictAdd") {
		return p.parseDictAdd(line)
	}
	if strings.HasPrefix(line, "DataStruct.DictDelete") {
		return p.parseDictDelete(line)
	}
	if strings.HasPrefix(line, "DataStruct.DictEdit") {
		return p.parseDictEdit(line)
	}
	if strings.HasPrefix(line, "DataStruct.DictFind.Bool") {
		return p.parseDictFindBool(line)
	}
	if strings.HasPrefix(line, "DataStruct.DictFind.Key") {
		return p.parseDictFindKey(line)
	}

	if strings.HasPrefix(line, "Loop") {
		return p.parseLoop(line)
	}

	if strings.HasPrefix(line, "If") {
		return p.parseIf(line)
	}

	// Handle Else/ElseIf at statement level (usually inside parseIf logic, but just in case)
	if strings.HasPrefix(line, "Else") {
		// This should ideally be handled within parseIf's block parsing, 
		// but if it appears here, it might be an orphan or part of a complex structure.
		// For now, we let parseIf handle the flow control.
		return nil, fmt.Errorf("Orphaned Else statement")
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

	if idx < len(parts) && (parts[idx] == "Int" || parts[idx] == "String" || parts[idx] == "Bool" || parts[idx] == "Num") {
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

// --- CRUD Parsers Implementation ---

func (p *Parser) parseListAdd(line string) (*ListOpAdd, error) {
	parts := strings.Fields(line)
	if len(parts) < 4 {
		return nil, fmt.Errorf("Invalid ListAdd syntax")
	}

	val := strings.Trim(parts[1], "\"")
	listName := strings.Trim(parts[3], "\"")

	return &ListOpAdd{ListName: listName, Value: val}, nil
}

func (p *Parser) parseListDelete(line string) (*ListOpDelete, error) {
	parts := strings.Fields(line)
	if len(parts) < 4 {
		return nil, fmt.Errorf("Invalid ListDelete syntax")
	}

	item := strings.Trim(parts[1], "\"")
	listName := strings.Trim(parts[3], "\"")

	return &ListOpDelete{ListName: listName, Item: item}, nil
}

func (p *Parser) parseListEdit(line string) (*ListOpEdit, error) {
	parts := strings.Fields(line)
	if len(parts) < 6 {
		return nil, fmt.Errorf("Invalid ListEdit syntax")
	}

	listName := strings.Trim(parts[1], "\"")
	indexStr := parts[3]
	index, err := strconv.Atoi(indexStr)
	if err != nil {
		return nil, fmt.Errorf("Invalid index in ListEdit: %s", indexStr)
	}
	newVal := strings.Trim(parts[5], "\"")

	return &ListOpEdit{ListName: listName, Index: index, NewValue: newVal}, nil
}

func (p *Parser) parseListFindBool(line string) (*ListOpFindBool, error) {
	parts := strings.Fields(line)
	if len(parts) < 6 {
		return nil, fmt.Errorf("Invalid ListFind.Bool syntax")
	}

	listName := strings.Trim(parts[1], "\"")
	target := strings.Trim(parts[3], "\"")
	varName := strings.Trim(parts[5], "\"")

	return &ListOpFindBool{ListName: listName, Target: target, VarName: varName}, nil
}

func (p *Parser) parseListFindIndex(line string) (*ListOpFindIndex, error) {
	parts := strings.Fields(line)
	if len(parts) < 6 {
		return nil, fmt.Errorf("Invalid ListFind.Index syntax")
	}

	listName := strings.Trim(parts[1], "\"")
	target := strings.Trim(parts[3], "\"")
	varName := strings.Trim(parts[5], "\"")

	return &ListOpFindIndex{ListName: listName, Target: target, VarName: varName}, nil
}

func (p *Parser) parseDictAdd(line string) (*DictOpAdd, error) {
	parts := strings.Fields(line)
	if len(parts) < 6 {
		return nil, fmt.Errorf("Invalid DictAdd syntax")
	}

	key := strings.Trim(parts[1], "\"")
	val := strings.Trim(parts[3], "\"")
	dictName := strings.Trim(parts[5], "\"")

	return &DictOpAdd{DictName: dictName, Key: key, Value: val}, nil
}

func (p *Parser) parseDictDelete(line string) (*DictOpDelete, error) {
	parts := strings.Fields(line)
	if len(parts) < 4 {
		return nil, fmt.Errorf("Invalid DictDelete syntax")
	}

	key := strings.Trim(parts[1], "\"")
	dictName := strings.Trim(parts[3], "\"")

	return &DictOpDelete{DictName: dictName, Key: key}, nil
}

func (p *Parser) parseDictEdit(line string) (*DictOpEdit, error) {
	parts := strings.Fields(line)
	if len(parts) < 6 {
		return nil, fmt.Errorf("Invalid DictEdit syntax")
	}

	dictName := strings.Trim(parts[1], "\"")
	key := strings.Trim(parts[3], "\"")
	newVal := strings.Trim(parts[5], "\"")

	return &DictOpEdit{DictName: dictName, Key: key, NewValue: newVal}, nil
}

func (p *Parser) parseDictFindBool(line string) (*DictOpFindBool, error) {
	parts := strings.Fields(line)
	if len(parts) < 6 {
		return nil, fmt.Errorf("Invalid DictFind.Bool syntax")
	}

	dictName := strings.Trim(parts[1], "\"")
	targetVal := strings.Trim(parts[3], "\"")
	varName := strings.Trim(parts[5], "\"")

	return &DictOpFindBool{DictName: dictName, TargetValue: targetVal, VarName: varName}, nil
}

func (p *Parser) parseDictFindKey(line string) (*DictOpFindKey, error) {
	parts := strings.Fields(line)
	if len(parts) < 6 {
		return nil, fmt.Errorf("Invalid DictFind.Key syntax")
	}

	dictName := strings.Trim(parts[1], "\"")
	targetVal := strings.Trim(parts[3], "\"")
	varName := strings.Trim(parts[5], "\"")

	return &DictOpFindKey{DictName: dictName, TargetValue: targetVal, VarName: varName}, nil
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

// Updated parseIf to support Else, ElseIf, and Loop Sugar
func (p *Parser) parseIf(line string) (*IfBlock, error) {
	start := strings.Index(line, "(")
	end := strings.LastIndex(line, ")")

	if start == -1 || end == -1 || end <= start {
		return nil, fmt.Errorf("Invalid If syntax")
	}

	rawCond := strings.TrimSpace(line[start+1 : end])

	// Check for Loop Sugar after the closing parenthesis of condition
	// Example: If (a=1) Loop(5) { ...
	loopCount := ""
	restAfterCond := strings.TrimSpace(line[end+1:])
	if strings.HasPrefix(restAfterCond, "Loop") {
		lStart := strings.Index(restAfterCond, "(")
		lEnd := strings.Index(restAfterCond, ")")
		if lStart != -1 && lEnd != -1 {
			loopCount = strings.TrimSpace(restAfterCond[lStart+1 : lEnd])
		}
	}

	// Parse Logic Keywords
	isNot := false
	logicOp := ""
	leftCond := ""
	rightCond := ""

	// Check for NOT
	if strings.HasPrefix(strings.ToLower(rawCond), "not ") {
		isNot = true
		rawCond = strings.TrimSpace(rawCond[4:])
	}

	// Check for AND / OR
	andIdx := findLogicKeyword(rawCond, "and")
	orIdx := findLogicKeyword(rawCond, "or")

	if andIdx != -1 {
		logicOp = "and"
		leftCond = strings.TrimSpace(rawCond[:andIdx])
		rightCond = strings.TrimSpace(rawCond[andIdx+3:])
	} else if orIdx != -1 {
		logicOp = "or"
		leftCond = strings.TrimSpace(rawCond[:orIdx])
		rightCond = strings.TrimSpace(rawCond[orIdx+2:])
	} else {
		leftCond = rawCond
	}

	body, err := p.parseBlock()
	if err != nil {
		return nil, err
	}

	ifBlock := &IfBlock{
		LeftCond:  leftCond,
		RightCond: rightCond,
		LogicOp:   logicOp,
		IsNot:     isNot,
		Body:      body,
		LoopCount: loopCount,
	}

	// Check for Else or Else If immediately following the block
	if p.pos < len(p.lines) {
		nextLine := strings.TrimSpace(p.lines[p.pos])
		if strings.HasPrefix(nextLine, "Else") {
			p.pos++ // Consume the Else line

			if strings.HasPrefix(nextLine, "Else If") {
				// Parse Else If recursively
				elseIfBlock, err := p.parseIf(nextLine) // Re-use parseIf logic
				if err != nil {
					return nil, err
				}
				ifBlock.ElseIf = elseIfBlock
			} else if nextLine == "Else" || strings.HasPrefix(nextLine, "Else ") {
				// Check for Else Loop sugar
				elseLoopCount := ""

				// Handle "Else Loop(N)" or "Else Loop (N)"
				// We need to check if the line contains "Loop"
				if strings.Contains(nextLine, "Loop") {
					lStart := strings.Index(nextLine, "Loop")
					if lStart != -1 {
						rest := nextLine[lStart:]
						pStart := strings.Index(rest, "(")
						pEnd := strings.Index(rest, ")")
						if pStart != -1 && pEnd != -1 {
							elseLoopCount = strings.TrimSpace(rest[pStart+1 : pEnd])
						}
					}
				}

				// Parse Else Body
				elseBody, err := p.parseBlock()
				if err != nil {
					return nil, err
				}
				ifBlock.ElseBody = elseBody
				ifBlock.ElseLoopCount = elseLoopCount
			}
		}
	}

	return ifBlock, nil
}

// Helper to find keyword surrounded by spaces or at start/end
func findLogicKeyword(s string, keyword string) int {
	lowerS := strings.ToLower(s)
	target := " " + keyword + " "

	// Check middle
	idx := strings.Index(lowerS, target)
	if idx != -1 {
		return idx + 1 // Return index of the keyword itself
	}

	// Check start (shouldn't happen if 'not' is handled first, but just in case)
	if strings.HasPrefix(lowerS, keyword+" ") {
		return 0
	}

	// Check end
	if strings.HasSuffix(lowerS, " "+keyword) {
		return len(s) - len(keyword)
	}

	return -1
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

// --- Dead Code Eliminator ---

type DeadCodeEliminator struct {
	usedFunctions map[string]bool
	usedClasses   map[string]bool
	usedVars      map[string]bool
	usedLists     map[string]bool
	usedDicts     map[string]bool
	mainFound     bool
}

func NewDeadCodeEliminator() *DeadCodeEliminator {
	return &DeadCodeEliminator{
		usedFunctions: make(map[string]bool),
		usedClasses:   make(map[string]bool),
		usedVars:      make(map[string]bool),
		usedLists:     make(map[string]bool),
		usedDicts:     make(map[string]bool),
	}
}

// Analyze performs the reachability analysis
func (dce *DeadCodeEliminator) Analyze(prog *Program) {
	// 1. Identify Roots: Main Function and All Entrusts
	for _, node := range prog.Nodes {
		switch n := node.(type) {
		case *FunctionDef:
			if n.IsMain {
				dce.mainFound = true
				dce.usedFunctions[n.Name] = true
				dce.scanBody(n.Body)
			}
		case *EntrustBlock:
			// Entrusts are implicitly called by the VM loop, so they are always "used"
			dce.scanBody(n.Body)
		}
	}

	// If no main found, we might still want to keep global definitions if any, 
	// but typically Quern scripts need a Main or Entrusts.
	// If there are no roots, nothing is used.
}

// scanBody recursively scans nodes for references
func (dce *DeadCodeEliminator) scanBody(nodes []Node) {
	for _, node := range nodes {
		switch n := node.(type) {
		case *FunctionDef:
			// Nested function definition? Usually not in this grammar, but if so:
			// We don't mark it as used unless called, but we scan its body if it WAS called elsewhere.
			// However, since we are scanning FROM a used function, we just scan the body.
			dce.scanBody(n.Body)
		
		case *ClassDef:
			dce.scanBody(n.Members)

		case *TemplateUse:
			dce.usedClasses[n.ClassName] = true
			// Note: We don't recursively scan the class members here immediately to avoid 
			// infinite loops if classes reference each other, but since we are doing a simple 
			// reachability from Main, we should probably scan the class content if it's used.
			// For simplicity in this pass, we mark it used. A second pass could scan class bodies.
			
		case *VarDef:
			dce.usedVars[n.Name] = true

		case *ListDef:
			dce.usedLists[n.Name] = true
			
		case *DictDef:
			dce.usedDicts[n.Name] = true

		case *ConsoleInfo:
			// No definitions to track

		case *LoopBlock:
			dce.scanBody(n.Body)

		case *IfBlock:
			dce.scanBody(n.Body)
			if n.ElseBody != nil {
				dce.scanBody(n.ElseBody)
			}
			if n.ElseIf != nil {
				// ElseIf is an IfBlock, so we need to scan its body too
				dce.scanBody(n.ElseIf.Body)
				if n.ElseIf.ElseBody != nil {
					dce.scanBody(n.ElseIf.ElseBody)
				}
				if n.ElseIf.ElseIf != nil {
					// Recursive chain handling would be complex, but scanBody handles the immediate body.
					// Ideally, we should treat ElseIf as a node to scan.
					dce.scanIfChain(n.ElseIf)
				}
			}

		case *ListOpAdd:
			dce.usedLists[n.ListName] = true
		case *ListOpDelete:
			dce.usedLists[n.ListName] = true
		case *ListOpEdit:
			dce.usedLists[n.ListName] = true
		case *ListOpFindBool:
			dce.usedLists[n.ListName] = true
			dce.usedVars[n.VarName] = true
		case *ListOpFindIndex:
			dce.usedLists[n.ListName] = true
			dce.usedVars[n.VarName] = true

		case *DictOpAdd:
			dce.usedDicts[n.DictName] = true
		case *DictOpDelete:
			dce.usedDicts[n.DictName] = true
		case *DictOpEdit:
			dce.usedDicts[n.DictName] = true
		case *DictOpFindBool:
			dce.usedDicts[n.DictName] = true
			dce.usedVars[n.VarName] = true
		case *DictOpFindKey:
			dce.usedDicts[n.DictName] = true
			dce.usedVars[n.VarName] = true

		case *CustomNode:
			// Custom nodes might call functions or use variables, 
			// but without parsing their internal syntax, we can't know.
			// Conservative approach: assume they might use something, 
			// or rely on the fact that if the CustomNode is reachable, it's kept.
		}
	}
}

// scanIfChain handles the linked list nature of ElseIf
func (dce *DeadCodeEliminator) scanIfChain(ifBlock *IfBlock) {
	if ifBlock == nil {
		return
	}
	dce.scanBody(ifBlock.Body)
	if ifBlock.ElseBody != nil {
		dce.scanBody(ifBlock.ElseBody)
	}
	if ifBlock.ElseIf != nil {
		dce.scanIfChain(ifBlock.ElseIf)
	}
}

// Filter creates a new Program with only used nodes
func (dce *DeadCodeEliminator) Filter(prog *Program) *Program {
	newProg := &Program{
		Imports: prog.Imports,
		Nodes:   make([]Node, 0),
	}

	// First, ensure Classes that are used have their bodies scanned for further dependencies
	// This is a simplified multi-pass approach.
	changed := true
	for changed {
		changed = false
		for _, node := range prog.Nodes {
			if cls, ok := node.(*ClassDef); ok {
				if dce.usedClasses[cls.Name] {
					// Check if members introduce new used items
					// We need to temporarily add them to used sets to see if they trigger more usage?
					// For now, just scan.
					dce.scanBody(cls.Members)
				}
			}
		}
		// In a real compiler, we'd iterate until stable. Here we do one deep scan.
		break 
	}

	for _, node := range prog.Nodes {
		keep := false
		warnMsg := ""

		switch n := node.(type) {
		case *FunctionDef:
			if dce.usedFunctions[n.Name] {
				keep = true
			} else {
				warnMsg = fmt.Sprintf("Unused Function: \"%s\"", n.Name)
			}
		case *ClassDef:
			if dce.usedClasses[n.Name] {
				keep = true
			} else {
				warnMsg = fmt.Sprintf("Unused Class: \"%s\"", n.Name)
			}
		case *EntrustBlock:
			// Always keep Entrusts as they are root entries
			keep = true
		case *VarDef:
			if dce.usedVars[n.Name] {
				keep = true
			} else {
				warnMsg = fmt.Sprintf("Unused Variable: %s", n.Name)
			}
		case *ListDef:
			if dce.usedLists[n.Name] {
				keep = true
			} else {
				warnMsg = fmt.Sprintf("Unused List: \"%s\"", n.Name)
			}
		case *DictDef:
			if dce.usedDicts[n.Name] {
				keep = true
			} else {
				warnMsg = fmt.Sprintf("Unused Dict: \"%s\"", n.Name)
			}
		default:
			// Keep other top-level nodes (Imports, etc.)
			keep = true
		}

		if keep {
			newProg.Nodes = append(newProg.Nodes, node)
		} else if warnMsg != "" {
			fmt.Printf("[DCE Warning] %s\n", warnMsg)
		}
	}

	return newProg
}


// --- Translator ---

type Translator struct {
	Instructions []string
	Classes      map[string]*ClassDef
	Aliases      map[string]string
	LabelCounter int
	ModLoader    *ModLoader
	MainFuncName string

	Entrusts []*EntrustBlock

	// Data State for Lists and Dicts
	Lists map[string][]string
	Dicts map[string]map[string]string

	// JS VM for expression evaluation
	jsVm *goja.Runtime
}

func NewTranslatorWithMods(loader *ModLoader) *Translator {
	return &Translator{
		Instructions: make([]string, 0),
		Classes:      make(map[string]*ClassDef),
		Aliases:      make(map[string]string),
		ModLoader:    loader,
		MainFuncName: "",
		Entrusts:     make([]*EntrustBlock, 0),
		Lists:        make(map[string][]string),
		Dicts:        make(map[string]map[string]string),
		jsVm:         goja.New(), // Initialize JS VM for math eval
	}
}

// evaluateExpression attempts to calculate numeric expressions using JS engine.
// If the input is a pure number or a valid math expression (e.g., "1+2", "10*5"), it returns the calculated result as a string.
// Otherwise, it returns the original string.
func (t *Translator) evaluateExpression(expr string) string {
	// Trim whitespace
	expr = strings.TrimSpace(expr)

	// Quick check: if it's already a simple number, return it
	if _, err := strconv.ParseFloat(expr, 64); err == nil {
		return expr
	}

	// Check if it looks like a math expression (contains digits and operators)
	// We allow: 0-9, ., +, -, *, /, %, (, ), space
	matched, _ := regexp.MatchString(`^[0-9\.\+\-\*\/\%\(\)\s]+$`, expr)
	if !matched {
		return expr
	}

	// Use JS VM to evaluate safely
	val, err := t.jsVm.RunString(expr)
	if err != nil {
		// If evaluation fails, treat it as a literal string
		return expr
	}

	// Convert result to string
	// JS numbers are floats, so we format them nicely
	floatVal := val.ToFloat()

	// Check if it's an integer value
	if floatVal == float64(int64(floatVal)) {
		return fmt.Sprintf("%d", int64(floatVal))
	}

	return fmt.Sprintf("%g", floatVal)
}

func (t *Translator) Translate(prog *Program) string {
	// First pass: Register Classes, Aliases, Collect Entrusts, Init Data Structures
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
		case *ListDef:
			t.Lists[n.Name] = n.Items
		case *DictDef:
			t.Dicts[n.Name] = n.Pairs
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

	for _, entrust := range t.Entrusts {
		funcName := fmt.Sprintf("Entrust_%d", entrust.ID)
		runFuncName := fmt.Sprintf("RunEntrust_%d", entrust.ID)

		t.emit(fmt.Sprintf("fnc %s {", funcName))
		t.translateBody(entrust.Body, make(map[string]string))
		t.emit("}")

		t.emit(fmt.Sprintf("fnc %s {", runFuncName))

		parts := strings.Fields(entrust.Condition)
		if len(parts) == 3 {
			left := parts[0]
			op := parts[1]
			right := parts[2]
			t.emit(fmt.Sprintf("jmp %s %s %s cal %s", left, right, op, funcName))
		} else {
			t.emit(fmt.Sprintf("# Invalid condition in Entrust %d: %s", entrust.ID, entrust.Condition))
		}

		t.emit("}")
	}

	loopFuncName := "_EntrustLoop"
	t.emit(fmt.Sprintf("fnc %s {", loopFuncName))

	for _, entrust := range t.Entrusts {
		runFuncName := fmt.Sprintf("RunEntrust_%d", entrust.ID)
		t.emit(fmt.Sprintf("jmp 1 = 1 cal %s", runFuncName))
	}

	t.emit(fmt.Sprintf("cal %s", loopFuncName))
	t.emit("}")

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

			// Apply aliases first
			if v, ok := allAliases[val]; ok {
				val = v
			}

			// NEW: Evaluate numeric expressions if type is Int or Num, or if it looks like math
			// We attempt evaluation for Int/Num types specifically, or if it's not a known alias/string literal
			if n.VarType == "Int" || n.VarType == "Num" {
				val = t.evaluateExpression(val)
			} else if n.VarType == "Any" {
				// For Any, we try to see if it's a math expression that should be pre-calculated
				// Only if it doesn't look like a variable name (starts with letter)
				if len(val) > 0 && !((val[0] >= 'a' && val[0] <= 'z') || (val[0] >= 'A' && val[0] <= 'Z') || val[0] == '_') {
					val = t.evaluateExpression(val)
				}
			}

			t.emit(fmt.Sprintf("psh %s %s", n.Name, val))

		case *ListDef:
			t.rebuildListStack(n.Name)

		case *DictDef:
			t.rebuildDictStack(n.Name)

		case *ConsoleInfo:
			content := n.Content
			if v, ok := allAliases[content]; ok {
				content = v
			}
			t.emit(fmt.Sprintf("out %s", content))
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

		// --- CRUD Handlers ---
		case *ListOpAdd:
			if list, ok := t.Lists[n.ListName]; ok {
				newList := append([]string{n.Value}, list...)
				t.Lists[n.ListName] = newList
				t.rebuildListStack(n.ListName)
			} else {
				t.emit(fmt.Sprintf("# Error: List %s not defined", n.ListName))
			}

		case *ListOpDelete:
			if list, ok := t.Lists[n.ListName]; ok {
				newList := make([]string, 0)
				found := false
				for _, item := range list {
					if item == n.Item && !found {
						found = true // Delete first occurrence
						continue
					}
					newList = append(newList, item)
				}
				if found {
					t.Lists[n.ListName] = newList
					t.rebuildListStack(n.ListName)
				} else {
					t.emit(fmt.Sprintf("# Warn: Item %s not found in %s", n.Item, n.ListName))
				}
			}

		case *ListOpEdit:
			if list, ok := t.Lists[n.ListName]; ok {
				if n.Index >= 0 && n.Index < len(list) {
					list[n.Index] = n.NewValue
					t.Lists[n.ListName] = list
					t.rebuildListStack(n.ListName)
				} else {
					t.emit(fmt.Sprintf("# Error: Index %d out of bounds for %s", n.Index, n.ListName))
				}
			}

		case *ListOpFindBool:
			if list, ok := t.Lists[n.ListName]; ok {
				found := false
				for _, item := range list {
					if item == n.Target {
						found = true
						break
					}
				}
				val := "False"
				if found {
					val = "True"
				}
				t.emit(fmt.Sprintf("crt %s", n.VarName))
				t.emit(fmt.Sprintf("psh %s %s", n.VarName, val))
			}

		case *ListOpFindIndex:
			if list, ok := t.Lists[n.ListName]; ok {
				idx := -1
				for i, item := range list {
					if item == n.Target {
						idx = i
						break
					}
				}
				t.emit(fmt.Sprintf("crt %s", n.VarName))
				t.emit(fmt.Sprintf("psh %s %d", n.VarName, idx))
			}

		case *DictOpAdd:
			if dict, ok := t.Dicts[n.DictName]; ok {
				dict[n.Key] = n.Value
				t.Dicts[n.DictName] = dict
				t.rebuildDictStack(n.DictName)
			}

		case *DictOpDelete:
			if dict, ok := t.Dicts[n.DictName]; ok {
				if _, exists := dict[n.Key]; exists {
					delete(dict, n.Key)
					t.Dicts[n.DictName] = dict
					t.rebuildDictStack(n.DictName)
				}
			}

		case *DictOpEdit:
			if dict, ok := t.Dicts[n.DictName]; ok {
				if _, exists := dict[n.Key]; exists {
					dict[n.Key] = n.NewValue
					t.Dicts[n.DictName] = dict
					t.rebuildDictStack(n.DictName)
				}
			}

		case *DictOpFindBool:
			if dict, ok := t.Dicts[n.DictName]; ok {
				found := false
				for _, v := range dict {
					if v == n.TargetValue {
						found = true
						break
					}
				}
				val := "False"
				if found {
					val = "True"
				}
				t.emit(fmt.Sprintf("crt %s", n.VarName))
				t.emit(fmt.Sprintf("psh %s %s", n.VarName, val))
			}

		case *DictOpFindKey:
			if dict, ok := t.Dicts[n.DictName]; ok {
				foundKey := "None"
				for k, v := range dict {
					if v == n.TargetValue {
						foundKey = k
						break
					}
				}
				t.emit(fmt.Sprintf("crt %s", n.VarName))
				t.emit(fmt.Sprintf("psh %s %s", n.VarName, foundKey))
			}

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

// Helper to rebuild a List stack
func (t *Translator) rebuildListStack(name string) {
	items := t.Lists[name]

	t.emit(fmt.Sprintf("del %s_len", name))
	for i := range items {
		t.emit(fmt.Sprintf("del %s_%d", name, i))
	}

	t.emit(fmt.Sprintf("crt %s_len", name))
	t.emit(fmt.Sprintf("psh %s_len %d", name, len(items)))

	for i, item := range items {
		varName := fmt.Sprintf("%s_%d", name, i)
		t.emit(fmt.Sprintf("crt %s", varName))
		t.emit(fmt.Sprintf("psh %s %s", varName, item))
	}
}

// Helper to rebuild a Dict stack
func (t *Translator) rebuildDictStack(name string) {
	pairs := t.Dicts[name]

	for k := range pairs {
		t.emit(fmt.Sprintf("del %s_key_%s", name, k))
		t.emit(fmt.Sprintf("del %s_val_%s", name, k))
	}

	for k, v := range pairs {
		keyVar := fmt.Sprintf("%s_key_%s", name, k)
		valVar := fmt.Sprintf("%s_val_%s", name, k)

		t.emit(fmt.Sprintf("crt %s", keyVar))
		t.emit(fmt.Sprintf("psh %s %s", keyVar, k))

		t.emit(fmt.Sprintf("crt %s", valVar))
		t.emit(fmt.Sprintf("psh %s %s", valVar, v))
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

// invertOp returns the logical opposite of a comparison operator
func invertOp(op string) string {
	switch op {
	case "=":
		return "!="
	case "!=":
		return "="
	case "<":
		return ">="
	case ">":
		return "=<"
	case "=<":
		return ">"
	case ">=":
		return "<"
	default:
		return op // Fallback
	}
}

// parseCondition splits "left op right"
func parseCondition(cond string) (string, string, string) {
	parts := strings.Fields(cond)
	if len(parts) == 3 {
		return parts[0], parts[1], parts[2]
	}
	return "", "", ""
}

// executeBodyWithLoop handles the Loop sugar for a given body
func (t *Translator) executeBodyWithLoop(body []Node, loopCount string, localAliases map[string]string) {
	if loopCount == "" {
		t.translateBody(body, localAliases)
		return
	}

	// Check for infinite loop (-1)
	if loopCount == "-1" {
		// Create a recursive function to simulate infinite loop
		funcName := fmt.Sprintf("_inf_loop_%d", t.LabelCounter)
		t.LabelCounter++

		t.emit(fmt.Sprintf("fnc %s {", funcName))
		t.translateBody(body, localAliases)
		t.emit(fmt.Sprintf("cal %s", funcName)) // Recursive call
		t.emit("}")

		t.emit(fmt.Sprintf("cal %s", funcName))
		return
	}

	// Create a temporary LoopBlock to reuse existing logic
	tempLoop := &LoopBlock{
		Count: loopCount,
		Body:  body,
	}
	t.translateLoop(tempLoop, localAliases)
}

func (t *Translator) translateIf(ifBlock *IfBlock, localAliases map[string]string) {
	// Generate unique labels/functions for this If chain
	bodyFuncName := fmt.Sprintf("_if_body_%d", t.LabelCounter)
	t.LabelCounter++

	// Define the Body Function
	t.emit(fmt.Sprintf("fnc %s {", bodyFuncName))
	t.executeBodyWithLoop(ifBlock.Body, ifBlock.LoopCount, localAliases)
	t.emit("}")

	// Determine where to jump if condition is FALSE
	// If there is an Else or ElseIf, we jump to that handler.
	// If there is nothing, we just fall through (do nothing).

	var falseTargetFunc string

	if ifBlock.ElseIf != nil {
		// Create a wrapper for the ElseIf chain
		elseIfFuncName := fmt.Sprintf("_if_elseif_%d", t.LabelCounter)
		t.LabelCounter++
		t.emit(fmt.Sprintf("fnc %s {", elseIfFuncName))
		t.translateIf(ifBlock.ElseIf, localAliases) // Recursive call for ElseIf
		t.emit("}")
		falseTargetFunc = elseIfFuncName
	} else if ifBlock.ElseBody != nil {
		// Create a wrapper for the Else body
		elseFuncName := fmt.Sprintf("_if_else_%d", t.LabelCounter)
		t.LabelCounter++
		t.emit(fmt.Sprintf("fnc %s {", elseFuncName))
		t.executeBodyWithLoop(ifBlock.ElseBody, ifBlock.ElseLoopCount, localAliases)
		t.emit("}")
		falseTargetFunc = elseFuncName
	}

	// Evaluate Condition
	lLeft, lOp, lRight := parseCondition(ifBlock.LeftCond)

	if ifBlock.IsNot {
		lOp = invertOp(lOp)
	}

	// Handle AND/OR logic for the primary condition if needed
	if ifBlock.LogicOp == "and" {
		// AND: Left MUST be true to check Right. Right MUST be true to run Body.

		checkRightFunc := fmt.Sprintf("_if_and_right_%d", t.LabelCounter)
		t.LabelCounter++

		rLeft, rOp, rRight := parseCondition(ifBlock.RightCond)
		if ifBlock.IsNot {
			rOp = invertOp(rOp)
		}

		t.emit(fmt.Sprintf("fnc %s {", checkRightFunc))
		if falseTargetFunc != "" {
			t.emit(fmt.Sprintf("jmp %s %s %s cal %s", rLeft, rRight, rOp, bodyFuncName))
		} else {
			t.emit(fmt.Sprintf("jmp %s %s %s cal %s", rLeft, rRight, rOp, bodyFuncName))
		}
		t.emit("}")

		t.emit(fmt.Sprintf("jmp %s %s %s cal %s", lLeft, lRight, lOp, checkRightFunc))

	} else if ifBlock.LogicOp == "or" {
		// OR: If Left True -> Body. If Left False -> Check Right.
		lLeft, lOp, lRight := parseCondition(ifBlock.LeftCond)
		rLeft, rOp, rRight := parseCondition(ifBlock.RightCond)

		if ifBlock.IsNot {
			lOp = invertOp(lOp)
			rOp = invertOp(rOp)
		}

		orCheckFunc := fmt.Sprintf("_if_or_check_%d", t.LabelCounter)
		t.LabelCounter++

		t.emit(fmt.Sprintf("fnc %s {", orCheckFunc))
		t.emit(fmt.Sprintf("jmp %s %s %s cal %s", lLeft, lRight, lOp, bodyFuncName))
		t.emit(fmt.Sprintf("jmp %s %s %s cal %s", rLeft, rRight, rOp, bodyFuncName))
		t.emit("}")

		t.emit(fmt.Sprintf("cal %s", orCheckFunc))

	} else {
		// Simple Condition
		if lLeft != "" {
			if falseTargetFunc != "" {
				// If True -> Body. If False -> Execute Else/ElseIf

				wrapperFunc := fmt.Sprintf("_if_wrapper_%d", t.LabelCounter)
				t.LabelCounter++

				t.emit(fmt.Sprintf("fnc %s {", wrapperFunc))
				t.emit(fmt.Sprintf("jmp %s %s %s cal %s", lLeft, lRight, lOp, bodyFuncName))
				// If jump didn't happen, we are here (False).
				t.emit(fmt.Sprintf("cal %s", falseTargetFunc))
				t.emit("}")

				t.emit(fmt.Sprintf("cal %s", wrapperFunc))

			} else {
				// No Else. Just check.
				t.emit(fmt.Sprintf("jmp %s %s %s cal %s", lLeft, lRight, lOp, bodyFuncName))
			}
		} else {
			fmt.Printf("[Warn] Could not parse condition: %s\n", ifBlock.LeftCond)
		}
	}
}

func (t *Translator) emit(instr string) {
	t.Instructions = append(t.Instructions, instr)
}

// --- Cache Manager ---

type CacheManager struct {
	SourceDir   string
	BytecodeDir string
}

func NewCacheManager(baseDir string) *CacheManager {
	sourceDir := filepath.Join(baseDir, "source")
	bytecodeDir := filepath.Join(baseDir, "bytecode")
	os.MkdirAll(sourceDir, os.ModePerm)
	os.MkdirAll(bytecodeDir, os.ModePerm)
	return &CacheManager{
		SourceDir:   sourceDir,
		BytecodeDir: bytecodeDir,
	}
}

func (cm *CacheManager) GetSourcePath(name string) string {
	return filepath.Join(cm.SourceDir, name+".q")
}

func (cm *CacheManager) GetBytecodePath(name string) string {
	return filepath.Join(cm.BytecodeDir, name+".qb")
}

func calculateHash(content string) string {
	hash := md5.Sum([]byte(content))
	return hex.EncodeToString(hash[:])
}

// SplitProgramIntoUnits splits the program into logical units (Functions, Classes, Entrusts)
// and returns a map of unitName -> sourceCodeSnippet
func SplitProgramIntoUnits(prog *Program, originalSource string) map[string]string {
	units := make(map[string]string)

	for _, node := range prog.Nodes {
		var name string
		var content string

		switch n := node.(type) {
		case *FunctionDef:
			name = n.Name
			// Reconstruct function source
			content = fmt.Sprintf("Function \"%s\"", n.Name)
			if n.IsMain {
				content += " (Main)"
			}
			content += " {\n"
			content += reconstructBody(n.Body, 1)
			content += "}\n"

		case *ClassDef:
			name = n.Name
			content = fmt.Sprintf("Class \"%s\" {\n", n.Name)
			content += reconstructBody(n.Members, 1)
			content += "}\n"

		case *EntrustBlock:
			name = fmt.Sprintf("Entrust_%d", n.ID) // Use ID as name for uniqueness if no explicit name
			content = fmt.Sprintf("Entrust (%s) {\n", n.Condition)
			content += reconstructBody(n.Body, 1)
			content += "}\n"
		}

		if name != "" {
			units[name] = content
		}
	}

	return units
}

// Helper to reconstruct body indentation
func reconstructBody(nodes []Node, indentLevel int) string {
	var sb strings.Builder
	indent := strings.Repeat("\t", indentLevel)

	for _, node := range nodes {
		sb.WriteString(indent)
		sb.WriteString(NodeToString(node, indentLevel))
		sb.WriteString("\n")
	}
	return sb.String()
}

func NodeToString(n Node, indentLevel int) string {
	indent := strings.Repeat("\t", indentLevel)

	switch v := n.(type) {
	case *VarDef:
		return fmt.Sprintf("Data.Var %s %s = \"%s\"", v.VarType, v.Name, v.Value)
	case *ConsoleInfo:
		return fmt.Sprintf("Console.Info(\"%s\")", v.Content)
	case *ListDef:
		return fmt.Sprintf("DataStruct.List \"%s\" = %v", v.Name, v.Items)
	case *DictDef:
		return fmt.Sprintf("DataStruct.Dict \"%s\" = %v", v.Name, v.Pairs)
	case *LoopBlock:
		s := fmt.Sprintf("Loop(%s) {\n", v.Count)
		s += reconstructBody(v.Body, indentLevel+1)
		s += indent + "}"
		return s
	case *IfBlock:
		// Reconstruct If with logic keywords
		condStr := v.LeftCond
		if v.LogicOp != "" {
			condStr = fmt.Sprintf("%s %s %s", v.LeftCond, v.LogicOp, v.RightCond)
		}
		if v.IsNot {
			condStr = "not " + condStr
		}

		loopSuffix := ""
		if v.LoopCount != "" {
			loopSuffix = fmt.Sprintf(" Loop(%s)", v.LoopCount)
		}

		s := fmt.Sprintf("If(%s)%s {\n", condStr, loopSuffix)
		s += reconstructBody(v.Body, indentLevel+1)
		s += indent + "}"

		// Reconstruct Else
		if v.ElseIf != nil {
			s += "\n" + indent + NodeToString(v.ElseIf, indentLevel)
		} else if v.ElseBody != nil {
			elseLoopSuffix := ""
			if v.ElseLoopCount != "" {
				elseLoopSuffix = fmt.Sprintf(" Loop(%s)", v.ElseLoopCount)
			}
			s += "\n" + indent + fmt.Sprintf("Else%s {\n", elseLoopSuffix)
			s += reconstructBody(v.ElseBody, indentLevel+1)
			s += indent + "}"
		}

		return s
	case *CustomNode:
		return v.RawLine
	case *ListOpAdd:
		return fmt.Sprintf("DataStruct.ListAdd \"%s\" > \"%s\"", v.Value, v.ListName)
	case *ListOpDelete:
		return fmt.Sprintf("DataStruct.ListDelete \"%s\" > \"%s\"", v.Item, v.ListName)
	case *ListOpEdit:
		return fmt.Sprintf("DataStruct.ListEdit \"%s\" - %d > \"%s\"", v.ListName, v.Index, v.NewValue)
	case *ListOpFindBool:
		return fmt.Sprintf("DataStruct.ListFind.Bool \"%s\" - \"%s\" > \"%s\"", v.ListName, v.Target, v.VarName)
	case *ListOpFindIndex:
		return fmt.Sprintf("DataStruct.ListFind.Index \"%s\" - \"%s\" > \"%s\"", v.ListName, v.Target, v.VarName)
	case *DictOpAdd:
		return fmt.Sprintf("DataStruct.DictAdd \"%s\" - \"%s\" > \"%s\"", v.Key, v.Value, v.DictName)
	case *DictOpDelete:
		return fmt.Sprintf("DataStruct.DictDelete \"%s\" > \"%s\"", v.Key, v.DictName)
	case *DictOpEdit:
		return fmt.Sprintf("DataStruct.DictEdit \"%s\" - \"%s\" > \"%s\"", v.DictName, v.Key, v.NewValue)
	case *DictOpFindBool:
		return fmt.Sprintf("DataStruct.DictFind.Bool \"%s\" - \"%s\" > \"%s\"", v.DictName, v.TargetValue, v.VarName)
	case *DictOpFindKey:
		return fmt.Sprintf("DataStruct.DictFind.Key \"%s\" - \"%s\" > \"%s\"", v.DictName, v.TargetValue, v.VarName)
	default:
		return fmt.Sprintf("# Unknown Node: %T", v)
	}
}

// --- Main Execution ---

func main() {
	if len(os.Args) < 2 {
		fmt.Println("Usage: quern-translator --Run <file.q> [--UnuseDeadCodeEli] [--LgnWarning] [--ForceRep]")
		os.Exit(1)
	}

	if os.Args[1] != "--Run" || len(os.Args) < 3 {
		fmt.Println("Usage: quern-translator --Run <file.q> [--UnuseDeadCodeEli] [--LgnWarning] [--ForceRep]")
		os.Exit(1)
	}

	sourceFile := os.Args[2]
	modDir := "mods"
	cacheBaseDir := "cache"
	
	// Flags
	enableDCE := false
	ignoreWarnings := false // --LgnWarning
	forceStrict := false    // --ForceRep (Strict mode: refuse to compile if warnings exist)

	for _, arg := range os.Args[3:] {
		switch arg {
		case "--UnuseDeadCodeEli":
			enableDCE = true
		case "--LgnWarning":
			ignoreWarnings = true
		case "--ForceRep":
			forceStrict = true
		}
	}

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
	sourceStr := string(content)

	// 4. Parse
	parser := NewParser(sourceStr)
	prog, err := parser.Parse()
	if err != nil {
		fmt.Printf("[Parse Error] %v\n", err)
		os.Exit(1)
	}

	// 5. Dead Code Elimination & Warning Check
	var dce *DeadCodeEliminator
	
	// We always analyze for warnings if strict mode or warning output is needed, 
	// but we only filter (modify AST) if enableDCE is true.
	dce = NewDeadCodeEliminator()
	dce.Analyze(prog)

	// Check for issues before proceeding
	hasWarnings := false
	
	// Temporarily capture warnings to decide whether to block execution
	// We need to know if there ARE unused items, regardless of whether we print them.
	// The Filter method currently prints. Let's create a check method or modify logic.
	// For simplicity, we will run a check pass.
	
	unusedItems := checkForUnusedItems(dce, prog)
	if len(unusedItems) > 0 {
		hasWarnings = true
	}

	if hasWarnings {
		if forceStrict {
			fmt.Println("[Error] Strict Mode (--ForceRep): Unresolved warnings detected. Translation refused.")
			for _, item := range unusedItems {
				fmt.Printf("  - %s\n", item)
			}
			os.Exit(1)
		}
		
		if !ignoreWarnings {
			fmt.Println("[Warning] Unused code detected:")
			for _, item := range unusedItems {
				fmt.Printf("  - %s\n", item)
			}
		}
	}

	// Apply DCE filtering if enabled
	if enableDCE {
		fmt.Println("[Info] Running Dead Code Elimination...")
		prog = dce.Filter(prog)
		fmt.Println("[Info] Dead Code Elimination finished.")
	}

	// 6. Cache Management Logic
	cm := NewCacheManager(cacheBaseDir)

	// Split program into units
	units := SplitProgramIntoUnits(prog, sourceStr)

	hasChanges := false

	// Check each unit against cache
	for name, unitSource := range units {
		cachePath := cm.GetSourcePath(name)

		// Calculate hash of current unit
		currentHash := calculateHash(unitSource)

		// Check if cached file exists
		cachedContent, err := ioutil.ReadFile(cachePath)
		if err != nil {
			// File doesn't exist or error reading
			hasChanges = true
			// Save new source
			ioutil.WriteFile(cachePath, []byte(unitSource), 0644)
			fmt.Printf("[Cache] New unit detected: %s\n", name)
			continue
		}

		// Compare hashes
		cachedHash := calculateHash(string(cachedContent))
		if currentHash != cachedHash {
			hasChanges = true
			// Update source cache
			ioutil.WriteFile(cachePath, []byte(unitSource), 0644)
			fmt.Printf("[Cache] Unit changed: %s\n", name)
		} else {
			fmt.Printf("[Cache] Unit unchanged: %s\n", name)
		}
	}

	// Determine output bytecode path
	baseName := filepath.Base(sourceFile)
	nameWithoutExt := strings.TrimSuffix(baseName, filepath.Ext(baseName))
	outputBytecodePath := cm.GetBytecodePath(nameWithoutExt)

	var qbCode string

	if !hasChanges {
		// Check if bytecode exists
		if _, err := os.Stat(outputBytecodePath); err == nil {
			fmt.Println("[Info] No changes detected. Using cached bytecode.")
			qbBytes, err := ioutil.ReadFile(outputBytecodePath)
			if err != nil {
				fmt.Printf("[Error] Failed to read cached bytecode: %v\n", err)
				os.Exit(1)
			}
			qbCode = string(qbBytes)
		} else {
			fmt.Println("[Info] No changes in source units, but bytecode missing. Re-translating.")
			hasChanges = true
		}
	}

	if hasChanges {
		fmt.Println("[Info] Changes detected or bytecode missing. Translating...")
		translator := NewTranslatorWithMods(loader)
		qbCode = translator.Translate(prog)

		// Save Bytecode
		err = ioutil.WriteFile(outputBytecodePath, []byte(qbCode), 0644)
		if err != nil {
			fmt.Printf("[Error] Cannot write bytecode file: %v\n", err)
			os.Exit(1)
		}
		fmt.Printf("[Info] Bytecode saved to: %s\n", outputBytecodePath)
	}

	// 7. Run QVM
	cmd := exec.Command("./Qvm.exe", "--Run", outputBytecodePath)
	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr

	fmt.Println("--- Running QVM ---")
	err = cmd.Run()
	if err != nil {
		fmt.Printf("[Runtime Error] QVM execution failed: %v\n", err)
	}
}

// checkForUnusedItems performs a dry-run analysis to return a list of warning messages
// without modifying the program or printing directly.
func checkForUnusedItems(dce *DeadCodeEliminator, prog *Program) []string {
	var warnings []string

	for _, node := range prog.Nodes {
		warnMsg := ""
		switch n := node.(type) {
		case *FunctionDef:
			if !dce.usedFunctions[n.Name] && !n.IsMain {
				warnMsg = fmt.Sprintf("Unused Function: \"%s\"", n.Name)
			}
		case *ClassDef:
			if !dce.usedClasses[n.Name] {
				warnMsg = fmt.Sprintf("Unused Class: \"%s\"", n.Name)
			}
		case *VarDef:
			if !dce.usedVars[n.Name] {
				warnMsg = fmt.Sprintf("Unused Variable: %s", n.Name)
			}
		case *ListDef:
			if !dce.usedLists[n.Name] {
				warnMsg = fmt.Sprintf("Unused List: \"%s\"", n.Name)
			}
		case *DictDef:
			if !dce.usedDicts[n.Name] {
				warnMsg = fmt.Sprintf("Unused Dict: \"%s\"", n.Name)
			}
		}

		if warnMsg != "" {
			warnings = append(warnings, warnMsg)
		}
	}
	return warnings
}

