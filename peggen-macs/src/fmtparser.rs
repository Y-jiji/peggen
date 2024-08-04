use quote::ToTokens;
use regex::Regex;

use crate::*;
use std::collections::HashMap;
use std::result::Result;
use std::sync::LazyLock;

#[non_exhaustive]
pub(crate) enum Fmt {
    /// A symbol refers to a non-terminal symbol
    Symbol {
        arg: String,
        typ: Type,
        group: usize,
    },
    /// A regular expression parses
    RegExp {
        arg: String,
        typ: Type,
        regex: String,
    },
    /// Consecutive expressions that push to one argument is grouped together. 
    /// 
    /// For one argument, only one segment is allowed. 
    /// 
    /// For example, `[0:...] {1} [0:...]` is invalid, because two segements are seperated by a `{1}`. 
    /// 
    /// tag.rule works as a counter. 
    SeqExp {
        arg: String,
        typ: Type,
        children: Vec<(Vec<Fmt>, Flag)>,
    },
    /// A token works like simple literals. 
    Token {
        token: String,
    },
    /// A consecutive spaces. 
    Space,
}

impl std::fmt::Debug for Fmt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {match self {
        Fmt::Symbol { arg, typ, group } => {
            write!(f, "Symbol {{ arg: {arg:?}, typ: {}, group: {group:?} }}", typ.to_token_stream())
        }
        Fmt::RegExp { arg, typ, regex } => {
            write!(f, "Regex {{ arg: {arg:?}, typ: {}, regex: {regex:?} }}", typ.to_token_stream())
        }
        Fmt::SeqExp { arg, typ, children: child} => {
            write!(f, "SeqExp {{ arg: {arg:?}, typ: {}, child: {child:?} }}", typ.to_token_stream())
        }
        Fmt::Token { token } => {
            write!(f, "Token {{ token: {token:?} }}")
        }
        Fmt::Space => {
            write!(f, "Space")
        }
    }}
}

#[derive(Debug)]
pub(crate) enum Flag {
    /// Nothing special
    Null,
    /// The pattern can be omitted
    OrNot,
    /// The pattern can be repeated
    Repeat,
}

/// A parser that parses peggen fmt string. 
pub struct FmtParser {
    /// A map from name to types
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
    NotToken,
    NotGenerics,
}

