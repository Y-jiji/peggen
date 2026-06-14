# Design Decisions

## Optimization Status

### Implemented / Designed

| Optimization | Document | Status |
|---|---|---|
| Predictive dispatch | PREDICTIVE_DISPATCH.md | Designed |
| Dyck bracket pairing | DYCK_LANGUAGE.md | Designed |
| Eager eviction | EAGER_EVICTION.md | Designed |
| Memo ring buffer | (inline discussion) | Designed |
| SIMD structural scan | (inline discussion) | Designed |
| Fuzz testing | FUZZ_TESTING.md | Designed |

### Decisions

| Topic | Decision | Rationale |
|---|---|---|
| SIMD terminal matching | Suspended | Non-regex literal matching is already efficient (memcmp). Regex terminals may benefit later but are not the bottleneck. |
| On-demand / lazy parsing | Not pursuing | Full AST construction is the target use case. On-demand adds API complexity for a mode that may not be needed. |
| Arena allocation | Keep as policy | No extra heap allocation during `parse()` calls. All buffers pre-allocated in `Parser::new()`, reused across calls. Arena (bumpalo) for AST child nodes remains optional. |
| Multi-threading | Not pursuing | Single-core throughput is the optimization target. Parallel chunked parsing adds synchronization complexity disproportionate to the gain for most grammars. |
| Chunked/streaming input | Deferred | Eager eviction makes this natural — intermediate state is O(depth), so only a bounded window of input needs to be resident. Design the interface later once eager eviction is implemented. |
| Error reporting | Good to have | See section below. |
| Left recursion | Keep current approach | See section below. |

---

## Core Invariant: No Speculative Object Construction

**A concrete object (String, Vec, Box, etc.) is constructed only when exactly one of two outcomes is possible:**

1. **The parser returns an error.** The constructed object is dropped along with everything else.
2. **The parser returns a result, and that result contains the constructed object.**

There is no third case. A constructed object is never discarded by backtracking.

### Tags vs Objects

Tags (`{ span: Range<usize>, rule: usize }`) are two integers. They are cheap to produce and free to discard (Vec truncation). Tags are fine in speculative regions — their purpose is to defer expensive object construction until it is safe.

Objects (`String`, `Vec<Json>`, `Box<Expr>`, etc.) involve heap allocation. Constructing them speculatively and then discarding them on backtrack wastes allocator work. The invariant prevents this.

| Phase | Tags | Objects |
|---|---|---|
| Speculative (before commit) | Pushed freely, discarded on backtrack | Never constructed |
| Committed (after commit point) | Consumed and evicted | Constructed from consumed tags |

### Current Problem

The current two-phase design defers ALL object construction to after the full parse. This is safe (no speculative construction), but the AST construction phase must reverse-traverse the entire tag stack via O(n) recursion (the `rev` function in `PushImpl`). This causes:

- O(n) tag storage (proportional to input, not nesting depth)
- O(n) call stack depth during AST construction
- Dependence on `stacker` to avoid stack overflow on large inputs

### The Fix: Eager Eviction

At each commit point, consume the tags for the committed region and construct the objects immediately. Then evict the consumed tags. The tag stack stays bounded by speculative depth (typically small), not input size.

In committed repetitions, this turns the recursive `rev` traversal into a flat loop:

```
// Before (deferred): parse all → tag stack of N elements → rev(N) with O(N) recursion
// After (eager): for each element, parse → construct → push into Vec → evict tags
```

Elements are constructed in natural left-to-right order and appended directly. No reversal, no recursion.

### What Qualifies as a Commit Point

A commit point is reached when backtracking past this position is impossible:

| Commit point | Guarantee |
|---|---|
| Predictive dispatch landed | FIRST sets are disjoint — no alternative to backtrack to |
| Dyck bracket opened | Closing bracket is guaranteed; interior failure is a hard error |
| Repetition separator consumed | Previous element is final; cannot be un-consumed |
| Last alternative in ordered choice | No remaining alternatives — succeed or hard error |

### Interaction with the Memo Table

