use std::collections::HashMap;
use punctuated::Punctuated;
use quote::ToTokens;
use token::Comma;

use crate::*;
use crate::rule_ast::*;
use crate::attr_parser::*;

mod ast_impl_build;
mod num_build;
mod rules_impl_build;
mod parse_impl_build;
mod first_set;
pub use ast_impl_build::*;
pub use num_build::*;
pub(crate) use first_set::*;

pub(crate) struct ImplMode {
    pub parse_trait: TokenStream,
    pub parse_method: TokenStream,
    pub rule_trait: TokenStream,
    pub rule_method: TokenStream,
    pub optimized: bool,
}

impl ImplMode {
    pub fn normal() -> Self {
        ImplMode {
            parse_trait: quote! { #CRATE::ParseImpl },
            parse_method: quote! { parse_impl },
            rule_trait: quote! { #CRATE::RuleImpl },
            rule_method: quote! { rule_impl },
            optimized: true,
        }
    }
    pub fn reference() -> Self {
        ImplMode {
            parse_trait: quote! { #CRATE::RefParseImpl },
            parse_method: quote! { ref_parse_impl },
            rule_trait: quote! { #CRATE::RefRuleImpl },
            rule_method: quote! { ref_rule_impl },
            optimized: false,
        }
    }
}

pub(crate) struct Rule {
    pub group: usize,
    pub tags: Vec<String>,
    pub named: bool,
    pub trace: bool,
    pub error: bool,
    pub body: RuleExpr,
    pub variant: Ident,
    pub fields: HashMap<String, Type>,
}

impl std::fmt::Debug for Rule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Rule")
            .field("group", &self.group)
            .field("tags", &self.tags)
            .field("named", &self.named)
            .field("trace", &self.trace)
            .field("error", &self.error)
            .field("body", &self.body)
            .field("variant", &self.variant.to_string())
            .finish()
    }
}

impl Rule {
    pub fn new(fields: Fields, ident: Ident, attr: Attribute, _ctx: &GrammarContext, all_tags: &[String]) -> Result<Rule> {
        let mut rule = Rule {
            group: 0,
            tags: vec![],
            named: matches!(fields, Fields::Named(..) | Fields::Unit),
            error: false,
            trace: false,
            body: RuleExpr::Literal(String::new()),
            variant: ident.clone(),
            fields: HashMap::new(),
        };
        match fields {
            Fields::Named(FieldsNamed { named, .. }) => {
                for field in named {
                    rule.fields.insert(
                        field.ident.to_token_stream().to_string(),
                        field.ty,
                    );
                }
            }
            Fields::Unnamed(FieldsUnnamed { unnamed, .. }) => {
                for (i, field) in unnamed.into_iter().enumerate() {
                    rule.fields.insert(format!("{i}"), field.ty);
                }
            }
            Fields::Unit => {}
        }
        let args = attr.meta.require_list()?;
        rule.body = parse_rule_tokens(args.tokens.clone(), &attr)?;

        if !all_tags.is_empty() {
            rule.group = 0;
        }

        Ok(rule)
    }
}

pub(crate) struct Builder {
    pub rules: Vec<Rule>,
    pub group: usize,
    pub all_tags: Vec<String>,
    pub is_enum: bool,
    pub ident: Ident,
    pub with: Option<TokenStream>,
    pub generics: Punctuated<GenericParam, Comma>,
    pub context: GrammarContext,
}

impl std::fmt::Debug for Builder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Builder {{ rules: {:?}, group: {:?}, ident: {:?}, generics: {:?} }}",
            self.rules, self.group, self.ident.to_string(), self.generics.to_token_stream().to_string())
    }
}