impl FmtParser {
    /// Create a new parser with given symbol table
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
    /// Eat many expressions
    pub fn many<'a>(&self, input: &'a str, mut end: usize) -> Result<(usize, Vec<Fmt>), FmtError<'a>> {
        let mut seq = vec![];
        loop {
            if let Ok((end_, expr)) = self.seqexp(input, end) {
                seq.push(expr);
                end = end_; continue;
            };
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
    /// Eat a token
    fn eat<'a>(tok: &'a str, input: &'a str, end: usize) -> Result<usize, FmtError<'a>> {
        if !input[end..].starts_with(tok) {
            Err(FmtError::Unmatched { token: tok, found: &input[end..(end+tok.len()).min(input.len())] })
        } else {
            Ok(end + tok.len())
        }
    }
    /// Count the appearences of a pattern
    fn count<'a>(tok: &'a str, input: &'a str, end: usize) -> (usize, usize) {
        let mut end = end;
        let mut count = 0;
        while let Ok(end_) = Self::eat(tok, input, end) {
            count += 1;
            end = end_;
        }
        (end, count)
    }
    /// Eat an identity
    fn ident<'a>(&self, input: &'a str, end: usize) -> Result<(usize, String, Type), FmtError<'a>>{
        for (delta, ch) in input[end..].char_indices() {
            if !ch.is_ascii_alphanumeric() {
                return Ok((end+delta, input[end..end+delta].to_string(), self.map.get(&input[end..end+delta]).cloned().ok_or_else(|| FmtError::NoSymbol { symbol: &input[end..end+delta] })?))
            }
        }
        Ok((input.len(), input[end..].to_string(), self.map.get(&input[end..]).cloned().ok_or_else(|| FmtError::NoSymbol { symbol: &input[end..] })?))
    }
    /// Eat a number
    fn num<'a>(input: &'a str, end: usize) -> Result<(usize, usize), FmtError<'a>> {
        for (delta, ch) in input[end..].char_indices() {
            if !ch.is_ascii_digit() {
                return Ok((end+delta, input[end..end+delta].parse::<usize>().map_err(|_| FmtError::BadNumber { found: &input[end..end+delta] })?))
            }
        }
        Ok((input.len(), input[end..].parse::<usize>().map_err(|_| FmtError::BadNumber { found: &input[end..] })?))
    }
    /// Eat multiple spaces
    fn space<'a>(&self, input: &'a str, end: usize) -> Result<(usize, Fmt), FmtError<'a>> {
        static REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^ +").unwrap());
        let Some(delta) = REGEX.find(&input[end..]) else { Err(FmtError::NotToken)? };
        Ok((end+delta.len(), Fmt::Space))
    }
    /// Parse a token
    fn token<'a>(&self, input: &'a str, end: usize) -> Result<(usize, Fmt), FmtError<'a>> {
        static REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(\\[\[\]\{\}tn\\\? ]|[^\[\]\{\}\? ])+").unwrap());
        let Some(delta) = REGEX.find(&input[end..]) else { Err(FmtError::NotToken)? };
        // Unescape the input
        let mut chars = input[end..end+delta.len()].chars();
        let mut token = String::new();
        while let Some(ch) = chars.next() {
            // Not escape
            if ch != '\\' { token.push(ch); continue }
            // Escape rules
            let Some(ch) = chars.next() else { continue; };
            if matches!(ch, '['|']'|'{'|'}'|'('|')'|' '|'\\'|'?') {
                token.push(ch);
            } else if matches!(ch, 't') {
                token.push('\t');
            } else if matches!(ch, 'n') {
                token.push('\n');
            }
        }
        // Move the cursor
        Ok((end+delta.len(), Fmt::Token { token }))
    }
    /// Parse a SeqExp
    fn seqexp<'a>(&self, input: &'a str, end: usize) -> Result<(usize, Fmt), FmtError<'a>> {
        let (mut end, arg, typ, child, flag) = self.seqexp_item(input, end)?;
        let mut children = vec![(child, flag)];
        loop {
            let Ok((end_, arg_, _, child, flag)) = self.seqexp_item(input, end) else {
                break
            };
            if arg_ != arg {
                break
            };
            end = end_;
            children.push((child, flag));
        }
        Ok((end, Fmt::SeqExp { arg, typ, children }))
    }
    /// Parse a SeqExp item
    fn seqexp_item<'a>(&self, input: &'a str, end: usize) -> Result<(usize, String, Type, Vec<Fmt>, Flag), FmtError<'a>> {
        // TODO: better error message
        // Determine the flag of the push group
        let mut flag = Flag::Null;
        let end = if let Ok(end) = Self::eat("[*", input, end) {
            flag = Flag::Repeat;
            end
        } else if let Ok(end) = Self::eat("[?", input, end) {
            flag = Flag::OrNot;
            end
        } else {
            Self::eat("[", input, end)?
        };
        // Eat the push group identity
        let (end, arg, typ) = self.ident(input, end)?;
        // Eat the ":" token
        let end = Self::eat(":", input, end)?;
        // Analyze component types and make a sub parser
        let inner = match &typ {
            Type::Path(TypePath { path, .. }) => {
                let last = path.segments.last().ok_or_else(|| FmtError::NotGenerics)?;
                let PathArguments::AngleBracketed(ref args) = last.arguments else {
                    Err(FmtError::NotGenerics)?
                };
                use GenericArgument::Type as Ty;
                let arg = args.args.iter()
                    .filter_map(|arg| if let Ty(arg) = arg { Some(arg) } else { None })
                    .last()
                    .ok_or_else(|| FmtError::NotGenerics)?;
                arg
            }
            _ => {
                return Err(FmtError::NotGenerics)
            }
        };
        // Load the subpattern type
        let mut map = HashMap::new();
        match inner {
            Type::Tuple(tuple) => {
                for (symb, ty) in tuple.elems.iter().enumerate() {
                    map.insert(format!("{symb}"), ty.clone());
                }
            }
            other => {
                map.insert(format!("0"), other.clone());
            }
        }
        // Generate a parser
        let subfmt = FmtParser { map };
        let (end, children) = subfmt.many(input, end)?;
        let end = Self::eat("]", input, end)?;
        Ok((end, arg, typ, children, flag))
    }
    /// Parse a symbol (non-terminal)
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
    /// Parse a regular expression
    fn regexp<'a>(&self, input: &'a str, end: usize) -> Result<(usize, Fmt), FmtError<'a>> {
        // Match the number of "#" before "`" and the number of "#" after "`"
        let end = Self::eat("{", input, end)?;
        let (end, arg, typ) = self.ident(input, end)?;
        let end = Self::eat(":", input, end)?;
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
        Ok((end, Fmt::RegExp {
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
        // TODO: tests
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