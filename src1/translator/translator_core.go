package main

import (
	"fmt"
	"regexp"
	"strconv"
	"strings"

	"github.com/dop251/goja"
)

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
	expandLoop bool
	constCalc  bool
	jsVm       *goja.Runtime
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
		expandLoop:   false,
		constCalc:    false,
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
			// ConstCalc: if enabled, always try to evaluate expressions at translation time
			if t.constCalc {
				val = t.evaluateExpression(val)
			} else if n.VarType == "Int" || n.VarType == "Num" {
				// Evaluate numeric expressions if type is Int or Num
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
			// ConstCalc: pre-evaluate expressions in Console.Info content
			if t.constCalc {
				content = t.evaluateExpression(content)
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
	// If --ExpandLoop is set, use the old static unroll method
	if t.expandLoop {
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
	return
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

