# Peggen

A parser generator for parsing expression grammar (PEG) that use inline macros to specify PEG operations. 

## How is it different from (...)?

| /    | Conceptual | User Experience | Performance | Error Handling |
| ---- | ---------- | --------------- | ----------- | -------------- |
| [PEST](https://pest.rs) | PEST only annotates text. <br> Peggen generates AST directly from your text. | In most cases, you still want rust `enum`s for your AST, which is directly provided by **Peggen**, but you have to manually create `enums` from **PEST** rules. | **PEST** use an optimizer to memorize your grammar rules, and use memorization for better performance; **Peggen** doesn't use memorization, arguably this gives better performance over memorization for most grammars. | / |
| [Chumsky](https://crates.io/crates/chumsky) | **Chumsky** provides parser combinators. **Peggen** is a parser generator. | Both **Chumsky** and **Peggen** provides ast directly. However, **Peggen** supports arena allocation.  | **Chumsky** deallocates successful sub-rules when a rule fails; **Peggen** uses a internal representation to eliminate deallocation. | / |
| [LALRPOP](https://lalrpop.github.io/lalrpop) | **Peggen** is PEG-based; **LALRPOP** uses **LR(1)** grammar. | **Peggen** is more intuitive to use than **LALRPOP**; **LR(1)** grammar is hard to extend and debug. | **LALRPOP** has better performance over **Peggen**. | **LR(1)** grammar can report errors far away from normally percepted cause; Peggen allows you to capture errors from customary cause. |

## Performance

I roughly tested the peggen on a sample json file against chumsky. 

CPU Model: Intel(R) Core(TM) i7-14700HX

Suprisingly, Peggen is faster than Chumsky. 

Here are some numbers: 
- Peggen : 867913 ns/iter
- Chumsky: 1464737 ns/iter

## Example: Json Parser

You can write a json parser in the following several lines: 

```rust
#[derive(Debug, ParseImpl, Space, Num, EnumAstImpl)]
pub enum Json {
    #[rule(r"null")]
    Null,
    #[rule(r"{0:`false|true`}")]
    Bool(bool),
    #[rule(r"{0:`-?(0|[1-9][0-9]*)\.([0-9]+)`}")]
    Flt(f32),
    #[rule("{0:`0|-?[1-9][0-9]*`}")]
    Num(i32),
    #[rule(r#""{0:`[^"]*`}""#)]
    Str(String),
    #[rule(r#"\{ [*0: "{0:`[^"]*`}" : {1} , ][?0: "{0:`[^"]*`}" : {1} ] \}"#)]
    Obj(RVec<(String, Json)>),
    #[rule(r"\[ [*0: {0} , ][?0: {0} ] \]")]
    Arr(RVec<Json>)
}
```

## Roadmap

- Optimizations: 
  - Rule dispatch: filter rules by the first symbol, instead of trying each of them. 
  - Thinner tag: currently each tag in internal representation is 3-pointers wide, I want to make them thinner. 
- Error Handling: 
  - Custom error handlers when error handlers fail. 
