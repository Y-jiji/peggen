# Dyck Language Analysis for Bracket Pair Detection

## Goal

Given a PEG grammar, statically identify which terminal symbols form **definite stack pairs** — terminals (L, R) such that in every possible derivation, L and R appear in properly nested (Dyck language) structure. These pairs enable jump-over optimizations: once L is matched, R's position is known from a pre-computed pair table, allowing O(1) subtree skipping.

## Definitions

- **Terminal**: a literal string in a rule (e.g., `"{"`, `"]"`, `"begin"`).
- **Non-terminal**: a type referenced by a field (e.g., `$0` of type `Json`).
- **Candidate pair**: any (L, R) where L and R are distinct terminals appearing as the first and last element of some rule.
- **Definite stack pair**: a candidate pair that satisfies all three safety conditions below.

## Safety Conditions

A candidate pair (L, R) is a definite stack pair **if and only if** all three conditions hold.

### Condition 1: Consistent Pairing

In every rule across the entire grammar:

- If L appears, then R appears exactly once to its right at the same nesting level within that rule, and no other terminal serves as L's closer.
- If R appears, then L appears exactly once to its left at the same nesting level within that rule, and no other terminal serves as R's opener.
- No rule uses L without R, or R without L.

**Counterexample:**
```rust
enum Foo {
    #[rule("{" $0 "}")]    // { closes with }
    #[rule("{" $0 ";")]    // { closes with ; — INCONSISTENT
}
// "{" is not part of any definite pair.
```

### Condition 2: Balanced Sub-grammars

Every non-terminal reachable between L and R positions (transitively, through all fields and their types) must produce L and R in balanced pairs, or not produce them at all. Formally, every such non-terminal is classified `never` or `balanced` — never `unbalanced`. See the classification algorithm below.

**Counterexample:**
```rust
enum Template {
    #[rule("{" $0 "}")]
    Block(Body),
}
enum Body {
    #[rule("}" $0)]        // produces } without {
    Escape(Expr),
}
// balance(Body, ({,})) = unbalanced → ({, }) is NOT safe.
```

### Condition 3: No Prefix Ambiguity

L is not a prefix of R, R is not a prefix of L, and neither L nor R is a prefix of any other terminal in a definite pair. This prevents the structural scanner from misidentifying one delimiter as part of another.

**Counterexample:**
```rust
enum Template {
    #[rule("{" $0 "}")]      // single braces
    Expr(Expr),
    #[rule("{{" $0 "}}")]    // double braces
    Interp(Expr),
}
// "{" is a prefix of "{{" — scanner cannot distinguish without parsing.
// Neither pair is safe for structural pre-scanning.
```

## Classification Algorithm

### Step 1: Terminal Reachability

Compute which terminals each non-terminal can transitively produce.

```
produces: NonTerminal -> Set<Terminal>

Initialize: produces(N) = {} for all N

Repeat until stable:
  For each non-terminal N:
    For each rule of N:
      produces(N) ∪= { all literal terminals in this rule }
      For each field $x with type T:
        produces(N) ∪= produces(T)
```

Standard fixed-point iteration. The domain is finite (|NonTerminals| x |Terminals|), so convergence is guaranteed.

### Step 2: Balance Classification

For each candidate pair (L, R), classify every non-terminal.

```
balance: NonTerminal x (L, R) -> { never, balanced, unbalanced }

Initialize: balance(N, (L,R)) = never  for all N where
            L ∉ produces(N) and R ∉ produces(N)
            (use reachability from Step 1 to prune early)

Repeat until stable:
  For each non-terminal N:
    For each rule of N:
      own_L = count of L in this rule's terminals
      own_R = count of R in this rule's terminals

      For each field $x with type T:
        if balance(T, (L,R)) == unbalanced:
          mark this rule unbalanced; break

      if any field is unbalanced:
        rule_class = unbalanced
      else if own_L == own_R AND properly_nested(rule, L, R):
        rule_class = balanced
      else if own_L == 0 AND own_R == 0 AND all fields are never:
        rule_class = never
      else:
        rule_class = unbalanced

    N's classification = join across all rules:
      if ANY rule is unbalanced -> unbalanced
      if ALL rules are never    -> never
      otherwise                 -> balanced
```

