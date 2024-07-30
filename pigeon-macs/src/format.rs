use once_cell::sync::Lazy;
use quote::ToTokens;
use regex::Regex;

use crate::*;
use std::collections::HashMap;
use std::result::Result;

pub(crate) enum Fmt {
    Symbol {
        arg: String,
        typ: Type,
        group: usize,
    },
    Regex {
        arg: String,
        typ: Type,
        regex: String,
    },
    Token {
        token: String,
    },
    Space,
}

impl std::fmt::Debug for Fmt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Fmt::Symbol { arg, typ, group } => {
                write!(f, "Symbol {{ arg: {arg:?}, typ: {}, group: {group:?} }}", typ.to_token_stream().to_string())
            },
            Fmt::Regex { arg, typ, regex } => {
                write!(f, "Regex {{ arg: {arg:?}, typ: {}, regex: {regex:?} }}", typ.to_token_stream().to_string())
            },
            Fmt::Token { token } => {
                write!(f, "Token {{ token: {token:?} }}")
            },
            Fmt::Space => {
                write!(f, "Space")
            }
        }
    }
}

pub struct FmtParser {
    pub map: HashMap<String, Type>,
}

#[allow(unused)]
#[derive(Debug)]
pub enum FmtError<'a> {
    Unmatched {
        token: &'a str,
        found: &'a str,
    },
    TooMany {
        token: &'a str,
    },
    Depleted,
    NoSymbol {
        symbol: &'a str,
    },
    BadNumber {
        found: &'a str, 
    },
    NotToken
}

