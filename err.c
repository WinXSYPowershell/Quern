#include <stdio.h>

#ifdef _WIN32
#define strdup _strdup
#endif
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef struct {
    char** items;
    int size;
    int capacity;
} Stack;

void init_stack(Stack* s) {
    s->capacity = 16;
    s->size = 0;
    s->items = (char**)malloc(s->capacity * sizeof(char*));
}

void push_stack(Stack* s, const char* val) {
    if (s->size == s->capacity) {
        s->capacity *= 2;
        s->items = (char**)realloc(s->items, s->capacity * sizeof(char*));
    }
    s->items[s->size++] = strdup(val);
}

void pop_stack(Stack* s) {
    if (s->size > 0) {
        free(s->items[--s->size]);
    }
}

void free_stack(Stack* s) {
    for (int i = 0; i < s->size; i++) free(s->items[i]);
    free(s->items);
    s->items = NULL;
    s->size = 0;
    s->capacity = 0;
}

void print_top(Stack* s) {
    if (s->size > 0) {
        printf("%s ", s->items[s->size - 1]);
    } else {
        printf("(empty) ");
    }
}

int compare_stacks(Stack* l, Stack* r, const char* op) {
    if (l->size == 0 || r->size == 0) return 0;
    char* lv = l->items[l->size - 1];
    char* rv = r->items[r->size - 1];
    
    char* lend;
    char* rend;
    double ln = strtod(lv, &lend);
    double rn = strtod(rv, &rend);
    
    int is_num = (*lend == '\0' && *rend == '\0' && lend != lv && rend != rv);
    
    if (is_num) {
        if (strcmp(op, "=") == 0) return (ln - rn) < 1e-9 && (ln - rn) > -1e-9;
        if (strcmp(op, "!=") == 0) return (ln - rn) >= 1e-9 || (ln - rn) <= -1e-9;
        if (strcmp(op, "<") == 0) return ln < rn;
        if (strcmp(op, ">") == 0) return ln > rn;
        if (strcmp(op, "=<") == 0) return ln <= rn;
        if (strcmp(op, ">=") == 0) return ln >= rn;
    } else {
        int cmp = strcmp(lv, rv);
        if (strcmp(op, "=") == 0) return cmp == 0;
        if (strcmp(op, "!=") == 0) return cmp != 0;
        if (strcmp(op, "<") == 0) return cmp < 0;
        if (strcmp(op, ">") == 0) return cmp > 0;
        if (strcmp(op, "=<") == 0) return cmp <= 0;
        if (strcmp(op, ">=") == 0) return cmp >= 0;
    }
    return 0;
}

// --- Variable (@name@) runtime support ---
typedef struct { const char* name; Stack* s; } StackRef;
StackRef __stack_registry[256];
int __stack_registry_len = 0;

void register_stack(const char* name, Stack* s) {
    if (__stack_registry_len < 256) {
        __stack_registry[__stack_registry_len].name = name;
        __stack_registry[__stack_registry_len].s = s;
        __stack_registry_len++;
    }
}

Stack* find_stack(const char* name) {
    for (int i = 0; i < __stack_registry_len; i++) {
        if (strcmp(__stack_registry[i].name, name) == 0) return __stack_registry[i].s;
    }
    return NULL;
}

const char* get_top(Stack* s) {
    return (s != NULL && s->size > 0) ? s->items[s->size - 1] : "";
}

const char* get_top_by_name(const char* name) {
    Stack* s = find_stack(name);
    return (s != NULL && s->size > 0) ? s->items[s->size - 1] : "";
}

void out_variable(Stack* var_stack) {
    if (var_stack == NULL || var_stack->size == 0) {
        printf("(undefined) ");
        return;
    }
    const char* val = var_stack->items[var_stack->size - 1];
    Stack* target = find_stack(val);
    if (target != NULL) {
        if (target->size > 0) printf("%s ", target->items[target->size - 1]);
        else printf("(empty) ");
    } else {
        printf("%s ", val);
    }
}

// --- Variable value register (New Feature 2 backing store) ---
// A per-name numeric register decoupled from the physical stack, so that
// arithmetic on @name@ accumulates a value that survives .
typedef struct { char* name; char* val; } VarVal;
VarVal __var_store[256];
int __var_store_len = 0;

const char* var_get(const char* name) {
    for (int i = 0; i < __var_store_len; i++)
        if (strcmp(__var_store[i].name, name) == 0) return __var_store[i].val;
    return NULL;
}