**`properly_nested(rule, L, R)`**: checks that within the rule's own terminal sequence, treating `balanced` sub-fields as opaque, the L's and R's form valid Dyck nesting. For example, rule `L ... L ... R ... R` is nested; rule `L ... R ... L` is not.

### Step 3: Pair Validation

For each candidate pair (L, R):

1. Check Condition 1 across all rules in the grammar.
2. For every non-terminal reachable between L and R in any rule, verify `balance(N, (L,R)) != unbalanced`.
3. Check Condition 3 against all other terminals and pairs.

If all three pass, (L, R) is emitted into the bracket alphabet.

## Top-Level Determination

The bracket alphabet is a property of `Parser::<T>`, NOT of `T` alone. The same type can have different safe pairs depending on what grammar it is embedded in.

```rust
// Parser::<Json>::new() → ({, }) and ([, ]) are safe pairs
// Parser::<MarkDown>::new() → ({, }) may NOT be safe if MarkDown
//   or any of its sub-grammars produce unbalanced { or }
```

The reason: at `#[derive(Parse)]` time, the proc macro only sees one type. It cannot know what other grammars will embed it. A type's brackets are safe only in the context of the full reachable grammar rooted at the top-level `T`.

### Local vs Global

Each type contributes **local facts** at derive time:

```rust
trait ParseImpl {
    /// Which terminals appear in which positions in which rules.
    const LOCAL_TERMINAL_INFO: &[RuleTerminalInfo];
    /// Which non-terminal types are referenced as fields.
    const FIELD_TYPES: &[TypeId];
}
```

`Parser::<T>::new()` assembles the **global analysis**:

1. Walk `FIELD_TYPES` transitively from `T` to collect all reachable types.
2. Gather `LOCAL_TERMINAL_INFO` from every reachable type.
3. Run the full Dyck analysis (reachability, balance classification, validation).
4. Store the resulting bracket alphabet in the parser instance.

```rust
impl<T: ParseImpl> Parser<T> {
    fn new() -> Self {
        let pairs = dyck_analyze::<T>();  // microseconds for any real grammar
        Self { pairs, .. }
    }
}
```

The grammar analysis runs once per `Parser::new()` call and takes microseconds — it operates on the grammar structure (tens of rules and terminals), not on input data.

The **runtime pair resolution** (Stage 1.5) is a separate concern: it walks the structural index of a specific input and is O(n) over the input size. For gigabyte inputs, this is a measurable cost. Whether to run it depends on the parse mode:

