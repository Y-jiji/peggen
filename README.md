# Peggen

A parser generator for parsing expression grammar (PEG) that use inline macros to specify PEG operations. 

```rust
use peggen::*;

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
}

fn main() {
    let json = Parser::<Json>::parse("{x: 1, y: 2}").unwrap();
    println!("{json:?}");
}
```

## Roadmap

Help needed! There is so much to do! [Contact Me](mailto:y.jijiji.data.science@gmail.com)

- Optimizations: 
  - Rule dispatch: filter rules by the first symbol, instead of trying each of them. 
  - Thinner tag: currently each tag in internal representation is 3-pointers wide, I want to make them thinner. 
- Error Handling: 
  - Custom final error handlers when custom error capturing fails. 
- Documentation: 
  - Demonstrate features like precedence climbing, error handling, repetition, custom `FromStr`, arean allocation, and left recursion handling. 

## How is it different from (...)?

| /    | Conceptual | User Experience | Performance & Code Quality | Error Handling |
| ---- | ---------- | --------------- | -------------------------- | -------------- |
| [PEST](https://pest.rs) | PEST only annotates text. <br> Peggen generates AST directly from your text. | In most cases, you still want rust `enum`s for your AST, which is directly provided by **Peggen**, but you have to manually create `enums` from **PEST** rules. | **PEST** use an optimizer to memorize your grammar rules, and use memorization for better performance; **Peggen** doesn't use memorization, arguably this gives better performance over memorization for most grammars. | / |
| [Chumsky](https://crates.io/crates/chumsky) | **Chumsky** provides parser combinators. **Peggen** is a parser generator. | Both **Chumsky** and **Peggen** provides ast directly. However, **Peggen** supports arena allocation.  | **Chumsky** deallocates successful sub-rules when a rule fails; **Peggen** uses a internal representation to eliminate deallocation. Besides, **Peggen** handles left recursion, while in **Chumsky** left recursion causes in stack overflow. | / |
| [LALRPOP](https://lalrpop.github.io/lalrpop) | **Peggen** is PEG-based; **LALRPOP** uses **LR(1)** grammar. | **Peggen** is more intuitive to use than **LALRPOP**; **LR(1)** grammar is hard to extend and debug. | **LALRPOP** has better performance over **Peggen**. | **LR(1)** grammar can report errors far away from normally percepted cause; Peggen allows you to capture errors from customary cause. |

## Performance

I roughly tested the peggen on a sample json file against chumsky. 

CPU Model: Intel(R) Core(TM) i7-14700HX

Suprisingly, Peggen is faster than Chumsky. 

Here are some numbers: 
- Peggen : 867913 ns/iter
- Chumsky: 1464737 ns/iter

## Example: Json Parser Step By Step

### Final Result

Before we start this tutorial, let's look at how it looks like after your first try. 

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

### Use Your Parser

We have a `Parser` type that help you use annotated `enum`. 

```rust
use peggen::*;

fn main() {
    let json = Parser::<Json>::parse("{x: 1, y: 2}").unwrap();
    println!("{json:?}");
}
```

### Step 1: Formatting String

A `rule` attribute looks like the following: 
```rust
#[rule("...")]
```

The string ensembles rust formatting string that you use in `println!()`. Rust formatting string represents a sequence of chars/structures printed one after another. Peggen formatting string represents a sequence of chars/parsers that eat the input string one after another. 

For example, the following statement prints: 
- A boolean `false` as the first argument
- A token ` and `
- An integer `19` as the second argument

```rust
println!("{0} and {1}", false, 19);
```

You can write a parser that parses `<bool> and <int>` using a `rule` attribute. However, Peggen needs to know what kind of string can be recognized as `bool` and what kind of string can be recognized as `i64`. For this purpose, we can write [regular expressions](https://en.wikipedia.org/wiki/Regular_expression). 

```rust
#[derive(Debug, ParseImpl, Space, Num, EnumAstImpl)]
#[rule("{0:`false|true`} and {1:`[0-9]+`}")]
struct BoolAndInt(bool, i64);
```

<details open>
<summary>Question</summary>

What will the following statement print?
```rust

println!("{:?}", Parser::<BoolAndInt>::parse("false and 19").unwrap());
```
</details>

<details>
<summary>Answer</summary>

```
BoolAndInt(false, 19);
```
</details>

An `enum` is a collection of rules, during parsing, the rules declared in an `enum` is tried one by one until one of them matches. 

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
}
```

<details open>
<summary>Question</summary>

* How to parse a string with `"` escaped to `\"`?
* For example: `"\"a string\""`. 
</details>

<details>
<summary>Answer</summary>

```rust
#[rule(r#""{0:`([^"]|\\")*`}""#)]
Str(String)
```
</details>

<details open>
<summary>Question</summary>

Given that you can have multiple rules on the same enum variant, what is the alternative way of writing the `Bool(bool)` operation?
</details>

<details>
<summary>Answer</summary>

```rust
#[rule(r#""{0:`false`}"#)]
#[rule(r#""{0:`true`}"#)]
Bool(bool)
```
</details>

### Step 2: Repetition

TODO
