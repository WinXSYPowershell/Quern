package main

import (
	"fmt"
	"io/ioutil"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
)

// --- Main Execution ---

func main() {
	if len(os.Args) < 2 {
		fmt.Println("Usage: quern-translator --Run <file.q> [--UnuseDeadCodeEli] [--LgnWarning] [--ForceRep] [--ExpandLoop] [--ConstCalc]")
		os.Exit(1)
	}

	if os.Args[1] != "--Run" || len(os.Args) < 3 {
		fmt.Println("Usage: quern-translator --Run <file.q> [--UnuseDeadCodeEli] [--LgnWarning] [--ForceRep] [--ExpandLoop] [--ConstCalc]")
		os.Exit(1)
	}

	sourceFile := os.Args[2]
	modDir := "mods"
	cacheBaseDir := "cache"

	// Flags
	enableDCE := false
	ignoreWarnings := false // --LgnWarning
	forceStrict := false    // --ForceRep (Strict mode: refuse to compile if warnings exist)
	extandLoop := false     // --ExpandLoop (use old static unroll method for loops)
	constCalc := false      // --ConstCalc (pre-evaluate expressions at translation time)
	for _, arg := range os.Args[3:] {
		switch arg {
		case "--UnuseDeadCodeEli":
			enableDCE = true
		case "--LgnWarning":
			ignoreWarnings = true
		case "--ForceRep":
			forceStrict = true
		case "--ExpandLoop":
			extandLoop = true
		case "--ConstCalc":
			constCalc = true
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
		translator.expandLoop = extandLoop
		translator.constCalc = constCalc
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