| Mode | Strategy | Rationale |
|---|---|---|
| Full parse (entire AST) | Pair inline during Stage 2 via depth stack + backpatching (simdjson's approach) | No extra pass; pairing is a side effect of tape construction |
| On-demand / selective parse | Separate Stage 1.5 pair resolution pass | Pair info needed upfront to skip subtrees without entering them |

### Full Parse: Inline Pairing (simdjson approach)

simdjson does no bracket pairing in Stage 1. Stage 1 emits a flat array of byte positions. Pairing happens in Stage 2 using a depth-indexed container stack and tape backpatching:

1. On open bracket: reserve a tape slot, record its index in `open_containers[depth]`, increment depth.
2. Parse contents sequentially.
3. On close bracket: decrement depth, backpatch the reserved slot with the current tape position, write end entry pointing back to start.

This creates bidirectional links in the tape with zero extra passes. The stack is bounded by max nesting depth (tiny). For peggen's full-parse mode, the same approach applies.

### Generality: Inline Pairing Works for Any Dyck Language

The depth-stack approach is not specific to JSON's two bracket types. It works for any number of pair types, as long as the identified pairs form a valid Dyck language (which the analysis above guarantees).

**Proof sketch.** The defining property of a Dyck language is proper nesting: the most recently opened bracket must be closed first. This is exactly the invariant the depth stack relies on — at any moment, there is at most one open bracket per depth level. A single depth counter handles all pair types:

```
( { [ ] [ ] } { } )     — 3 pair types, single depth counter

depth 0: (               open_containers[0] = (
depth 1:   {             open_containers[1] = {
depth 2:     [           open_containers[2] = [
depth 1:       ]         close [, backpatch ✓
depth 2:         [       open_containers[2] = [  (reuse slot)
depth 1:           ]     close [, backpatch ✓
depth 0:             }   close {, backpatch ✓
depth 1:               { open_containers[1] = {  (reuse slot)
depth 0:                 }  close {, backpatch ✓
         )               close (, backpatch ✓
```

Sequential pairs at the same depth reuse the same slot (the previous pair is already closed). Nested pairs use deeper slots. No conflicts arise.

**Cross-pair interleaving is impossible.** If `( { ) }` were derivable, the sub-grammar between `(` and `)` would produce `{` without `}`. The balance classification catches this as `unbalanced`, so `({, })` and/or `((, ))` would be excluded from the bracket alphabet. The per-pair Dyck analysis is sufficient to guarantee all identified pairs together form a valid multi-type Dyck language.

**Match validation is free.** In simdjson, `}` always closes `{` and `]` always closes `[` because JSON's grammar enforces it. In peggen, the PEG rule structure provides the same guarantee — a rule that opens with `{` must close with `}`. No runtime type-checking of bracket pairs against the depth stack is needed; it is implicit in rule matching.

### On-demand Parse: Upfront Pair Table

When subtrees may be skipped without parsing their contents, the pair table must be available before Stage 2 begins. This justifies a separate Stage 1.5 linear pass over the structural index.

`Parser::<T>` can provide both paths, selecting based on how `parse()` is called.

### Why Not Compile-Time Const

A `const` associated item on `T` cannot see other types' derive outputs — Rust's proc macros are per-item. The full grammar graph is only available when all types are compiled, which means the assembly must happen either:

- At `Parser::<T>::new()` time (runtime, trivially cheap), or
- Via a separate whole-grammar proc macro invoked after all types are defined.

The `new()` approach is simpler and sufficient — the analysis cost is negligible compared to any single `parse()` call.

## Output

The bracket alphabet is stored in the `Parser` instance, not as a type-level constant:

```rust
struct Parser<T> {
    pairs: Vec<(Terminal, Terminal)>,
    // ...
}
```

Only pairs that survive all three conditions are included. Conservatism is correct: a missed pair loses a performance optimization; a false pair causes incorrect parsing.

## Runtime Usage

### Stage 1.5: Pair Resolution

After Stage 1 (SIMD structural scan) produces a flat array of structural character positions, a linear pass pairs the brackets:

```
Input:  structural_index — array of byte positions
Output: pair_table       — partner[i] = structural index of matching bracket

Algorithm:
  stack = []
  For i in 0..structural_index.len():
    pos = structural_index[i]
    if input[pos] matches an L in PAIRS:
      stack.push(i)
    else if input[pos] matches an R in PAIRS:
      open_idx = stack.pop()
      partner[open_idx] = i     // open -> close
      partner[i] = open_idx     // close -> open

Cost: O(n) time, O(max_nesting_depth) stack space
```

### Jump-over Operations

With the pair table, three operations become O(1):

| Operation | Method |
|-----------|--------|
| Skip subtree in tape | Open entry stores partner tape index; jump to it |
| Find close position in input | `structural_index[pair_table.partner[i]]` |
| Bound backtracking | Interior of a committed pair cannot backtrack past the open |

## Complexity

### Compile-time (proc macro)

| Step | Cost |
|------|------|
| Terminal reachability | O(\|rules\| x \|terminals\|), iterated to fixed point |
| Balance classification | O(\|rules\| x \|candidate_pairs\|), iterated to fixed point |
| Pair validation | O(\|candidate_pairs\| x \|rules\|) |
| Total | O(\|rules\| x \|terminals\|^2) worst case |

For any real grammar (tens of rules, tens of terminals), this completes in microseconds.

### Runtime

| Step | Cost |
|------|------|
| Stage 1.5 pair resolution | O(n) time, O(depth) space |
| Jump-over lookup | O(1) per query |

The pair table is a flat `Vec<u32>` with the same length as the structural index. No hashing, no pointer chasing.

## Limitations

- **Context-sensitive brackets**: if a terminal acts as a bracket in one embedding but not another (e.g., `"` inside a raw string context), the analysis correctly rejects it — the sub-grammar that allows unbalanced usage causes `unbalanced` classification.
- **Multi-character delimiters**: supported (e.g., `"```"`) but require substring matching in the structural scanner instead of single-byte comparison, which may reduce SIMD effectiveness.
- **Self-pairing delimiters**: terminals where L == R (e.g., `"` pairs with `"`) require special handling — the structural scanner cannot distinguish open from close. These are excluded from the bracket alphabet unless the grammar guarantees alternation (which is hard to verify statically).