The memo table only exists for speculative regions — rules where backtracking is possible. After a commit point, the memo is irrelevant (the path is determined). Since construction only happens after commit, the memo never stores constructed objects — only `Result<usize, ()>` (position or failure). This keeps the memo entries small and evictable.

---

## Error Reporting

### Post-Commit Errors (Easy)

After a commit point (predictive dispatch, Dyck bracket, repetition separator), failure is a hard error with rich context. The parser knows:

- **Which rule** was being parsed.
- **Which position** in the input the failure occurred.
- **What was expected** — the next literal, the close bracket, or the FIRST set of the next element.

These errors are precise and require no extra bookkeeping:

```
error: expected "}" at position 42, found "," 
  in rule Json::Obj, opened "{" at position 0
```

Predictive dispatch misses are equally informative:

```
error: unexpected byte 'x' at position 0
  expected one of: '{', '[', '"', 'n', 't', 'f', '-', '0'-'9'
```

### Speculative Errors (Hard)

In speculative regions (ambiguous groups with overlapping FIRST sets), failed alternatives are silently discarded per PEG semantics. When ALL alternatives fail, which error to report?

**Strategy: furthest failure position.**

Track the deepest position the parser reached across all failed alternatives. When the parse ultimately fails, report the error at that position — it indicates where the parser got closest to succeeding:

```rust
struct ParseContext {
    // ...
    furthest_failure: usize,       // deepest position reached before failure
    furthest_rule: usize,          // which rule was active at that position
    furthest_expected: Expected,   // what was expected at that position
}

// On every failure:
fn record_failure(&mut self, pos: usize, rule: usize, expected: Expected) {
    if pos >= self.furthest_failure {
        self.furthest_failure = pos;
        self.furthest_rule = rule;
        self.furthest_expected = expected;
    }
}
```

This adds one comparison per failure (cheap) and produces errors like:

```
error: parse failed
  furthest match at position 37: expected digit in rule Json::Num
  (after successfully matching "[1, 2, ")
```

### Interaction with Optimizations

| Optimization | Effect on error reporting |
|---|---|
| Predictive dispatch | Makes errors better — FIRST set gives exact expected bytes |
| Dyck pairing | Makes errors better — unmatched bracket errors include open position |
| Eager eviction | No effect — errors occur before construction, eviction only happens after commit |
| Memo ring buffer | Memo hit means the rule already failed — error was already recorded on the first attempt |

Error reporting quality improves with more commit points because more errors are post-commit (precise) rather than speculative (furthest-failure heuristic).

---

## Left Recursion

### Can All PEGs Be Rewritten Without Left Recursion?

Not equivalently. Left recursion can be mechanically eliminated to accept the same **language**, but the transformation changes parse tree **structure**:

- Left-recursive rules produce left-associative trees.
- Elimination rewrites them to right-recursive, producing right-associative trees.
- `1 - 2 - 3` must parse as `(1 - 2) - 3` (left-associative), not `1 - (2 - 3)` (right-associative).

In the presence of semantic actions (which peggen has — AST variant construction), restoring correct associativity after elimination is difficult and in general cases impossible.

