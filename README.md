# Peggen

A parser generator for parsing expression grammar (PEG) that uses inline macros to specify PEG operations. Grammars are written as bare Rust tokens inside `#[rule(...)]` attributes — no escape-heavy format strings.

```rust
use peggen::*;

#[derive(Debug, Parse)]
#[regex(_    = r"\s*")]
#[regex(num  = r"0|-?[1-9][0-9]*")]
#[regex(flt  = r"-?(0|[1-9][0-9]*)\.([0-9]+)")]
#[regex(str  = r#"[^"]*"#)]
#[regex(bool = r"false|true")]
#[subrule(kv = "\"" $0:str "\"" _ ":" _ $1)]
pub enum Json {
    #[rule("null")]
    Null,
    #[rule($0:bool)]
    Bool(bool),
    #[rule($0:flt)]
    Flt(f32),
    #[rule($0:num)]
    Num(i32),
    #[rule("\"" $0:str "\"")]
    Str(String),
    #[rule("{" _ $0:kv *% (_ "," _) _ "}")]
    Obj(Vec<(String, Json)>),
    #[rule("[" _ $0 *% (_ "," _) _ "]")]
    Arr(Vec<Json>),
}

fn main() {
    let json = Parser::<Json>::parse(r#"{"x": 1, "y": [2, 3]}"#).unwrap();
    println!("{json:?}");
}
```

## Rule Syntax Reference

Rules are written as bare Rust tokens inside `#[rule(...)]`. A LALRPOP-generated LR(1) parser verifies the grammar is unambiguous at build time.

### Terminals

| Syntax | Meaning | Example |
|--------|---------|---------|
| `"text"` | Match a string literal | `"null"`, `"{"`, `","` |

All terminals are string literals. Use `"\""` to match a literal double-quote character.

### Field References

| Syntax | Meaning |
|--------|---------|
| `$0`, `$field` | Parse field 0 / named field as a non-terminal |
| `$0:name` | Parse field using a named regex or subrule |
| `$0@tag` | Parse field at a specific precedence tag level |
| `${0,1}:name` | Parse using a subrule, unpacking into fields 0 and 1 |

### Bare Identifiers (Skip Patterns)

A bare identifier (not prefixed with `$`) references a `#[regex(...)]` or a `#[subrule(...)]` without field captures. The matched input is consumed and discarded.

```rust
#[regex(_ = r"\s*")]     // whitespace skip
#[regex(ws = r"[ \t]*")] // horizontal whitespace only
```

### Repetition

| Syntax | Meaning |
|--------|---------|
| `e*` | Zero or more |
| `e+` | One or more |
| `e?` | Optional (zero or one) |
| `e *% sep` | Zero or more, separated by `sep` |
| `e +% sep` | One or more, separated by `sep` |

Separated repetition (`*%`, `+%`) does **not** allow a trailing separator.

### Grouping

| Syntax | Meaning |
|--------|---------|
| `( e )` | Transparent grouping (PEG) |

### PEG Operators

| Syntax | Meaning |
|--------|---------|
| `e1 e2` | Sequence — match `e1` then `e2` (no implicit spacing) |
| `e1 \| e2` | Ordered choice — try `e1`, on failure try `e2` |
| `!e` | Negative lookahead — succeed if `e` fails, consume nothing |
| `&e` | Positive lookahead — succeed if `e` succeeds, consume nothing |

Negative lookahead with a bare identifier can be used for refutation:

```rust
#[regex(kw = r"int\b")]
#[regex(alpha = r"[A-Za-z]+")]
#[rule(!kw $0:alpha)]    // match alpha, but not the keyword "int"
```

### Whitespace

Sequences are **tight by default** — no whitespace is skipped between elements. Declare a regex skip pattern (conventionally `_`) and place it explicitly where spacing is allowed:

```rust
#[regex(_ = r"\s*")]
#[rule($0@add _ "+" _ $1@mul)]  // whitespace allowed around the operator
#[rule("\"" $0:str "\"")]       // tight — no space between quotes and content
```

## Attributes

### `#[regex(name = r"pattern")]`

Declare a named regex pattern. Referenced in rules as `$field:name` (captured) or as a bare `name` (discarded).

