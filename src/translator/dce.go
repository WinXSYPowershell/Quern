package main

import (
	"fmt"
)

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