impl FmtParser {
    pub fn new(fields: Fields) -> crate::Result<Self> {
        let mut map = HashMap::new();
        match fields.clone() {
            Fields::Named(FieldsNamed { named, .. }) => {
                for field in named {
                    map.insert(field.ident.to_token_stream().to_string(), field.ty);
                }
            }
            Fields::Unnamed(FieldsUnnamed { unnamed, .. }) => {
                for (i, field) in unnamed.into_iter().enumerate() {
                    map.insert(format!("{i}"), field.ty);
                }
            }
            Fields::Unit => {}
        };
        Ok(Self { map })
    }
    pub fn many<'a>(&self, input: &'a str, mut end: usize) -> Result<(usize, Vec<Fmt>), FmtError<'a>> {
        let mut seq = vec![];
        loop {
            if let Ok((end_, expr)) = self.space(input, end) {
                seq.push(expr);
                end = end_; continue;
            };
            if let Ok((end_, expr)) = self.token(input, end) {
                seq.push(expr); 
                end = end_; continue;
            };
            if let Ok((end_, expr)) = self.symbol(input, end) {
                seq.push(expr); 
                end = end_; continue;
            };
            if let Ok((end_, expr)) = self.regexp(input, end) {
                seq.push(expr); 
                end = end_; continue;
            };
            break
        }
        Ok((end, seq))
    }
    fn eat<'a>(tok: &'a str, input: &'a str, end: usize) -> Result<usize, FmtError<'a>> {
        if !input[end..].starts_with(tok) {
            Err(FmtError::Unmatched { token: tok, found: &input[end..(end+tok.len()).min(input.len())] })
        } else {
            Ok(end + tok.len())
        }
    }
    fn count<'a>(tok: &'a str, input: &'a str, end: usize) -> (usize, usize) {
        let mut end = end;
        let mut count = 0;
        while let Ok(end_) = Self::eat(tok, input, end) {
            count += 1;
            end = end_;
        }
        (end, count)
    }
    fn ident<'a>(&self, input: &'a str, end: usize) -> Result<(usize, String, Type), FmtError<'a>>{
        for (delta, ch) in input[end..].char_indices() {
            if !ch.is_ascii_alphanumeric() {
                return Ok((end+delta, input[end..end+delta].to_string(), self.map.get(&input[end..end+delta]).cloned().ok_or_else(|| FmtError::NoSymbol { symbol: &input[end..end+delta] })?))
            }
        }
        Ok((input.len(), input[end..].to_string(), self.map.get(&input[end..]).cloned().ok_or_else(|| FmtError::NoSymbol { symbol: &input[end..] })?))
    }
    fn num<'a>(input: &'a str, end: usize) -> Result<(usize, usize), FmtError<'a>> {
        for (delta, ch) in input[end..].char_indices() {
            if !ch.is_ascii_digit() {
                return Ok((end+delta, input[end..end+delta].parse::<usize>().map_err(|_| FmtError::BadNumber { found: &input[end..end+delta] })?))
            }
        }
        Ok((input.len(), input[end..].parse::<usize>().map_err(|_| FmtError::BadNumber { found: &input[end..] })?))
    }
    fn space<'a>(&self, input: &'a str, end: usize) -> Result<(usize, Fmt), FmtError<'a>> {
        static REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^ +").unwrap());
        let Some(delta) = REGEX.find(&input[end..]) else { Err(FmtError::NotToken)? };
        Ok((end+delta.len(), Fmt::Space))
    }
    fn token<'a>(&self, input: &'a str, end: usize) -> Result<(usize, Fmt), FmtError<'a>> {
        static REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^(\\[\(\)\{\}tn\\ ]|[^\(\)\{\} ])+").unwrap());
        let Some(delta) = REGEX.find(&input[end..]) else { Err(FmtError::NotToken)? };
        fn unescape(input: &str) -> String {
            let mut output = String::new();
            let mut chars = input.chars();
            while let Some(ch) = chars.next() {
                if ch != '\\' {
                    output.push(ch);
                }
                else {
                    let Some(ch) = chars.next() else {
                        continue;
                    };
                    if matches!(ch, '['|']'|'{'|'}'|'('|')'|' '|'\\') {
                        output.push(ch);
                    } else if matches!(ch, 't') {
                        output.push('\t');
                    } else if matches!(ch, 'n') {
                        output.push('\n');
                    }
                }
            }
            output
        }
        let token = unescape(&input[end..end+delta.len()]);
        Ok((end+delta.len(), Fmt::Token { token }))
    }
    fn symbol<'a>(&self, input: &'a str, end: usize) -> Result<(usize, Fmt), FmtError<'a>> {
        let end = Self::eat("{", input, end)?;
        let (end, arg, typ) = self.ident(input, end)?;
        if let Ok(end) = Self::eat(":", input, end) {
            let (end, group) = Self::num(input, end)?;
            let end = Self::eat("}", input, end)?;
            Ok((end, Fmt::Symbol { arg, typ, group }))
        } else {
            let end = Self::eat("}", input, end)?;
            Ok((end, Fmt::Symbol { arg, typ, group: 0 }))
        }
    }
    // match the number of "#" before "`" and the number of "#" after "`"
    fn regexp<'a>(&self, input: &'a str, end: usize) -> Result<(usize, Fmt), FmtError<'a>> {
        let end = Self::eat("{", input, end)?;
        let (end, arg, typ) = self.ident(input, end)?;
        let (end, cnt) = Self::count("#", input, end);
        let mut end = Self::eat("`", input, end)?;
        let mut range = end..end;
        loop {
            if &input[range.end..] == "" {
                Err(FmtError::Depleted)?
            }
            if !input[range.end..].starts_with("`") {
                range.end = range.end + input[range.end..].chars().next().unwrap().len_utf8();
                continue;
            }
            let (end_, cnt_) = Self::count("#", input, range.end + 1);
            if cnt_ < cnt { 
                range.end = range.end + 1;
                continue;
            }
            if cnt_ > cnt {
                Err(FmtError::TooMany { token: "#" })?
            }
            end = end_;
            break;
        }
        let end = Self::eat("}", input, end)?;
        Ok((end, Fmt::Regex {
            typ, arg, 
            regex: input[range].to_string() 
        }))
    }
}

#[cfg(test)]
mod test {
    use punctuated::Punctuated;

    use super::*;

    #[test]
    fn regex() {
        let parser = FmtParser { map: HashMap::from_iter([
            (String::from("0"), Type::Path(TypePath { qself: None, path: Path { leading_colon: None, segments: Punctuated::default() } }))
        ]) };
        println!("{:?}", parser.regexp("{0`asdfasdf`}", 0));
    }

    #[test]
    fn symbol() {
        let parser = FmtParser { map: HashMap::from_iter([
            (String::from("0"), Type::Path(TypePath { qself: None, path: Path { leading_colon: None, segments: Punctuated::default() } }))
        ]) };
        println!("{:?}", parser.symbol("{0:10}", 0));
    }

    #[test]
    fn many() {
        let parser = FmtParser { map: HashMap::from_iter([
            (String::from("0"), Type::Path(TypePath { qself: None, path: Path { leading_colon: None, segments: Punctuated::default() } })),
            (String::from("1"), Type::Path(TypePath { qself: None, path: Path { leading_colon: None, segments: Punctuated::default() } }))
        ]) };
        println!("{:?}", parser.many(r#"{0:10} + {1#`asd"fas`dff`#}"#, 0));
    }
}