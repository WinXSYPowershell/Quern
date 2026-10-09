package main

import (
	"fmt"
	"strconv"
	"strings"
)

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
	// value = strings.Trim(value, "\"")

	return &VarDef{Name: name, Value: value, IsPrivate: isPrivate, VarType: varType}, nil
}

func (p *Parser) parseConsoleInfo(line string) (*ConsoleInfo, error) {
	start := strings.Index(line, "(")
	end := strings.LastIndex(line, ")")
	if start == -1 || end == -1 {
		return nil, fmt.Errorf("Invalid Console.Info syntax")
	}
	content := line[start+1 : end]
	// content = strings.Trim(content, "\"")
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
				// item = strings.Trim(item, "\"")
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

