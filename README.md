# Peggen

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

## Algorithm

Runtime (matching): Try each variant's rule by order. If not matching, try next one. 
* For regex / literal str, match regex / literal str and record the span. 
* For space, match space and skip the span. 
* For repeat, match content and separator, record each span. Add an repeat tag on top. 
* A group is a type with an ordered list of its variants: `Type@{V1, V2, ...}`, or a bare `Type` for its `#[parse]` variants in declaration order. A call of a group that starts at the same position as a running call of the same group is left recursion: the inner call fails immediately, and its variant tries the next one.
* After its first match, such a call grows: it retries the variants that start with its own type, with the match so far as their first element, and keeps the longer match, until none extends it. A failure after `%` while growing tries the `#[error]` variants after that variant, from the call's start. 
* The whole input must be consumed: after the root type matches, the parser checks end of input.

Runtime (errors): 
* only explicit failures are reported, each as a message with the span of input it covers, a kind and a text:
    * an `#[error]` variant that matches records its message over the text it matched, kind `Error`, and parsing goes on
    * a failure after `%` that no `#[error]` variant recovers ends the parse with `no match after %, and no #[error] variant matched`, kind `Fatal`, over the failed variant's text up to where it failed
* with no explicit failure, a parse that cannot reach the end of input reports the input left: `parsing stops here`, kind `Fatal`

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
