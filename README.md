# Peggen

## Syntax

Peggen's syntax is macro-flavored. 

* `r"abc"` matching a regex (inside a Rust string literal, follow regex grammar; use a raw string `r"..."` when the regex contains `\`)
* `_` space regex
* `var` parse and store as `var`
    * parse by default expression, follow the peggen expression binded to `var`'s type
* `var = (exp)` parse and store as `var`
    * `var` is matching variable
    * `exp` is any peggen expression or a type name
* `var = (exp ; closure)` parse and process with closure
* `var = (exp ; closure)?` parse (or not) and process with closure, closure will get `None`
* `var = (exp ; init ; fold) sep *` parse and process with folding function `+` means at least once, `*` means zero or multiple, sep means separator
* eliding `var =` fowards it to outer closure, for example `$(exp)` parse as exp directly
* an expression can be a type directly; a bare `Type` tries every variant in declaration order
* `Type@{V1, V2, ...}` parse `Type` using only the rules of variants `V1, V2, ...`, tried in the listed order

## Algorithm

Static Analysis:
* Capture all 
* 

Runtime: Try each variant's rule by order. If not matching, try next one. 
* For regex / literal str, match regex / literal str and record the span. 
* For space, match space and skip the span. 
* For repeat, match content and separator, record each span. Add an repeat tag on top. 
* If matching rule and progress are both the same, but depth goes deeper, it is left recursion. Therefore, fail current rule immediately and try the next one. 

## Example

A simplified json parser:

```rust
#[peggen(space = r"\s*")]
pub enum Json {
    #[parse(_0 = ("true|false" ; |x: &str| x == "true"))]
    Bool(bool),
    #[parse(r"\{" _0 = (
        "\"[a-z]*\"" ":" Json // kv pair
        ; HashMap::new()     // init
        ; |map, key, _colon, value, _comma| { map.insert(key, value); }
    ) "," * r"\}")]
    Map(HashMap<String, Json>),
    #[parse(r"\[" $list = (Json ; Vec::new() ; |list, val| { list.push(val); }) "," * r"\]")]
    List { list : Vec<Json> },
    #[parse("\"" _ _0 = ("[a-z]*"; |x: &str| x.to_string()) _ "\"")]
    String(String),
    #[parse(_0 = ("[1-9]+"; |x: &str| x.to_string()))]
    Number(String)
}
```

A calculator: 

```rust
#[peggen(space = r"\s*", extra = Bump)]
pub enum CalcEx {
    #[parse(_0 = (CalcEx@{Add, Sub, Mul, Div, Paren, Num}) "+" _1 = (CalcEx@{Mul, Div, Paren, Num})))]
    Add(CalcEx, CalcEx),
    #[parse(_0 = (CalcEx@{Add, Sub, Mul, Div, Paren, Num}) "-" _1 = (CalcEx@{Mul, Div, Paren, Num}))]
    Sub(CalcEx, CalcEx),
    #[parse(_0 = (CalcEx@{Mul, Div, Paren, Num}) r"\*" _1 = (CalcEx@{Mul, Div}))]
    Mul(CalcEx, CalcEx),
    #[parse(_0 = (CalcEx@{Mul, Div, Paren, Num}) "/" _1 = (CalcEx@{Mul, Div}))]
    Div(CalcEx, CalcEx),
    #[parse(r"\(" _0 = CalcEx r"\)")]
    Paren(CalcEx),
    #[parse(_0 = ("[1-9]+"; |x: &str| FromStr::parse(x).unwrap()))]
    Num(usize)
}
```