impl Builder {
    pub fn new(input: DeriveInput) -> Result<Self> {
        let context = GrammarContext::from_attrs(&input.attrs)?;

        let mut this = Builder {
            with: match input.attrs.iter().filter(|x| x.path().is_ident("with")).next().cloned() {
                None => None,
                Some(Attribute { meta, .. }) => Some(meta.require_list()?.tokens.to_token_stream()),
            },
            rules: vec![],
            group: 0,
            all_tags: vec![],
            is_enum: false,
            ident: input.ident.clone(),
            generics: {
                let mut p = input.generics.clone().params;
                if !p.empty_or_trailing() { p.push_punct(Comma::default()); }
                p
            },
            context,
        };
        match input.data {
            Data::Struct(r#struct) => {
                let tags = parse_tags(&input.attrs)?;
                for attr in input.attrs {
                    if attr.path().to_token_stream().to_string() != "rule" {
                        continue;
                    }
                    let mut rule = Rule::new(
                        r#struct.fields.clone(),
                        input.ident.clone(),
                        attr,
                        &this.context,
                        &this.all_tags,
                    )?;
                    rule.tags = tags.clone();
                    this.add_rule(rule);
                }
                this.is_enum = false;
                Ok(this)
            },
            Data::Enum(r#enum) => {
                let mut has_tags = false;
                let mut has_no_tags = false;
                let variants: Vec<_> = r#enum.variants.into_iter()
                    .filter(|var| var.attrs.iter().any(|x| x.path().is_ident("rule")))
                    .collect();
                for variant in &variants {
                    let tags = parse_tags(&variant.attrs)?;
                    if !tags.is_empty() {
                        has_tags = true;
                        for t in &tags {
                            if !this.all_tags.contains(t) {
                                this.all_tags.push(t.clone());
                            }
                        }
                    } else {
                        has_no_tags = true;
                    }
                }
                if has_tags && has_no_tags {
                    return Err(Error::new_spanned(
                        input.ident,
                        "if any variant has #[tag(...)], all variants with #[rule] must have #[tag(...)]"
                    ));
                }
                for variant in variants {
                    let tags = parse_tags(&variant.attrs)?;
                    for attr in variant.attrs.iter().filter(|a| a.path().is_ident("rule")) {
                        let mut rule = Rule::new(
                            variant.fields.clone(),
                            variant.ident.clone(),
                            attr.clone(),
                            &this.context,
                            &this.all_tags,
                        )?;
                        rule.tags = tags.clone();
                        this.add_rule(rule);
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
    pub fn tag_index(&self, name: &str) -> Result<usize> {
        self.all_tags.iter().position(|t| t == name)
            .ok_or_else(|| Error::new(
                proc_macro2::Span::call_site(),
                format!("unknown tag '{name}', known tags: {:?}", self.all_tags)
            ))
    }
    pub fn max_group(&self) -> usize {
        if self.all_tags.is_empty() {
            self.group
        } else {
            self.all_tags.len() - 1
        }
    }
}

impl Builder {
    pub fn type_stub_build(&self) -> Result<TokenStream> {
        let this = &self.ident;
        let generics = &self.generics;
        let has_lifetimes = generics.iter().any(|p| matches!(p, GenericParam::Lifetime(_)));

        if !has_lifetimes {
            return Ok(quote! {
                impl<#generics> #CRATE::PeggenTypeStub for #this<#generics> {
                    type Reflect<'__peggen_a> = Self;
                }
            });
        }

        let reflect_args: Vec<TokenStream> = generics.iter().filter_map(|p| {
            match p {
                GenericParam::Lifetime(_) => Some(quote! { '__peggen_a }),
                GenericParam::Type(tp) => { let id = &tp.ident; Some(quote! { #id }) },
                GenericParam::Const(cp) => { let id = &cp.ident; Some(quote! { #id }) },
            }
        }).collect();

        Ok(quote! {
            impl<#generics> #CRATE::PeggenTypeStub for #this<#generics> {
                type Reflect<'__peggen_a> = #this<#(#reflect_args),*>;
            }
        })
    }
}

pub(crate) fn extract_item_type(typ: &Type) -> Option<Type> {
    match typ {
        Type::Path(TypePath { path, .. }) => {
            let last = path.segments.last()?;
            let PathArguments::AngleBracketed(ref args) = last.arguments else {
                return None;
            };
            use GenericArgument::Type as Ty;
            args.args.iter()
                .filter_map(|arg| if let Ty(arg) = arg { Some(arg) } else { None })
                .last()
                .cloned()
        }
        _ => None,
    }
}

pub(crate) fn build_item_fields(item_type: &Type) -> HashMap<String, Type> {
    let mut map = HashMap::new();
    match item_type {
        Type::Tuple(tuple) => {
            for (i, ty) in tuple.elems.iter().enumerate() {
                map.insert(format!("{i}"), ty.clone());
            }
        }
        other => {
            map.insert("0".to_string(), other.clone());
        }
    }
    map
}

pub(crate) fn expand_subrule_refs(expr: &RuleExpr, subrules: &HashMap<String, RuleExpr>) -> RuleExpr {
    match expr {
        RuleExpr::FieldRegex(_fref, name) => {
            if let Some(body) = subrules.get(name) {
                body.clone()
            } else {
                expr.clone()
            }
        }
        RuleExpr::Seq(elems) => {
            RuleExpr::Seq(elems.iter().map(|e| expand_subrule_refs(e, subrules)).collect())
        }
        RuleExpr::Choice(a, b) => {
            RuleExpr::Choice(
                Box::new(expand_subrule_refs(a, subrules)),
                Box::new(expand_subrule_refs(b, subrules)),
            )
        }
        RuleExpr::Not(e) => RuleExpr::Not(Box::new(expand_subrule_refs(e, subrules))),
        RuleExpr::And(e) => RuleExpr::And(Box::new(expand_subrule_refs(e, subrules))),
        _ => expr.clone(),
    }
}

pub(crate) fn remap_subrule_fields(expr: &RuleExpr, mapping: &[FieldRef]) -> RuleExpr {
    match expr {
        RuleExpr::Field(FieldRef::Positional(i)) if *i < mapping.len() => {
            RuleExpr::Field(mapping[*i].clone())
        }
        RuleExpr::FieldRegex(FieldRef::Positional(i), name) if *i < mapping.len() => {
            RuleExpr::FieldRegex(mapping[*i].clone(), name.clone())
        }
        RuleExpr::FieldTag(FieldRef::Positional(i), tag) if *i < mapping.len() => {
            RuleExpr::FieldTag(mapping[*i].clone(), tag.clone())
        }
        RuleExpr::Seq(elems) => {
            RuleExpr::Seq(elems.iter().map(|e| remap_subrule_fields(e, mapping)).collect())
        }
        RuleExpr::Choice(a, b) => {
            RuleExpr::Choice(
                Box::new(remap_subrule_fields(a, mapping)),
                Box::new(remap_subrule_fields(b, mapping)),
            )
        }
        RuleExpr::Rep(e, k) => RuleExpr::Rep(Box::new(remap_subrule_fields(e, mapping)), *k),
        RuleExpr::SepRep { expr, sep, at_least_one } => {
            RuleExpr::SepRep {
                expr: Box::new(remap_subrule_fields(expr, mapping)),
                sep: Box::new(remap_subrule_fields(sep, mapping)),
                at_least_one: *at_least_one,
            }
        }
        RuleExpr::Not(e) => RuleExpr::Not(Box::new(remap_subrule_fields(e, mapping))),
        RuleExpr::And(e) => RuleExpr::And(Box::new(remap_subrule_fields(e, mapping))),
        _ => expr.clone(),
    }
}

pub(crate) fn resolve_rep_fields(
    body: &RuleExpr,
    outer_fields: &HashMap<String, Type>,
) -> Option<(String, HashMap<String, Type>)> {
    if let RuleExpr::Field(fref) | RuleExpr::FieldTag(fref, _) = body {
        let key = fref.key();
        if let Some(typ) = outer_fields.get(&key) {
            if let Some(item_type) = extract_item_type(typ) {
                return Some((key, build_item_fields(&item_type)));
            }
        }
        return None;
    }
    let mut refs = vec![];
    body.collect_field_refs(&mut refs);
    if refs.is_empty() {
        return None;
    }
    for key in ["0", "1", "2", "3", "4", "5", "6", "7"] {
        if let Some(typ) = outer_fields.get(key) {
            if let Some(item_type) = extract_item_type(typ) {
                return Some((key.to_string(), build_item_fields(&item_type)));
            }
        }
    }
    for (key, typ) in outer_fields {
        if let Some(item_type) = extract_item_type(typ) {
            return Some((key.clone(), build_item_fields(&item_type)));
        }
    }
    None
}
