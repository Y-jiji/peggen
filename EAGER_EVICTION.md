# Eager Eviction: Fused Parse and Construct

## Goal

Reduce the intermediate tag stack from O(n) (proportional to input length) to O(depth) (proportional to nesting depth) by constructing actual Rust return types at commit points and immediately evicting consumed tags.

## Problem: Deferred Construction

In a two-phase design, parsing produces a flat tag stack, then a second pass constructs the AST:

```
Input:  [1, 2, 3, ..., 1000000]

Phase 1: parse entire input → 1,000,000+ tags on stack
Phase 2: walk stack → construct 1,000,000 Json::Num values → Vec<Json>
```

Stack size is O(n). For gigabyte inputs, the tag stack alone can consume gigabytes of memory, defeating the goal of bounded intermediate state.

## Solution: Construct at Commit Points

Once a commit point is reached, the completed sub-element will never be backtracked over. Construct the actual Rust type immediately and discard the tags.

```
Input:  [1, 2, 3, ..., 1000000]

parse "[" → commit (Dyck pair, predictive dispatch)
parse "1" → tags: [Num@0..1]
  → construct Json::Num(1), push into Vec<Json>
  → evict tags. stack is empty.
parse ","
parse "2" → tags: [Num@2..3]
  → construct Json::Num(2), push into Vec<Json>
  → evict tags. stack is empty.
...
parse "]" → close bracket, return Vec<Json>
```

The tag stack never holds more than one element's worth of tags. Completed values go directly into the output structure.

## Commit Point Safety

Eager construction is safe only when backtracking cannot undo it. Three kinds of commit points guarantee this:

| Commit point | Why safe |
|---|---|
| Predictive dispatch landed | No sibling alternative to backtrack to (see PREDICTIVE_DISPATCH.md) |
| Dyck bracket opened | Close bracket is guaranteed; interior failure is a hard error, not backtracking (see DYCK_LANGUAGE.md) |
| Repetition separator consumed | Previous element is final; only forward progress or hard error from here |

Before any commit point (speculative region), only track positions — do not construct. After commit, construct eagerly. Failure past a commit point is a parse error reported to the caller, not a backtrack.

## Fused Code Generation

The proc macro generates a single function that parses and constructs simultaneously, instead of separate `ParseImpl` and `AstImpl`:

```rust
// Generated for: #[rule("[" _ $0 *% (_ "," _) _ "]")]
fn parse_and_build(
    input: &str, pos: usize, ctx: &mut ParseContext
) -> Result<(usize, Vec<Json>), Error> {
    let pos = match_literal(input, pos, "[")?;
    ctx.commit(pos);

    let mut items = Vec::new();
    let pos = skip(input, pos);

    // first element
    let (pos, item) = Json::parse_and_build(input, pos, ctx)?;
    items.push(item);

    // remaining elements
    loop {
        let pos = skip(input, pos);
        match match_literal(input, pos, ",") {
            Ok(pos) => {
                ctx.commit(pos);
                let pos = skip(input, pos);
                let (pos, item) = Json::parse_and_build(input, pos, ctx)?;
                items.push(item);
            }
            Err(_) => break,
        }
    }

    let pos = skip(input, pos);
    let pos = match_literal(input, pos, "]")?;
    Ok((pos, items))
}
```

No tag stack in the hot path. Values are constructed inline and pushed directly into the output.

## Speculative Regions

Tags survive only in speculative regions — before a commit point, where backtracking might discard the work:

```rust
// Ambiguous group: Call vs Var (overlapping FIRST sets, no predictive dispatch)
fn parse_ambiguous(
    input: &str, pos: usize, ctx: &mut ParseContext
) -> Result<(usize, Expr), Error> {
    let checkpoint = ctx.save();
    match parse_call_speculative(input, pos, ctx) {
        Ok((pos, tags)) => {
            ctx.commit(pos);
            Ok((pos, build_call(input, tags)))
        }
        Err(_) => {
            ctx.restore(checkpoint);
            let (pos, var) = parse_var(input, pos, ctx)?;
            Ok((pos, var))
        }
    }
}
```

The tag stack is a small working buffer for speculative parsing, bounded by the maximum speculative depth — typically a few entries.

## Memory Profile

| Design | Intermediate state | Construction |
|---|---|---|
| Two-phase (deferred) | O(n) — proportional to input length | Batch, after full parse |
| Eager eviction | O(depth) — proportional to nesting depth | Incremental, at commit points |

For a 1GB JSON array with 10M elements at nesting depth 3: deferred construction allocates ~160MB of tags; eager eviction uses ~48 bytes of working stack.

## Failure as a Rare Case

This design optimizes for the success path. Construction happens eagerly, assuming the parse will succeed. If it fails past a commit point, the already-constructed values are dropped — but this is a hard error anyway, so the cost is irrelevant.

The only wasted work from eager construction would occur in speculative regions, where construction is deliberately deferred. The combination of predictive dispatch (eliminating most speculation) and Dyck commit points (bounding the rest) means the vast majority of parsing follows the eager path.

## Interaction with Other Optimizations

- **Predictive dispatch**: determines that a branch is committed, enabling eager construction of its contents.
- **Dyck pairs**: bracket open commits the interior, enabling eager construction of each element within.
- **Memo table**: only needed for speculative (non-committed) rules. Predictive + eager rules bypass the memo entirely.
- **Structural index**: commit points align with structural characters, so the structural index drives both dispatch and eviction decisions.
