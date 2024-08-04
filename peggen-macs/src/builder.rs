use std::sync::LazyLock;
use quote::ToTokens;
use regex::Regex;

use crate::*;
mod ast_impl_build;
mod num_build;
mod rules_impl_build;
mod parse_impl_build;
pub use ast_impl_build::*;
pub use num_build::*;
pub use rules_impl_build::*;
pub use parse_impl_build::*;

#[derive(Debug)]
pub(crate) struct Rule {
    group: usize,
    named: bool,
    trace: bool,
    error: bool,
    exprs: Vec<Fmt>,
    ident: Ident,
}

impl Rule {
    pub fn new(fields: Fields, ident: Ident, attr: Attribute) -> Result<Rule> {
        let mut rule = Rule {
            group: 0,
            named: matches!(fields, Fields::Named(..)),
            error: false,
            trace: false,
            exprs: vec![],
            ident: ident.clone(),
        };
        let fmt = FmtParser::new(fields)?;
        // The arguments to parse
        let args = attr.meta.require_list()?;
        let mut tokens = args.tokens.clone().into_iter();
        // Iterate over all the tokens within attribute marker
        while let Some(token) = tokens.next() {
            macro_rules! expect {
                ($x: literal) => {{
                    let Some(token) = tokens.next() else {
                        Err(Error::new_spanned(args, format!("expected {}, tokens depleted", $x)))?
                    };
                    if token.to_string() != $x {
                        Err(Error::new_spanned(token, format!("expected {}", $x)))?
                    }
                }};
            }
            macro_rules! get {
                ($T: ty) => {{
                    let Some(token) = tokens.next() else {
                        Err(Error::new_spanned(args, format!("tokens depleted")))?
                    };
                    let value = token.to_string().parse::<$T>().map_err(|_| 
                        Error::new_spanned(token.clone(), format!("expected {} found {token}", stringify!($T)))
                    )?;
                    value
                }};
            }
            if let TokenTree::Literal(lit) = token {
                static REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new("^r#*\"").unwrap());
                let s = lit.to_string();
                let s = if let Some(i) = REGEX.find(&s) {
                    &s[i.len()..s.len()-i.len()+1]
                } else if s.starts_with("\"") {
                    &s[1..s.len()-1]
                } else {
                    continue;
                };
                let (_, exprs) = fmt.many(&s, 0)
                    .map_err(|e| Error::new_spanned(lit.clone(), format!("{e:?}")))?;
                if rule.exprs.is_empty() {
                    rule.exprs = exprs;
                }
                else {
                    Err(Error::new_spanned(lit, format!("Why do you want a second format string?")))?
                }
                continue;
            }
            if let TokenTree::Ident(ident) = token {
                match ident.to_string().as_str() {
                    "group" => {
                        expect!("=");
                        let g = get!(usize);
                        rule.group = g;
                    },
                    "error" => {
                        rule.error = true;
                    },
                    "trace" => {
                        rule.trace = true;
                    }
                    _ => {
                        Err(Error::new_spanned(ident, "bad identity, expect group"))?
                    }
                }
            }
        }
        Ok(rule)
    }
}

/// A builder type for building parser
pub(crate) struct Builder {
    /// All the rules. 
    rules: Vec<Rule>,
    /// Largest group number
    group: usize,
    /// If this type is enum
    is_enum: bool,
    /// The identity of this type
    ident: Ident,
    /// The attributes attached to type
    attrs: Vec<Attribute>,
    /// The generics of this type
    generics: Generics,
}

impl std::fmt::Debug for Builder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RuleGroup {{ rules: {:?}, group: {:?}, ident: {:?}, generics: {:?} }}", 
            self.rules, self.group, self.ident.to_string(), self.generics.to_token_stream().to_string())
    }
}

impl Builder {
    pub fn new(input: DeriveInput) -> Result<Self> {
        let mut this = Builder {
            rules: vec![],
            group: 0,
            is_enum: false,
            attrs: input.attrs.clone(),
            ident: input.ident.clone(),
            generics: input.generics.clone()
        };
        match input.data {
            Data::Struct(r#struct) => {
                for attr in input.attrs {
                    if attr.path().to_token_stream().to_string() != "rule" {
                        continue;
                    }
                    this.add_rule(Rule::new(
                        r#struct.fields.clone(), 
                        input.ident.clone(), 
                        attr
                    )?);
                }
                this.is_enum = false;
                Ok(this)
            },
            Data::Enum(r#enum) => {
                let variants = r#enum.variants.into_iter()
                    .filter(|var| var.attrs.iter().filter(|x| x.path().is_ident("rule")).next().is_some());
                for variant in variants {
                    for attr in variant.attrs {
                        if attr.path().to_token_stream().to_string() != "rule" {
                            continue;
                        }
                        this.add_rule(Rule::new(
                            variant.fields.clone(), 
                            variant.ident.clone(), 
                            attr
                        )?);
                    }
                }
                this.is_enum = true;
                Ok(this)
            }
            Data::Union(_) => Err(Error::new_spanned(
                input, 
                "expect derive(ParseImpl) to work on enum or struct, but we get union. "
            )),
        }
    }
    pub fn add_rule(&mut self, rule: Rule) {
        self.group = self.group.max(rule.group);
        self.rules.push(rule);
    }
}