package main

import (
	"crypto/md5"
	"encoding/hex"
	"fmt"
	"io/ioutil"
	"os"
	"path/filepath"
	"strings"
)

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

