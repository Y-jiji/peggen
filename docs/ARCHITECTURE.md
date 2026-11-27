# Architecture

We implement parsing in two stages: 
- **Match** Parse text into Polish prefix notation
- **Assemble** Parse prefix notation into AST

## Match Stage

`Matcher` tracks the current progress of match stage. 

It manages that semantics of the language a **variant** can use. 
- Regex `m.regex(&regex)`
- Literal `m.literal(&str)`
- Repetition `m.repeat(min, max, |m| { sep }, |m| { item })`
- Choice `m.choice<Type>()`
    - With limited variants `m.variants<Type>(&[Type::vix("V1"), Type::vix("V2")])`
    - Insert `tix<<16|vix` as a **tag**
- Message `m.message(span, kind, text)`

`Match` is the a set of rules collected on a type. It calls `Matcher` with the atomics in order.
- `<Type as Match>::variant(m, vix)` will apply one variant with `vix` (`v` for variant, `ix` means index)
- `variant` calls `Matcher` methods internally to parse content.
- We put [error handling](docs/ERROR.md) in a separate chapter.

## Assemble Stage

`Assembler` is finalized from `Matcher`
