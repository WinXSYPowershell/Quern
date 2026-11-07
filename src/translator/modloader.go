package main

import (
	"fmt"
	"io/ioutil"
	"os"
	"path/filepath"
	"reflect"
	"regexp"
	"strings"

	"github.com/dop251/goja"
)

// --- Mod Loader ---

type SyntaxHandler func(input string) (string, error)

type ModLoader struct {
	vm            *goja.Runtime
	handlers      map[string]SyntaxHandler
	loadedModules map[string]bool
	dataStructs   *DataStructures
}

// DataStructures holds the runtime data structures that JS modules can access
type DataStructures struct {
	Lists  map[string][]string
	Dicts  map[string]map[string]string
	VarDefs map[string]string
}

func NewModLoader() *ModLoader {
	vm := goja.New()
	loader := &ModLoader{
		vm:            vm,
		handlers:      make(map[string]SyntaxHandler),
		loadedModules: make(map[string]bool),
	}
	loader.injectAPIs()
	return loader
}

func (ml *ModLoader) injectAPIs() {
	vm := ml.vm
	quernObj := vm.NewObject()

	// Quern.Log - lets modules output logs
	quernObj.Set("Log", func(call goja.FunctionCall) goja.Value {
		if len(call.Arguments) > 0 {
			msg := call.Argument(0).String()
			fmt.Printf("[Mod Log] %s\n", msg)
		}
		return goja.Undefined()
	})

	// Quern.Reg - register custom syntax handlers
	// Usage: Quern.Reg("keyword", function(inputText) { return "QVM code"; })
	// Quern.Reg本质上是指定一串自定义的文本，写入各种自定义参数，然后让translator翻译为对应Qvm代码
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

	// Quern.DataStruct - lets modules access Quern code written data structures
	// This exposes Lists, Dicts, and VarDefs to JavaScript
	quernObj.Set("DataStruct", func(call goja.FunctionCall) goja.Value {
		// Return an object with access to the data structures
		dsObj := vm.NewObject()

		// Expose Lists as a JS object
		listsObj := vm.NewObject()
		if ml.dataStructs != nil && ml.dataStructs.Lists != nil {
			for k, v := range ml.dataStructs.Lists {
				// Convert Go slice to JS array
				jsArray := vm.NewArray()
				for i, item := range v {
					jsArray.Set(fmt.Sprintf("%d", i), vm.ToValue(item))
				}
				listsObj.Set(k, jsArray)
			}
		}
		dsObj.Set("Lists", listsObj)

		// Expose Dicts as a JS object
		dictsObj := vm.NewObject()
		if ml.dataStructs != nil && ml.dataStructs.Dicts != nil {
			for k, v := range ml.dataStructs.Dicts {
				jsDict := vm.NewObject()
				for dk, dv := range v {
					jsDict.Set(dk, vm.ToValue(dv))
				}
				dictsObj.Set(k, jsDict)
			}
		}
		dsObj.Set("Dicts", dictsObj)

		// Expose VarDefs as a JS object
		varsObj := vm.NewObject()
		if ml.dataStructs != nil && ml.dataStructs.VarDefs != nil {
			for k, v := range ml.dataStructs.VarDefs {
				varsObj.Set(k, vm.ToValue(v))
			}
		}
		dsObj.Set("Vars", varsObj)

		// Add helper method to get list length
		dsObj.Set("GetListLength", func(call goja.FunctionCall) goja.Value {
			if len(call.Arguments) < 1 {
				return goja.Undefined()
			}
			listName := call.Argument(0).String()
			if ml.dataStructs != nil && ml.dataStructs.Lists != nil {
				if list, ok := ml.dataStructs.Lists[listName]; ok {
					return vm.ToValue(len(list))
				}
			}
			return vm.ToValue(0)
		})

		// Add helper method to push item to list
		dsObj.Set("PushToList", func(call goja.FunctionCall) goja.Value {
			if len(call.Arguments) < 2 {
				return goja.Undefined()
			}
			listName := call.Argument(0).String()
			item := call.Argument(1).String()
			if ml.dataStructs != nil && ml.dataStructs.Lists != nil {
				if list, ok := ml.dataStructs.Lists[listName]; ok {
					ml.dataStructs.Lists[listName] = append([]string{item}, list...)
				} else {
					ml.dataStructs.Lists[listName] = []string{item}
				}
			}
			return goja.Undefined()
		})

		// Add helper method to set dict value
		dsObj.Set("SetDictValue", func(call goja.FunctionCall) goja.Value {
			if len(call.Arguments) < 3 {
				return goja.Undefined()
			}
			dictName := call.Argument(0).String()
			key := call.Argument(1).String()
			value := call.Argument(2).String()
			if ml.dataStructs != nil && ml.dataStructs.Dicts != nil {
				if dict, ok := ml.dataStructs.Dicts[dictName]; ok {
					dict[key] = value
				} else {
					ml.dataStructs.Dicts[dictName] = map[string]string{key: value}
				}
			}
			return goja.Undefined()
		})

		// Add helper method to get dict value
		dsObj.Set("GetDictValue", func(call goja.FunctionCall) goja.Value {
			if len(call.Arguments) < 2 {
				return goja.Undefined()
			}
			dictName := call.Argument(0).String()
			key := call.Argument(1).String()
			if ml.dataStructs != nil && ml.dataStructs.Dicts != nil {
				if dict, ok := ml.dataStructs.Dicts[dictName]; ok {
					if val, ok := dict[key]; ok {
						return vm.ToValue(val)
					}
				}
			}
			return goja.Undefined()
		})

		return dsObj
	})

	vm.Set("Quern", quernObj)
}

