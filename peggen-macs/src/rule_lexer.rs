use proc_macro2::{Delimiter, TokenStream, TokenTree};
use quote::ToTokens;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum RuleTok {
    Lit(String),
    Ident(String),
    Num(usize),
    Dollar,
    Colon,
    At,
    Star,
    Plus,
    Question,
    Percent,
    Pipe,
    Bang,
    Ampersand,
    LParen,
    RParen,
    MultiField(Vec<crate::rule_ast::FieldRef>),
}

impl std::fmt::Display for RuleTok {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuleTok::Lit(s) => write!(f, "\"{s}\""),
            RuleTok::Ident(s) => write!(f, "{s}"),
            RuleTok::Num(n) => write!(f, "{n}"),
            RuleTok::Dollar => write!(f, "$"),
            RuleTok::Colon => write!(f, ":"),
            RuleTok::At => write!(f, "@"),
            RuleTok::Star => write!(f, "*"),
            RuleTok::Plus => write!(f, "+"),
            RuleTok::Question => write!(f, "?"),
            RuleTok::Percent => write!(f, "%"),
            RuleTok::Pipe => write!(f, "|"),
            RuleTok::Bang => write!(f, "!"),
            RuleTok::Ampersand => write!(f, "&"),
            RuleTok::LParen => write!(f, "("),
            RuleTok::RParen => write!(f, ")"),
            RuleTok::MultiField(refs) => {
                write!(f, "${{")?;
                for (i, r) in refs.iter().enumerate() {
                    if i > 0 { write!(f, ",")?; }
                    write!(f, "{}", r.key())?;
                }
                write!(f, "}}")
            }
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct LexError(pub String);

impl std::fmt::Display for LexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub fn lex(ts: TokenStream) -> Result<Vec<(usize, RuleTok, usize)>, LexError> {
    let mut tokens = Vec::new();
    let mut pos = 0;
    lex_inner(ts, &mut tokens, &mut pos)?;
    Ok(tokens)
}

fn lex_inner(
    ts: TokenStream,
    out: &mut Vec<(usize, RuleTok, usize)>,
    pos: &mut usize,
) -> Result<(), LexError> {
    let tokens: Vec<TokenTree> = ts.into_iter().collect();
    let mut i = 0;
    while i < tokens.len() {
        match &tokens[i] {
            TokenTree::Group(g) => {
                match g.delimiter() {
                    Delimiter::Parenthesis => {
                        let start = *pos;
                        *pos += 1;
                        out.push((start, RuleTok::LParen, *pos));
                        lex_inner(g.stream(), out, pos)?;
                        let start = *pos;
                        *pos += 1;
                        out.push((start, RuleTok::RParen, *pos));
                    }
                    Delimiter::None => {
                        lex_inner(g.stream(), out, pos)?;
                    }
                    Delimiter::Brace => {
                        return Err(LexError("bare { } not supported in rules; use \"{\" and \"}\" string literals, or ${field1,field2}:subrule".into()));
                    }
                    Delimiter::Bracket => {
                        return Err(LexError("bare [ ] not supported in rules; use \"[\" and \"]\" string literals".into()));
                    }
                }
            }
            TokenTree::Punct(p) if p.as_char() == '$' => {
                if i + 1 < tokens.len() {
                    if let TokenTree::Group(g) = &tokens[i + 1] {
                        if g.delimiter() == Delimiter::Brace {
                            let fields = parse_multi_field(g.stream())?;
                            let start = *pos;
                            *pos += 2;
                            out.push((start, RuleTok::MultiField(fields), *pos));
                            i += 2;
                            continue;
                        }
                    }
                }
                let start = *pos;
                *pos += 1;
                out.push((start, RuleTok::Dollar, *pos));
            }
            TokenTree::Punct(p) => {
                let tok = match p.as_char() {
                    ':' => RuleTok::Colon,
                    '@' => RuleTok::At,
                    '*' => RuleTok::Star,
                    '+' => RuleTok::Plus,
                    '?' => RuleTok::Question,
                    '%' => RuleTok::Percent,
                    '|' => RuleTok::Pipe,
                    '!' => RuleTok::Bang,
                    '&' => RuleTok::Ampersand,
                    c => return Err(LexError(format!("unexpected punctuation '{c}' in rule"))),
                };
                let start = *pos;
                *pos += 1;
                out.push((start, tok, *pos));
            }
            TokenTree::Ident(id) => {
                let start = *pos;
                *pos += 1;
                out.push((start, RuleTok::Ident(id.to_string()), *pos));
            }
            TokenTree::Literal(lit) => {
                let s = lit.to_string();
                let tok = if s.starts_with('"') || s.starts_with("r\"") || s.starts_with("r#") {
                    let lit_str: syn::LitStr = syn::parse2(lit.to_token_stream())
                        .map_err(|e| LexError(format!("bad string literal: {e}")))?;
                    RuleTok::Lit(lit_str.value())
                } else if let Ok(n) = s.parse::<usize>() {
                    RuleTok::Num(n)
                } else {
                    return Err(LexError(format!("unexpected literal: {s}; use string literals (\"...\") for terminals")));
                };
                let start = *pos;
                *pos += 1;
                out.push((start, tok, *pos));
            }
        }
        i += 1;
    }
    Ok(())
}

fn parse_multi_field(ts: TokenStream) -> Result<Vec<crate::rule_ast::FieldRef>, LexError> {
    use crate::rule_ast::FieldRef;
    let mut fields = Vec::new();
    for tt in ts {
        match tt {
            TokenTree::Literal(lit) => {
                let s = lit.to_string();
                if let Ok(n) = s.parse::<usize>() {
                    fields.push(FieldRef::Positional(n));
                } else {
                    return Err(LexError(format!("expected field index in ${{...}}, got: {s}")));
                }
            }
            TokenTree::Ident(id) => {
                fields.push(FieldRef::Named(id.to_string()));
            }
            TokenTree::Punct(p) if p.as_char() == ',' => {}
            other => return Err(LexError(format!("unexpected token in ${{...}}: {other}"))),
        }
    }
    if fields.len() < 2 {
        return Err(LexError("${...} requires at least 2 field references".into()));
    }
    Ok(fields)
}
