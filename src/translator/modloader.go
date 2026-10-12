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