// SetDataStructures exposes Quern runtime data structures to JS modules
// This allows JS modules to read and modify Lists, Dicts, and VarDefs defined in Quern code
func (ml *ModLoader) SetDataStructures(lists map[string][]string, dicts map[string]map[string]string, vars map[string]string) {
	ml.dataStructs = &DataStructures{
		Lists:     lists,
		Dicts:     dicts,
		VarDefs:   vars,
	}
}

// LoadModule loads a specific JavaScript module by name from the mods directory
// This is called when the parser encounters an "Import X" statement
func (ml *ModLoader) LoadModule(name string) error {
	// Check if already loaded
	if ml.loadedModules[name] {
		fmt.Printf("[Mod] Module '%s' already loaded, skipping.\n", name)
		return nil
	}

	modDir := "mods"
	moduleFile := name + ".js"
	fullPath := filepath.Join(modDir, moduleFile)   // ← 第一轮：找 name.js

	fmt.Printf("[Info] Loading mod: %s\n", moduleFile)

	if _, err := os.Stat(fullPath); os.IsNotExist(err) {
		// Try without .js extension
		fullPath = filepath.Join(modDir, name)      // ← fallback：找 name
		if _, err := os.Stat(fullPath); os.IsNotExist(err) {
			return fmt.Errorf("mod file not found: %s (searched in %s/%s and %s/%s)",
				name, modDir, moduleFile, modDir, name)
		}
	}
	content, err := ioutil.ReadFile(fullPath)
	if err != nil {
		return fmt.Errorf("failed to read mod %s: %v", name, err)
	}

	visited := make(map[string]bool)
	visited[fullPath] = true
	mergedCode, err := ProcessIncludes(filepath.Dir(fullPath), string(content), visited)
	if err != nil {
		return fmt.Errorf("failed to process includes for %s: %v", name, err)
	}

	_, err = ml.vm.RunString(mergedCode)
	if err != nil {
		return fmt.Errorf("failed to execute mod %s: %v", name, err)
	}

	ml.loadedModules[name] = true
	fmt.Printf("[Mod] Successfully loaded module: %s\n", name)
	return nil
}

// LoadModsFromDirectory loads all JavaScript modules from the mods directory
// This is a convenience method for loading all mods at startup
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

		moduleName := strings.TrimSuffix(f.Name(), ".js")
		if err := ml.LoadModule(moduleName); err != nil {
			fmt.Printf("[Error] Failed to load mod %s: %v\n", moduleName, err)
		}
	}

	return nil
}

// GetHandler returns the syntax handler registered for a given keyword
func (ml *ModLoader) GetHandler(keyword string) (SyntaxHandler, bool) {
	handler, ok := ml.handlers[keyword]
	return handler, ok
}

// ProcessIncludes processes Include statements in JavaScript files,
// recursively merging included files into the parent file content.
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
