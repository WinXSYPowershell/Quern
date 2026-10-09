package main

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
	ElseBody      []Node   // Code block for Else
	ElseIf        *IfBlock // Linked list for Else If
	ElseLoopCount string   // If not empty, the Else body acts as a loop body. "-1" means infinite.
	LoopCount     string   // If not empty, the If/Else body acts as a loop body
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
	Name  string
	Pairs map[string]string
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
