use peggen::{peggen, Message};

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

/// Parses a JSON text. On any failure, returns every message: the bad arrays and objects that were skipped,
/// and the one that stopped parsing, if any.
pub fn parse(input: &str) -> Result<Json, Vec<Message<'_>>> {
    match peggen::parse::<Json, ()>(input, &mut ()) {
        Ok((json, messages)) if messages.is_empty() => Ok(json),
        Ok((_, messages)) | Err(messages) => Err(messages),
    }
}

/// Decodes the escapes in the body of a JSON string; a lone surrogate becomes U+FFFD.
/// Caller obligation: `body` matched the string regex of `Text`, so every escape is complete.
fn unescape(body: &str) -> String {
    let mut text = String::with_capacity(body.len());
    let mut rest = body;
    while let Some(at) = rest.find('\\') {
        text.push_str(&rest[..at]);
        let (code, len) = match rest.as_bytes()[at + 1] {
            b'b' => (0x08, 2),
            b'f' => (0x0C, 2),
            b'n' => (0x0A, 2),
            b'r' => (0x0D, 2),
            b't' => (0x09, 2),
            b'u' => unicode(&rest[at..]),
            other => (other as u32, 2),
        };
        text.push(char::from_u32(code).unwrap_or('\u{FFFD}'));
        rest = &rest[at + len..];
    }
    text.push_str(rest);
    text
}

/// The code point of the `\uXXXX` escape at the front of `escape`, joining a surrogate pair, and its byte length.
fn unicode(escape: &str) -> (u32, usize) {
    let hex = |at: usize| u32::from_str_radix(&escape[at..at + 4], 16).unwrap();
    let high = hex(2);
    if (0xD800..0xDC00).contains(&high) && escape[6..].starts_with("\\u") {
        let low = hex(8);
        if (0xDC00..0xE000).contains(&low) {
            return (0x10000 + ((high - 0xD800) << 10) + (low - 0xDC00), 12);
        }
    }
    (high, 6)
}

