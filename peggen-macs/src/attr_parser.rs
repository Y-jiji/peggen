use std::collections::HashMap;
use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::*;

use crate::rule_ast::RuleExpr;
use crate::rule_lexer;

pub(crate) struct GrammarContext {
    pub regexes: HashMap<String, String>,
    pub subrules: HashMap<String, RuleExpr>,
}

impl GrammarContext {
    pub fn from_attrs(attrs: &[Attribute]) -> Result<Self> {
        let mut ctx = GrammarContext {
            regexes: HashMap::new(),
            subrules: HashMap::new(),
        };
        for attr in attrs {
            if attr.path().is_ident("regex") {
                ctx.parse_regex_attr(attr)?;
            } else if attr.path().is_ident("subrule") {
                ctx.parse_subrule_attr(attr)?;
            }
        }
        Ok(ctx)
    }

    fn parse_regex_attr(&mut self, attr: &Attribute) -> Result<()> {
        let list = attr.meta.require_list()?;
        let tokens: Vec<proc_macro2::TokenTree> = list.tokens.clone().into_iter().collect();
        let mut i = 0;
        while i < tokens.len() {
            let name = match &tokens[i] {
                proc_macro2::TokenTree::Ident(id) => id.to_string(),
                other => return Err(Error::new_spanned(other, "expected regex name")),
            };
            i += 1;
            if i >= tokens.len() {
                return Err(Error::new_spanned(&list.tokens, "expected '=' after regex name"));
            }
            match &tokens[i] {
                proc_macro2::TokenTree::Punct(p) if p.as_char() == '=' => {}
                other => return Err(Error::new_spanned(other, "expected '='")),
            }
            i += 1;
            if i >= tokens.len() {
                return Err(Error::new_spanned(
                    &list.tokens,
                    "expected regex pattern after '='",
                ));
            }
            let pattern = match &tokens[i] {
                proc_macro2::TokenTree::Literal(lit) => {
                    let lit_str: LitStr = syn::parse2(lit.to_token_stream())
                        .map_err(|e| Error::new_spanned(lit, format!("bad regex pattern: {e}")))?;
                    lit_str.value()
                }
                other => return Err(Error::new_spanned(other, "expected string literal for regex")),
            };
            i += 1;
            self.regexes.insert(name, pattern);
            if i < tokens.len() {
                match &tokens[i] {
                    proc_macro2::TokenTree::Punct(p) if p.as_char() == ',' => {
                        i += 1;
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }

    fn parse_subrule_attr(&mut self, attr: &Attribute) -> Result<()> {
        let list = attr.meta.require_list()?;
        let tokens: Vec<proc_macro2::TokenTree> = list.tokens.clone().into_iter().collect();
        if tokens.len() < 3 {
            return Err(Error::new_spanned(
                &list.tokens,
                "subrule needs: name = pattern",
            ));
        }
        let name = match &tokens[0] {
            proc_macro2::TokenTree::Ident(id) => id.to_string(),
            other => return Err(Error::new_spanned(other, "expected subrule name")),
        };
        match &tokens[1] {
            proc_macro2::TokenTree::Punct(p) if p.as_char() == '=' => {}
            other => return Err(Error::new_spanned(other, "expected '='")),
        }
        let rest: TokenStream = tokens[2..].iter().map(|t| t.to_token_stream()).collect();
        let expr = parse_rule_tokens(rest, attr)?;
        self.subrules.insert(name, expr);
        Ok(())
    }
}

pub(crate) fn parse_tags(attrs: &[Attribute]) -> Result<Vec<String>> {
    let mut tags = Vec::new();
    for attr in attrs {
        if !attr.path().is_ident("tag") {
            continue;
        }
        let list = attr.meta.require_list()?;
        for tt in list.tokens.clone() {
            match tt {
                proc_macro2::TokenTree::Ident(id) => tags.push(id.to_string()),
                proc_macro2::TokenTree::Punct(p) if p.as_char() == ',' => {}
                other => return Err(Error::new_spanned(other, "expected tag name")),
            }
        }
    }
    Ok(tags)
}

pub(crate) fn parse_rule_tokens(ts: TokenStream, span_source: &impl ToTokens) -> Result<RuleExpr> {
    let tokens = rule_lexer::lex(ts)
        .map_err(|e| Error::new_spanned(span_source, format!("lex error: {e}")))?;
    let parser = crate::rule_grammar::RuleParser::new();
    parser
        .parse(tokens.into_iter().map(Ok))
        .map_err(|e| Error::new_spanned(span_source, format!("parse error: {e}")))
}