```rust
#[derive(Debug, Parse)]
#[regex(num = r"[0-9]+")]
pub struct Number(#[rule($0:num)] i32);
```

### `#[tag(name1, name2, ...)]`

Declare which precedence levels a variant participates in. Use `$field@tag` in rules to parse at a specific tag level.

If any variant has `#[tag(...)]`, all variants with `#[rule]` must have it. The first declared tag (index 0) is the root level used by `Parser::<T>::parse()`.

```rust
#[derive(Debug, Parse)]
#[regex(_ = r"\s*")]
#[regex(id = r"[a-z]+")]
pub enum Expr {
    #[tag(add)]
    #[rule($0@add _ "+" _ $1@mul)]
    Add(Box<Expr>, Box<Expr>),

    #[tag(add, mul)]
    #[rule($0@mul _ "*" _ $1@atom)]
    Mul(Box<Expr>, Box<Expr>),

    #[tag(add, mul, atom)]
    #[rule($0:id)]
    Ident(String),

    #[tag(add, mul, atom)]
    #[rule("(" _ $0@add _ ")")]
    Scope(Box<Expr>),
}
```

### `#[subrule(name = pattern)]`

Declare a reusable grammar fragment. A subrule's `$0`, `$1`, etc. are its own positional outputs, independent of the parent rule's fields. The caller decides how to map them:

| Reference | Meaning |
|-----------|---------|
| `$field:name` | Capture subrule output into a single field (tuple or scalar) |
| `${f1,f2}:name` | Unpack subrule outputs into separate parent fields |
| `name` | Match and discard (only for subrules without field captures) |

```rust
#[derive(Debug, Parse)]
#[regex(_ = r"\s*")]
#[regex(str = r#"[^"]*"#)]
#[subrule(kv = "\"" $0:str "\"" _ ":" _ $1)]
pub enum Json {
    // subrule kv captures (String, Json) — $0:kv repeats into Vec<(String, Json)>
    #[rule("{" _ $0:kv *% (_ "," _) _ "}")]
    Obj(Vec<(String, Json)>),
    // ...
}
```

## Derive Macros

| Macro | Purpose |
|-------|---------|
| `Parse` | All-in-one: generates `ParseImpl`, `RuleImpl`, `AstImpl`, `Num` |
| `ParseImpl` | Generates `ParseImpl` and `RuleImpl` traits only |
| `EnumAstImpl` | Generates `AstImpl` for enums using rule-based AST construction |
| `FromStrAstImpl` | Generates `AstImpl` using `FromStr` conversion |
| `Num` | Generates unique rule numbering |

For simple cases, `derive(Parse)` is all that is needed. Use the individual derives (`ParseImpl`, `Num`, `EnumAstImpl`) when building types across crate boundaries or when custom `AstImpl` is desired.

## How is it different from (...)?

| / | Conceptual | User Experience | Performance |
|---|------------|-----------------|-------------|
| [PEST](https://pest.rs) | PEST only annotates text. Peggen generates AST directly. | PEST requires manually creating enums from parse results. Peggen maps directly to Rust types. | PEST uses memoization; Peggen avoids it, which is often faster for typical grammars. |
| [Chumsky](https://crates.io/crates/chumsky) | Chumsky uses parser combinators. Peggen is a parser generator. | Both produce AST directly. Peggen supports arena allocation. | Peggen avoids deallocating failed sub-rules via a tag-based internal representation. Peggen handles left recursion; Chumsky does not. |
| [LALRPOP](https://lalrpop.github.io/lalrpop) | Peggen is PEG-based; LALRPOP uses LR(1). | Peggen grammars are inline Rust attributes. LR(1) grammars are harder to extend and debug. | LALRPOP generally has better throughput. |

## Performance

Tested on a sample JSON file. CPU: Intel(R) Core(TM) i7-14700HX.

- Peggen: 867913 ns/iter
- Chumsky: 1464737 ns/iter

## Roadmap

- Optimizations: rule dispatch by first symbol; thinner tag representation
- Error handling: custom error handlers
- Documentation: precedence climbing, error handling, arena allocation, left recursion