#[cfg(test)]
mod correct {
    use super::*;
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};

    /// The 17 inputs RFC 8259 accepts parse to the expected values; the 27 it rejects, including `""`, all fail.
    #[test]
    fn standard() {
        let slash = '\\';
        let text = |s: &str| Json::String(s.to_string());
        let accepted: Vec<(String, Json)> = vec![
            ("null".into(), Json::Null),
            (" true ".into(), Json::Bool(true)),
            ("false".into(), Json::Bool(false)),
            ("0".into(), Json::Number(0.0)),
            ("-0".into(), Json::Number(-0.0)),
            ("12.5e-3".into(), Json::Number(0.0125)),
            ("1E+2".into(), Json::Number(100.0)),
            ("-1.0e10".into(), Json::Number(-1e10)),
            (r#""a\"b\\c\/d\b\f\n\r\t""#.into(), text("a\"b\\c/d\u{8}\u{c}\n\r\t")),
            (format!("\"{slash}u00e9{slash}uD83D{slash}uDE00{slash}u00E9\""), text("é😀é")),
            (format!("\"{slash}uD800\""), text("\u{FFFD}")),
            (r#""""#.into(), text("")),
            ("[]".into(), Json::Array(vec![])),
            ("[ 1 , [2] , {} ]".into(), Json::Array(vec![
                Json::Number(1.0), Json::Array(vec![Json::Number(2.0)]), Json::Object(vec![]),
            ])),
            (r#"{"a": {"b": [true, null]}, "": 0}"#.into(), Json::Object(vec![
                ("a".into(), Json::Object(vec![("b".into(), Json::Array(vec![Json::Bool(true), Json::Null]))])),
                ("".into(), Json::Number(0.0)),
            ])),
            (" \n {\"x\":1} \t".into(), Json::Object(vec![("x".into(), Json::Number(1.0))])),
            ("{}".into(), Json::Object(vec![])),
        ];
        for (input, value) in &accepted {
            assert_eq!(parse(input).ok().as_ref(), Some(value), "{input:?}");
        }
        let rejected = [
            "01", "1.", ".5", "+1", "-", "1e", "[1,]", "[,1]", r#"{"a":1,}"#, r#"{"a"}"#, "{a:1}", "'a'",
            r#""\x""#, r#""\u12""#, "\"tab\there\"", "tru", "nul", "[1 2]", "  ", "", "1 2", r#""abc"#,
            r#"{"a":1}}"#, "[1]]", "\u{a0}1", "NaN", "Infinity",
        ];
        for input in rejected {
            assert!(parse(input).is_err(), "{input:?}");
        }
    }

    /// `[[1,], {"a" 1}, 2]` recovers to `Array([BadArray, BadObject, Number(2.0)])`, with messages at 1:2 and 1:8.
    #[test]
    fn recovery() {
        let input = r#"[[1,], {"a" 1}, 2]"#;
        let (value, messages) = peggen::parse::<Json, ()>(input, &mut ()).expect("recovers");
        assert_eq!(value, Json::Array(vec![Json::BadArray, Json::BadObject, Json::Number(2.0)]));
        let found: Vec<_> = messages.iter().map(|message| (message.locate(input), message.text)).collect();
        assert_eq!(found, [((1, 2), "bad array, skipped to its `]`"), ((1, 8), "bad object, skipped to its `}`")]);
    }

    /// 2000 pseudo-random values, printed with mixed whitespace and escapes, parse back equal.
    #[test]
    fn roundtrip() {
        let mut rng = StdRng::seed_from_u64(0x9E37_79B9_7F4A_7C15);
        for _ in 0..2000 {
            let value = random(&mut rng, 4);
            let mut text = String::new();
            print(&value, &mut text);
            assert_eq!(parse(&text).ok(), Some(value), "{text:?}");
        }
    }

    /// A pseudo-random value, nested at most `depth` deep.
    fn random(rng: &mut StdRng, depth: u32) -> Json {
        match rng.random_range(0..if depth == 0 { 4 } else { 6 }) {
            0 => Json::Null,
            1 => Json::Bool(rng.random_bool(0.5)),
            2 => {
                let number = (rng.random::<i64>() as f64) / 2f64.powi(rng.random_range(0..80));
                Json::Number(if rng.random_range(0..5) == 0 { number.trunc() } else { number })
            }
            3 => {
                let pool = ['a', '"', '\\', '\n', '\u{1}', 'é', '😀', '/', '\t', ' '];
                Json::String((0..rng.random_range(0..6)).map(|_| pool[rng.random_range(0..pool.len())]).collect())
            }
            4 => Json::Array((0..rng.random_range(0..4)).map(|_| random(rng, depth - 1)).collect()),
            _ => Json::Object((0..rng.random_range(0..4)).map(|_| {
                let name = (0..rng.random_range(0..3)).map(|_| ['k', '"', 'ü'][rng.random_range(0..3)]).collect();
                (name, random(rng, depth - 1))
            }).collect()),
        }
    }

    /// Writes `value` as JSON text with mixed whitespace.
    fn print(value: &Json, out: &mut String) {
        match value {
            Json::Null => out.push_str(" null"),
            Json::Bool(bool) => out.push_str(if *bool { "true\n" } else { "false" }),
            Json::Number(number) => out.push_str(&format!("{number}")),
            Json::String(text) => quote(text, out),
            Json::Array(items) => {
                out.push_str("[ ");
                for (place, item) in items.iter().enumerate() {
                    if place > 0 {
                        out.push_str(" ,\n");
                    }
                    print(item, out);
                }
                out.push_str(" ]");
            }
            Json::Object(members) => {
                out.push_str("\t{\t");
                for (place, (name, item)) in members.iter().enumerate() {
                    if place > 0 {
                        out.push(',');
                    }
                    quote(name, out);
                    out.push_str(" : ");
                    print(item, out);
                }
                out.push_str("\r\n}");
            }
            Json::BadArray | Json::BadObject => unreachable!("`random` never builds error variants"),
        }
    }

    /// Writes `text` as a JSON string, escaping quotes, backslashes, control characters and astral characters.
    fn quote(text: &str, out: &mut String) {
        out.push('"');
        for ch in text.chars() {
            match ch {
                '"' => out.push_str("\\\""),
                '\\' => out.push_str("\\\\"),
                '\n' => out.push_str("\\n"),
                ch if (ch as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", ch as u32)),
                ch if (ch as u32) > 0xFFFF => {
                    let code = ch as u32 - 0x10000;
                    out.push_str(&format!("\\u{:04X}\\u{:04x}", 0xD800 + (code >> 10), 0xDC00 + (code & 0x3FF)));
                }
                ch => out.push(ch),
            }
        }
        out.push('"');
    }
}