void var_set(const char* name, const char* val) {
    Stack* s = find_stack(name);
    // keep the physical top in sync when the stack has a value
    if (s != NULL && s->size > 0) {
        free(s->items[s->size - 1]);
        s->items[s->size - 1] = strdup(val);
    }
    for (int i = 0; i < __var_store_len; i++) {
        if (strcmp(__var_store[i].name, name) == 0) {
            free(__var_store[i].val);
            __var_store[i].val = strdup(val);
            return;
        }
    }
    if (__var_store_len < 256) {
        __var_store[__var_store_len].name = strdup(name);
        __var_store[__var_store_len].val = strdup(val);
        __var_store_len++;
    }
}

// out @name@: prefer the numeric register; fall back to the original
// indirect (value-is-a-stack-name) resolution when the register is unset.
void out_varval(const char* name) {
    const char* val = var_get(name);
    if (val == NULL) { out_variable(find_stack(name)); return; }
    Stack* target = find_stack(val);
    if (target != NULL) {
        if (target->size > 0) printf("%s ", target->items[target->size - 1]);
        else printf("(empty) ");
    } else {
        printf("%s ", val);
    }
}

// --- Arithmetic push (New Feature 2): psh <stack> <op> "<operand>" ---
// base = current register value of <stack> (or its stack top, or 0);
// res = base OP operand; the result is written back to the register.
void psh_arith(const char* name, const char* op, const char* operand) {
    double base = 0.0;
    const char* cur = var_get(name);
    if (cur != NULL) {
        base = strtod(cur, NULL);
    } else {
        Stack* s = find_stack(name);
        if (s != NULL && s->size > 0) base = strtod(s->items[s->size - 1], NULL);
    }
    double rhs = strtod(operand, NULL);
    double res = base;
    if (strcmp(op, "+") == 0)      res = base + rhs;
    else if (strcmp(op, "-") == 0) res = base - rhs;
    else if (strcmp(op, "*") == 0) res = base * rhs;
    else if (strcmp(op, "/") == 0) res = (rhs != 0.0) ? (base / rhs) : 0.0;

    char buf[64];
    if (res == (long long)res) snprintf(buf, sizeof(buf), "%lld", (long long)res);
    else                       snprintf(buf, sizeof(buf), "%g", res);
    var_set(name, buf);
    // Fix: sync physical stack top so get_top_by_name returns updated value
    Stack* __psh_arith_s = find_stack(name);
    if (__psh_arith_s != NULL) {
        if (__psh_arith_s->size > 0) {
            free(__psh_arith_s->items[__psh_arith_s->size - 1]);
            __psh_arith_s->size--;
        }
        push_stack(__psh_arith_s, buf);
    }
}

int compare_values(const char* lv, const char* rv, const char* op) {
    if (lv == NULL || rv == NULL) return 0;
    char* lend;
    char* rend;
    double ln = strtod(lv, &lend);
    double rn = strtod(rv, &rend);
    int is_num = (*lend == '\0' && *rend == '\0' && lend != lv && rend != rv);
    if (is_num) {
        if (strcmp(op, "=") == 0) return (ln - rn) < 1e-9 && (ln - rn) > -1e-9;
        if (strcmp(op, "!=") == 0) return (ln - rn) >= 1e-9 || (ln - rn) <= -1e-9;
        if (strcmp(op, "<") == 0) return ln < rn;
        if (strcmp(op, ">") == 0) return ln > rn;
        if (strcmp(op, "=<") == 0) return ln <= rn;
        if (strcmp(op, ">=") == 0) return ln >= rn;
    } else {
        int cmp = strcmp(lv, rv);
        if (strcmp(op, "=") == 0) return cmp == 0;
        if (strcmp(op, "!=") == 0) return cmp != 0;
        if (strcmp(op, "<") == 0) return cmp < 0;
        if (strcmp(op, ">") == 0) return cmp > 0;
        if (strcmp(op, "=<") == 0) return cmp <= 0;
        if (strcmp(op, ">=") == 0) return cmp >= 0;
    }
    return 0;
}
Stack _100;
Stack cunters;

void qb_exit();
void qb_loop1();

void qb_exit() {
}

void qb_loop1() {
    if (compare_values(get_top_by_name("cunters"), "100", ">=")) qb_exit();
    printf("Hello Number:");printf("%s", get_top_by_name("cunters"));
    psh_arith("cunters", "+", "1");
    qb_loop1();
}

int main() {
    register_stack("100", &_100);
    register_stack("cunters", &cunters);
    init_stack(&_100);
    init_stack(&cunters);
    push_stack(&cunters, "1"); var_set("cunters", "1");
    qb_loop1();
    free_stack(&_100);
    free_stack(&cunters);
    return 0;
}