Sources:
- [Tratt: Direct Left-Recursive Parsing Expression Grammars](https://tratt.net/laurie/research/pubs/html/tratt__direct_left_recursive_parsing_expression_grammars/)
- [Medeiros & Ierusalimschy: Left Recursion in PEGs (ScienceDirect)](https://www.sciencedirect.com/science/article/pii/S0167642314000288)
- [Redziejowski: More About Left Recursion in PEG](https://ceur-ws.org/Vol-2240/paper9.pdf)

### Current Approach: Bounded Left Recursion via Depth Parameter

The current peggen design handles left recursion with a `depth` parameter that bounds the recursion. This is essentially an iterative deepening approach: try the left-recursive rule at increasing depths until it stops matching.

For an expression like `1 + 2 + 3`, each `+` triggers a full re-parse from `start` with `first=true`. Cost is O(rules x operators) per precedence level.

### New Approach: Tag-State Machine (Generalized Pratt)

Tags are not "levels" or "binding powers." They are arbitrary named groups. A rule's tag set defines which groups it belongs to. The tag set of a parsed result determines which operators can bind to it next.

#### Tags as States

```rust
enum Expr {
    #[tag(a)]          #[rule($0@a "+" $1@b)]    Add(...)
    #[tag(a)]          #[rule($0@a "-" $1@c)]    Sub(...)
    #[tag(a, b)]       #[rule($0@b "*" $1@d)]    Mul(...)
    #[tag(a, c)]       #[rule($0@c "/" $1@d)]    Div(...)
    #[tag(a, b, c, d)] #[rule($0:num)]           Num(...)
}
```

Tag structure is a DAG, not a line:

```
    a
   / \
  b   c
   \ /
    d
```

Each parsed result carries a tag set (its rule's `#[tag(...)]` list). That tag set is a **state** that determines which operators can follow:

| Parsed result | Tag set (state) | Available operators |
|---|---|---|
| `Num` | `{a, b, c, d}` | `+`, `-`, `*`, `/` — all |
| `Mul` | `{a, b}` | `+`, `-`, `*` — not `/` (requires `c`) |
| `Div` | `{a, c}` | `+`, `-`, `/` — not `*` (requires `b`) |
| `Add` | `{a}` | `+`, `-` only |
| `Sub` | `{a}` | `+`, `-` only |

An operator `$0@G ... $1@H` requires tag `G` in the left operand's state. After binding, the result's state is the operator rule's own tag set. The state shrinks as operators bind.

#### Transition Table

The proc macro builds a transition table from the grammar:

| Left state contains | Operator literal | Right operand parsed at | Result state |
|---|---|---|---|
| `a` | `+` | group `b` | `{a}` |
| `a` | `-` | group `c` | `{a}` |
| `b` | `*` | group `d` | `{a, b}` |
| `c` | `/` | group `d` | `{a, c}` |

#### Disambiguation Requirement

For the state machine to dispatch deterministically, each reachable state must map each lookahead literal to at most one operator — the same requirement as LR parsing.

**The check:** For every reachable tag set (state) `S`, collect all `(literal, tag, rule)` triples where `tag ∈ S`. Group by literal. Any literal that maps to more than one rule is a **conflict**.

Example of a conflict: if state `{a, b}` is reachable, and tag `a` gates operator `"+"` producing result state `{a}`, and tag `b` also gates operator `"+"` producing result state `{a, b}`, the state machine cannot determine which rule to apply on seeing `"+"`. This is analogous to a reduce/reduce conflict in LR.

**Reachable states** are computed by the proc macro at compile time:

1. Start from every primary rule's tag set (these are the initial states).
2. For each state `S`, enumerate all operators whose required tag is in `S`. Each operator produces a new state (the operator rule's own tag set). Add it to the reachable set.
3. Fixed-point iteration until no new states appear.

**On conflict**, the proc macro has two options:

| Strategy | When |
|---|---|
| **Reject** | The grammar is ill-formed for the tag-state machine. Emit a compile error identifying the conflicting operators and the state that triggers the ambiguity. |
| **Fall back** | Demote the conflicting operators out of the state machine and handle them via ordered PEG choice (standard backtracking). The remaining conflict-free operators stay in the loop. |

A conflict-free grammar produces a deterministic dispatch table: at each state, peek one literal, look up the unique operator. No backtracking, no ordering dependency.

#### The Loop

A single flat loop dispatches on the operator literal, guarded by the left operand's tag state. Because the grammar is conflict-free (verified at compile time), each `(state, literal)` pair maps to exactly one operator — the dispatch is deterministic:

```
left = parse_primary()              // Num → state = {a, b, c, d}
loop:
    match (left.state, peek):
        (contains b, "*") →
            right = parse_at(group d)
            left = Mul(left, right)     // state = {a, b}
        (contains c, "/") →
            right = parse_at(group d)
            left = Div(left, right)     // state = {a, c}
        (contains a, "+") →
            right = parse_at(group b)
            left = Add(left, right)     // state = {a}
        (contains a, "-") →
            right = parse_at(group c)
            left = Sub(left, right)     // state = {a}
        _ → break
```

No ordering dependency between arms — the conflict-free property guarantees at most one arm matches for any `(state, literal)` pair. The dispatch can be implemented as a lookup table indexed by `(state_id, first_byte)`.

#### Linear Chains as a Special Case

When the tag DAG happens to be a linear chain (like `calc.rs`: `add ⊃ mul ⊃ atom`), this reduces to standard Pratt parsing with binding powers. The flat loop becomes equivalent to nested `parse_at(bp)` calls. But the tag-state machine handles arbitrary DAGs without assuming linearity.

#### Rule Classification

The proc macro classifies each rule:

| Classification | Pattern | Detection |
|---|---|---|
| **Infix** | `$left@group ... literal ... <rest>` | Starts with a tagged field ref where the group is in the rule's own tag set (left-recursive), followed by a dispatchable literal. Handler body after the literal can be arbitrarily complex (binary, ternary, postfix-with-brackets, etc.) |
| **Prefix** | `literal ... $operand@group` | Starts with a literal, no left-recursive self-reference at head |
| **Primary** | No left recursion | No self-referencing field at head position |

Ternary and multi-operand operators fit as infix — the dispatch key is the first literal after the left operand, and the handler body parses the rest:

```rust
#[tag(ternary, ...)]
#[rule($0@ternary _ "?" _ $1@add _ ":" _ $2@ternary)]
Ternary(Box<Expr>, Box<Expr>, Box<Expr>),

#[tag(postfix, ...)]
#[rule($0@postfix "[" $1@add "]")]
Index(Box<Expr>, Box<Expr>),
```

#### Generated Code Structure

```
parse_at(group G):
    // 1. Try prefix operators whose tag set includes G
    left = try_prefix_neg() or try_prefix_not() or ...
    // 2. If no prefix matched, try primaries whose tag set includes G
    ... or try_primary_at(G)
    // 3. Infix loop: tag-state dispatch
    loop:
        for each infix operator:
            if left.state contains operator's required left tag
               AND peek matches operator's literal:
                parse right operand at operator's right group
                left = wrap(left, op, right)
                left.state = operator rule's tag set
                continue loop
        break
    return left
```

#### What Stays as Standard PEG

- Rules without tags — no tag state, no infix loop
- Rules where left recursion doesn't start with a tagged field followed by a dispatchable literal
- Groups where operator literals overlap (no predictive dispatch between operators at the same required tag)
- Indirect left recursion that crosses type boundaries

These fall back to the current bounded-depth approach, which remains as a general-purpose fallback.

#### Comparison

| Aspect | Iterative deepening (current) | Tag-state machine (new) |
|---|---|---|
| Time complexity | O(rules x operators) per group | O(n) total |
| Associativity | Correct (via re-parse loop) | Correct (via infix loop, natural left-to-right) |
| Backtracking | Re-parses from start on each extension | No backtracking — operator dispatch is predictive |
| Tag structure | Assumes linear precedence | Arbitrary DAG |
| Tag emission | Tags emitted during re-parse, old ones discarded | Tags emitted once per operator; tag state tracked |
| `depth` / `first` / `trace` | Required for left recursion guard | Not needed for classified rules |

#### Interaction with Other Optimizations

| Optimization | Interaction |
|---|---|
| Predictive dispatch | Operator dispatch IS predictive dispatch — the operator literal is the dispatch key |
| Dyck pairing | Compatible — infix loop operates within bracket scopes |
| Eager eviction | Natural fit — each loop iteration produces a committed result; the previous left operand is final |
| No speculative construction | Loop only constructs after successfully parsing operands — no wasted construction |
| Memo ring buffer | Classified rules don't need memoization — no backtracking, no re-parsing |

### Decision

Use the tag-state machine for rules that fit the infix/prefix pattern (detected automatically by the proc macro). Tags are arbitrary named groups forming a DAG — not assumed to be linear. The tag set of a parsed result determines which operators can follow. Fall back to bounded-depth iterative deepening for rules that don't fit. The change is in code generation, not grammar syntax.
