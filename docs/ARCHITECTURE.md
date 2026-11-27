# Architecture

We implement parsing in two stages: 
- **Match** Parse text into Polish prefix notation
- **Assemble** Parse prefix notation into AST

`#[peggen]` generates both from the [syntax](#syntax); `peggen::parse` runs both.

## Syntax

Peggen's syntax is macro-flavored. 

* `r"abc"` matching a regex (a raw string literal, follow regex grammar; write `r#"..."#` when the regex contains `"`)
* without `r`, simple `"abc"` matches plain text
* no `!` / `&` predicates; exclude keywords inside the regex (e.g. `r"int\b"`) and by variant order
* whitespace (the `space` regex) is skipped automatically before, between and after the elements of each rule; `_` at one of these spots means no whitespace there
* `var` parse and store as `var`
    * parse by default expression, follow the peggen expression binded to `var`'s type
* `var = (exp)` parse and store as `var`
    * `var` is matching variable
    * `exp` is any peggen expression or a type name
* `var = (exp ; closure)` parse and process with closure
* `var = (exp ; closure)?` parse (or not) and process with closure; the closure gets one `Option` of all values of `exp` (`None` when `exp` does not match)
* `var = (exp ; init ; fold) sep *` parse and process with folding function `+` means at least once, `*` means zero or multiple, sep means separator
* eliding `var =` forwards it to outer closure, for example `(exp)` parse as exp directly
    * inside a group `(exp ; closure)`, only types, fields and regexes pass values to the closure (a regex passes its matched `&str`); plain-text literals and `_` pass nothing
* `exp |args| fields` at the end of a rule: every part passes its value to the closure in order (a type its value, a regex or plain-text literal its matched `&str`, a group its closure's result; `_` passes nothing), then `extra`; the closure returns the variant's fields: the value itself for a single-field variant, a tuple in field order otherwise
    * only a group changes how its parts are passed
    * parts not bound with `var =` go to this closure
    * recursive fields are boxed explicitly, in a group closure (`; Box::new`) or in this closure
* an expression can be a type directly; a bare `Type` tries every variant in declaration order
* `Type@{V1, V2, ...}` parse `Type` using only the rules of variants `V1, V2, ...`, tried in the listed order
* `prefix % rest` if `prefix` matches, `rest` must match, otherwise the input is wrong (an error, not a backtrack); `%` splits the sequence it appears in
* `_` turns off the automatic space skip at that point: no whitespace is allowed there

Attributes: 
* `#[peggen(extra = T)]` passes `&mut T` to every closure as its last argument. 
* `#[peggen(space = r"\s*")]` matching spaces. 
    * without `space`, no whitespace is skipped
* `#[parse(...)]` fills peggen expression: on each variant of an enum, or once on a struct for its single rule. 
* `#[error("..." : ...)]` is the fallback of the `#[parse]` variant before it. 
    * message and pattern. 
    * When an element after `%` fails, a type, a regex or a literal alike, the variant fails, and the `#[error]` variants after it are tried from the variant's start, up to the next `#[parse]` variant (or the end of the enum). 
    * If none matches, it is not recoverable. Raise error. 
    * Inside an `#[error]` pattern every failure is a mismatch, and `Type@{…}` may name `#[error]` variants, which a `#[parse]` pattern may not. 

## Match Stage

`Matcher` tracks the current progress of match stage. 

It manages that semantics of the language a **variant** can use. 
- Regex `m.regex(&regex)`
- Literal `m.literal(&str)`
- Space `m.skip(&regex)`
- Repetition `m.repeat(min, max, |m| { sep }, |m| { item })`
- Choice `m.choice::<Type>()`
    - With limited variants `m.variants::<Type>(&[vix::<Type>("V1"), vix::<Type>("V2")], &GROUP)`
    - Handles [left recursion](LEFTREC.md)
- Message `m.message(span, kind, text)`
    - kind `Error` or `Fatal`

`Match` is the a set of rules collected on a type. It calls `Matcher` with the atomics in order.
- `<Type as Match>::variant(m, vix, first)` will apply one variant with `vix` (`v` for variant, `ix` means index)
- `variant` calls `Matcher` methods internally to parse content.
- We put [error handling](ERROR.md) in a separate chapter.

Code generation: one `Matcher` call per element
- `"abc"` → `m.literal("abc")`, `r"abc"` → `m.regex(&regex)`, space → `m.skip(&space)` unless `_`
- `Type` → `m.choice::<Type>()`, `Type@{V1}` → `m.variants::<Type>(&[vix::<Type>("V1")], &GROUP)`
- `(exp)?` → `m.repeat(0, 1, …)`, `(exp) sep *` → `m.repeat(0, usize::MAX, …)`

## Assemble Stage

`Assembler` is finalized from `Matcher`.
- Variant `assembler.variant()`
- Regex `assembler.text()`
- Repetition `assembler.many()`

`<Type as Assemble>::assemble(assembler, extra)` builds one variant: it reads the parts in input order, then runs the closures.

Code generation: one read per part
- `r"abc"` → `assembler.text()`, `Type` → `<Type as Assemble>::assemble(assembler, extra)`
- `(exp)?` and `(exp) sep *` → `assembler.many()`, then the items
- `"abc"` and `_` → nothing
