# Peggen

A PEG parser generator for Rust, with left recursion and error recovery.

- [Architecture](docs/ARCHITECTURE.md): syntax, stages, code generation
- [Error Handling](docs/ERROR.md)
- [Left Recursion](docs/LEFTREC.md)

## Examples

JSON (RFC 8259), from `peggen-examples/src/json.rs` (`unescape` decodes the escapes; see that file):

```rust
/// A JSON text (RFC 8259): one value, with whitespace skipped around it and between its parts.
#[peggen(space = r"[ \t\n\r]*")]
#[derive(Debug, Clone, PartialEq)]
pub enum Json {
    #[parse("null")]
    Null,
    #[parse(_0 = (r"true|false" ; |x: &str| x == "true"))]
    Bool(bool),
    #[parse(_0 = (r"-?(?:0|[1-9][0-9]*)(?:\.[0-9]+)?(?:[eE][+-]?[0-9]+)?" ; |x: &str| x.parse::<f64>().unwrap()))]
    Number(f64),
    #[parse(_0 = (Text ; |Text(text)| text))]
    String(String),
    #[parse("[" % _0 = (Json ; Vec::new() ; |list, value| list.push(value)) "," * "]")]
    Array(Vec<Json>),
    #[error("bad array, skipped to its `]`" : "[" r"[^\]]*" "]")]
    BadArray,
    #[parse("{" % _0 = (
        Text ":" Json
        ; Vec::new()
        ; |members, Text(name), value| members.push((name, value))
    ) "," * "}")]
    Object(Vec<(String, Json)>),
    #[error("bad object, skipped to its `}`" : "{" r"[^}]*" "}")]
    BadObject,
}

/// A JSON string with its escapes decoded (RFC 8259, section 7).
#[peggen]
#[parse("\"" % _0 = (r#"(?:[^"\\\x00-\x1F]|\\["\\/bfnrt]|\\u[0-9a-fA-F]{4})*"# ; unescape) "\"")]
struct Text(String);
```

A calculator with precedence, left recursion and recovery from an unclosed parenthesis, from `peggen-examples/src/calc.rs`:

```rust
/// The README calculator: `+` and `-` bind looser than `*` and `/`; all four associate to the left.
#[peggen(space = r"\s*")]
#[derive(Debug, Clone, PartialEq)]
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
    #[error("unclosed parenthesis" : "(" r"[^)]*")]
    Unclosed,
    #[parse(_0 = (r"[0-9]+" ; |x: &str| x.parse::<usize>().unwrap()))]
    Num(usize),
}
```
