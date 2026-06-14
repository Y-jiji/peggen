# Predictive Dispatch for Ordered Choice

## Goal

For PEG ordered choices where alternatives begin with distinct bytes, replace sequential try-and-backtrack with a single table lookup that jumps directly to the correct branch. This eliminates backtracking, memo overhead, and O(N) rule attempts at choice points.

## Dispatch Table

The proc macro computes a FIRST set (set of possible first bytes) for each alternative in an ordered choice. When all FIRST sets are pairwise disjoint, it emits a 256-entry lookup table:

```rust
// Example: Json with 6 alternatives, all disjoint FIRST sets
static DISPATCH: [u8; 256] = {
    let mut table = [0xFF; 256];  // 0xFF = no match
    table[b'n' as usize] = 0;    // -> Null    ("null")
    table[b't' as usize] = 1;    // -> Bool    ("true"|"false")
    table[b'f' as usize] = 1;    // -> Bool
    table[b'-' as usize] = 2;    // -> Num     (digit or minus)
    for b in b'0'..=b'9' { table[b as usize] = 2; }
    table[b'"' as usize] = 3;    // -> Str
    table[b'{' as usize] = 4;    // -> Obj
    table[b'[' as usize] = 5;    // -> Arr
    table
};

fn parse_impl(input: &[u8], pos: usize, ...) -> Result<usize, ()> {
    let branch = DISPATCH[input[pos] as usize];
    match branch {
        0 => parse_null(input, pos, ...),
        1 => parse_bool(input, pos, ...),
        2 => parse_num(input, pos, ...),
        3 => parse_str(input, pos, ...),
        4 => parse_obj(input, pos, ...),
        5 => parse_arr(input, pos, ...),
        _ => Err(()),
    }
}
```

Cost: one array index + one match. No backtracking, no memo needed for this rule.

## Computing FIRST Sets

The proc macro computes `first(element) -> Set<u8>` for each grammar element:

| Pattern | FIRST |
|---------|-------|
| `"literal"` | `{ literal[0] }` |
| `$0:regex` | First bytes matchable by the regex |
| `$0` (non-terminal of type T) | `union of first(alt) for all alts of T` (recursive) |
| `e1 e2` (sequence) | `first(e1)` if e1 cannot match empty; `first(e1) ∪ first(e2)` if it can |
| `e?` / `e*` | `first(e) ∪ first(next_element)` (can match empty) |
| `!e` / `&e` | Lookahead — doesn't consume, FIRST comes from what follows |

This is a standard fixed-point computation over the grammar. The domain is finite (|alternatives| x 256 bytes), so convergence is guaranteed.

## Partial Predictiveness

Not all choices are fully predictive. Some alternatives may have overlapping FIRST sets:

```rust
enum Expr {
    #[rule($0:ident "(" $1 *% (_ "," _) ")")]  // FIRST = ident chars
    Call(String, Vec<Expr>),
    #[rule($0:ident)]                            // FIRST = ident chars (overlap!)
    Var(String),
}
```

### Strategy: Ambiguous Groups

The dispatch table partitions alternatives into predictive singletons (direct jump) and ambiguous groups (ordered choice within the group):

```rust
const PREDICTIVE_BRANCH: u8 = 0;   // direct dispatch
const AMBIGUOUS_GROUP_1: u8 = 1;   // ordered choice fallback

static DISPATCH: [u8; 256] = {
    let mut table = [0xFF; 256];
    // predictive branches
    table[b'{' as usize] = PREDICTIVE_BRANCH;
    // ambiguous group: ident-starting bytes
    for b in b'a'..=b'z' { table[b as usize] = AMBIGUOUS_GROUP_1; }
    for b in b'A'..=b'Z' { table[b as usize] = AMBIGUOUS_GROUP_1; }
    table
};

match branch {
    PREDICTIVE_BRANCH => parse_obj(input, pos, ...),
    AMBIGUOUS_GROUP_1 => {
        // only alternatives with overlapping FIRST, tried in order
        parse_call(input, pos, ...)
            .or_else(|_| parse_var(input, pos, ...))
    }
    _ => Err(()),
}
```

Even partial predictiveness wins: if 5 out of 7 alternatives are predictive, those 5 never participate in backtracking.

### Alternative: Multi-byte Lookahead

When single-byte FIRST sets overlap but the alternatives diverge after a common prefix, a two-stage dispatch can resolve the ambiguity:

1. Dispatch on first byte to select the ambiguous group.
2. Parse the common prefix (e.g., `ident`).
3. Dispatch on the next byte after the prefix (`(` vs other).

This turns a two-alternative backtracking choice into two predictive steps.

### Alternative: Grammar Factoring

The user can rewrite the grammar to factor out the common prefix:

```rust
// Before: overlapping FIRST sets
#[rule($0:ident "(" $1 *% (_ "," _) ")")]  Call(String, Vec<Expr>)
#[rule($0:ident)]                           Var(String)

// After: common prefix factored out
#[rule($0:ident $1)]  IdentExpr(String, IdentTail)
// IdentTail dispatches on "(" vs end — disjoint FIRST sets
```

This is a grammar-level concern, not an optimization the proc macro applies automatically.

## What This Eliminates

| Without dispatch | With dispatch |
|------------------|---------------|
| Try alt 0, fail, try alt 1, fail, ... try alt N, succeed | One table lookup, jump to alt N |
| O(N) rule attempts per choice point | O(1) |
| Memo needed to avoid re-parsing failed attempts | No memo needed for predictive rules |
| Backtracking state for each failed attempt | No backtracking state |

## Interaction with Other Optimizations

- **Memo table**: predictive rules need no memoization — the dispatch is deterministic. The memo table is only needed for non-predictive (ambiguous group) rules.
- **Dyck pairs / commit points**: a predictive dispatch that lands on a bracket-opening rule can immediately commit. The bracket's pair guarantees the close exists; the dispatch guarantees no backtracking to sibling alternatives.
- **Structural index**: the dispatch table byte can be read directly from the structural index instead of the raw input, avoiding cache misses on the input buffer for large documents.

## Compile-Time Cost

| Step | Cost |
|------|------|
| FIRST set computation | O(|rules| x 256), iterated to fixed point |
| Disjointness check | O(|alternatives|^2 x 256) per choice |
| Table generation | O(256) per choice |

Trivial for any real grammar.
