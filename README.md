# Peggen

## Syntax

Peggen's syntax is macro-flavored. 

* `r"abc"` matching a regex (a raw string literal, follow regex grammar; write `r#"..."#` when the regex contains `"`)
* without `r`, simple `"abc"` matches plain text
* whitespace (the `space` regex) is skipped automatically between adjacent elements; `_` between two elements means no whitespace there
* `var` parse and store as `var`
    * parse by default expression, follow the peggen expression binded to `var`'s type
* `var = (exp)` parse and store as `var`
    * `var` is matching variable
    * `exp` is any peggen expression or a type name
* `var = (exp ; closure)` parse and process with closure
* `var = (exp ; closure)?` parse (or not) and process with closure, closure will get `None`
* `var = (exp ; init ; fold) sep *` parse and process with folding function `+` means at least once, `*` means zero or multiple, sep means separator
* eliding `var =` forwards it to outer closure, for example `(exp)` parse as exp directly
    * inside a group `(exp ; closure)`, only types, fields and regexes pass values to the closure (a regex passes its matched `&str`); plain-text literals and `_` pass nothing
* `exp |args| fields` at the end of a rule: every part passes its value to the closure in order (a type its value, a regex or plain-text literal its matched `&str`, a group its closure's result; `_` passes nothing), then `extra`; the closure returns the variant's fields: the value itself for a single-field variant, a tuple in field order otherwise
    * only a group changes how its parts are passed
* an expression can be a type directly; a bare `Type` tries every variant in declaration order
* `Type@{V1, V2, ...}` parse `Type` using only the rules of variants `V1, V2, ...`, tried in the listed order
* `prefix % rest` if `prefix` matches, `rest` must match, otherwise the input is wrong (an error, not a backtrack); `%` splits the sequence it appears in
* `_` skips space

Attributes: 
* `#[peggen(extra = T)]` passes a value of type `T` to every closure as its last argument. 
* `#[peggen(space = r"\s*")]` matching spaces. 
* `#[parse(...)]` fills peggen expression. 
* `#[error("..." : ...)]` is the fallback that `%` prefix matches. 
    * message and pattern. 
    * For a type after `%`, go through error branches of that type. 
    * For a regex/literal after `%`, go through error branches after that variant. 
    * If both fails, finally fall to next variant. 
    * If it still fails, it is not recoverable. Raise error. 

## Algorithm

Runtime (matching): Try each variant's rule by order. If not matching, try next one. 
* For regex / literal str, match regex / literal str and record the span. 
* For space, match space and skip the span. 
* For repeat, match content and separator, record each span. Add an repeat tag on top. 
* If matching rule and progress are both the same, but depth goes deeper, it is left recursion. Therefore, fail current rule immediately and try the next one. 

## Example

A simplified json parser:

```rust
#[peggen(space = r"\s*")]
pub enum Json {
    #[parse(_0 = (r"true|false" ; |x: &str| x == "true"))]
    Bool(bool),
    #[parse("{" % _0 = (
        "\"" _ r"[a-z]*" _ "\"" % ":" Json                                // kv pair
        ; HashMap::new()                                                  // init
        ; |map, key: &str, value| { map.insert(key.to_string(), value); } // fold
    ) "," * "}")]
    Map(HashMap<String, Json>),
    #[parse("[" % list = (Json ; Vec::new() ; |list, val| { list.push(val); }) "," * "]")]
    List { list: Vec<Json> },
    #[parse("\"" % _ _0 = (r"[a-z]*" ; |x: &str| x.to_string()) _ "\"")]
    String(String),
    #[parse(_0 = (r"[0-9]+" ; |x: &str| x.to_string()))]
    Number(String),
}
```

A calculator: 

```rust
#[peggen(space = r"\s*")]
pub enum CalcEx {
    #[parse(
        _0 = (CalcEx@{Add, Sub, Mul, Div, Paren, Num} ; Box::new) "+" % 
        _1 = (CalcEx@{Mul, Div, Paren, Num} ; Box::new)
    )]
    Add(Box<CalcEx>, Box<CalcEx>),
    #[parse(
        _0 = (CalcEx@{Add, Sub, Mul, Div, Paren, Num} ; Box::new) "-" % 
        _1 = (CalcEx@{Mul, Div, Paren, Num} ; Box::new)
    )]
    Sub(Box<CalcEx>, Box<CalcEx>),
    #[parse(
        _0 = (CalcEx@{Mul, Div, Paren, Num} ; Box::new) "*" % 
        _1 = (CalcEx@{Paren, Num} ; Box::new)
    )]
    Mul(Box<CalcEx>, Box<CalcEx>),
    #[parse(
        _0 = (CalcEx@{Mul, Div, Paren, Num} ; Box::new) "/" % 
        _1 = (CalcEx@{Paren, Num} ; Box::new)
    )]
    Div(Box<CalcEx>, Box<CalcEx>),
    #[parse("(" % _0 = (CalcEx ; Box::new) ")")]
    Paren(Box<CalcEx>),
    #[parse(_0 = (r"[0-9]+" ; |x: &str| x.parse::<usize>().unwrap()))]
    Num(usize),
}
```

A calculator with arena allocation (each rule ends with a closure that receives every part, then `extra`, and returns the fields):

```rust
#[peggen(space = r"\s*", extra = &'a Bump)]
pub enum ArenaEx<'a> {
    #[parse(
        ArenaEx@{Add, Mul, Paren, Num} "+" % ArenaEx@{Mul, Paren, Num}
        |x, _plus, y, bump| (bump.alloc(x), bump.alloc(y))
    )]
    Add(&'a ArenaEx<'a>, &'a ArenaEx<'a>),
    #[parse(
        ArenaEx@{Mul, Paren, Num} "*" % ArenaEx@{Paren, Num}
        |x, _star, y, bump| (bump.alloc(x), bump.alloc(y))
    )]
    Mul(&'a ArenaEx<'a>, &'a ArenaEx<'a>),
    #[parse(
        "(" % ArenaEx ")"
        |_lpar, x, _rpar, bump| bump.alloc(x)
    )]
    Paren(&'a ArenaEx<'a>),
    #[parse(
        r"[0-9]+"
        |x: &str, _bump| x.parse::<usize>().unwrap()
    )]
    Num(usize),
}
```
