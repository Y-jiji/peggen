use std::{collections::{HashMap, HashSet}, fmt::Debug, hash::Hash};
use proc_macro::TokenStream;
use quote::ToTokens;
use syn::{self, Ident, punctuated::Punctuated, token::Comma, Attribute, Error, Field, FieldsNamed, FieldsUnnamed, Meta, Result};
use crate::*;

// Implement parser_impl_ ... for a variant
pub fn impl_parser_variant(name: &syn::Ident, var: syn::Variant) -> Result<(Ident, TokenStream)> {
    let func = prepare_func_name(&var);
    let func = syn::parse_str::<syn::Ident>(&func).unwrap();
    let attr = prepare_attribute(&var)?;
    let body = prepare_func_body(&var, &attr)?;
    let body = syn::parse_str::<syn::Expr>(&body).unwrap();
    Ok((func.clone(), quote::quote! {
        impl #name {
            fn #func() -> Result<usize, usize> {
                #body
            }
        }
    }.into()))
}

// Get function name from variant name: e.g. CamelCaseA -> parser_impl_camel_case_a
fn prepare_func_name(var: &syn::Variant) -> String {
    let mut name_fn = var.ident.to_string();
    for i in 'A'..='Z' {
        name_fn = name_fn.replace(i, &format!("_{}", i.to_lowercase()));
    }
    let name_fn = name_fn.trim_start_matches("_").to_string();
    let name_fn = format!("parser_impl_{name_fn}");
    return name_fn;
}

// Remove unwanted cases in attribute
fn prepare_attribute(var: &syn::Variant) -> Result<String> {
    if matches!(var.fields, syn::Fields::Unit) {
        Err(Error::new_spanned(
            var, 
            "#[derive(ParserImpl)] can only handle non-unit variants, e.g. A(...) or A{...}"
        ))?
    }
    let Some(Attribute { meta: Meta::List(fmt), .. }) = var.attrs.last() else {
        Err(Error::new_spanned(
            var, 
            format!("#[derive(ParserImpl)] can only handle variants marked with attribute #[parse(\"...\")]")
        ))?
    };
    let Some(proc_macro2::TokenTree::Literal(fmt)) = fmt.tokens.clone().into_iter().next() else {
        Err(Error::new_spanned(
            var, 
            format!("#[derive(ParserImpl)] can only handle variants marked with attribute #[parse(\"...\")]")
        ))?
    };
    let fmt = fmt.to_string();
    let fmt = &fmt[1..fmt.len()-1];
    Ok(fmt.to_string())
}

// Prepare function body from format string
fn prepare_func_body(var: &syn::Variant, fmt: &str) -> Result<String> {
    let fmt = parse_fmt_string(fmt)
        .map_err(|_| Error::new_spanned(var, format!("cannot parse the format string")))?;
    if let syn::Fields::Named(FieldsNamed { named, .. }) = &var.fields {
        return prepare_func_body_named(var, fmt, named);
    }
    else if let syn::Fields::Unnamed(FieldsUnnamed { unnamed, .. }) = &var.fields {
        return prepare_func_body_unnamed(var, fmt, unnamed);
    }
    unreachable!()
}

/// Generate function body for unnamed fields
fn prepare_func_body_unnamed(var: &syn::Variant, fmt: Vec<Token>, unnamed: &Punctuated<Field, Comma>) -> Result<String> {
    use Token::*;
    use std::fmt::Write;
    let holes = fmt.iter().try_fold(HashSet::new(), |mut set, token| match token {
        Space{..} | Literal{..} => Ok(set),
        Format((fmt, _bp)) => match fmt.parse::<usize>() {
            Ok(out) if !set.contains(&out) => {set.insert(out); Ok(set)},
            Ok(out) => Err(Error::new_spanned(var, format!("repeated format symbol {{{}}}", out))),
            Err(_) => Err(Error::new_spanned(var, format!("the variant is named, but we get {{{}}}", fmt))),
        },
    })?;
    let fills = (0..var.fields.len()).collect::<HashSet<usize>>();
    difference(var, holes, fills)?;
    let fields = unnamed.iter().map(|x| x.ty.clone()).collect::<Vec<_>>();
    // write function body
    let mut body = String::new();
    for token in fmt {
        match token {
            Format((fmt, _bp)) => {
                let ty = fields[fmt.parse::<usize>().unwrap()].to_token_stream().to_string();
                writeln!(&mut body, "let (_{fmt}, p) = {ty}::parser_impl(input, p)?;").unwrap();
            }
            Space => {
                writeln!(&mut body, "let p = Self::space({INPUT}, p)?;").unwrap();
            }
            Literal(literal) => {
                let len = literal.len();
                writeln!(&mut body, "let p = if {INPUT}[p..p+{len}] == \"{literal}\" {{ p + {len} }} else {{ Err(p)? }};").unwrap();
            }
        }
    }
    Ok(body)
}

/// Generate function body for named fields
fn prepare_func_body_named(var: &syn::Variant, fmt: Vec<Token>, named: &Punctuated<Field, Comma>) -> Result<String> {
    use Token::*;
    use std::fmt::Write;
    let holes = fmt.iter().try_fold(HashSet::new(), |mut set, token| match token {
        Space{..} | Literal{..} => Ok(set),
        Format((fmt, _)) if set.contains(fmt) => Err(Error::new_spanned(var, format!("repeated format symbol {{{}}}", fmt))),
        Format((fmt, _)) => match fmt.parse::<usize>() {
            Err(_) => {set.insert(fmt.clone()); Ok(set)},
            Ok(_) => Err(Error::new_spanned(var, format!("the variant is named, but we get {{{}}}", fmt))),
        },
    })?;
    let fills = named.iter().map(|x| x.ident.as_ref().unwrap().to_string()).collect::<HashSet<String>>();
    difference(var, holes, fills)?;
    let fields = named.iter().map(|x| (x.ident.as_ref().unwrap().to_string(), x.ty.clone())).collect::<HashMap<_, _>>();
    // write function body
    let mut body = String::new();
    for token in fmt {
        match token {
            Format((fmt, _)) => {
                let ty = fields[&fmt].to_token_stream().to_string();
                writeln!(&mut body, "let ({fmt}, p) = {ty}::parser_impl({INPUT}, p)?;").unwrap();
            }
            Space => {
                writeln!(&mut body, "let p = Self::space({INPUT}, p)?;").unwrap();
            }
            Literal(literal) => {
                let len = literal.len();
                writeln!(&mut body, "let p = if {INPUT}[p..p+{len}] == \"{literal}\" {{ p + {len} }} else {{ Err(p)? }};").unwrap();
            }
        }
    }
    Ok(body)
}

/// Compute the difference between holes in fmt string and the fields
fn difference<T: Eq + Hash + Debug>(var: &syn::Variant, holes: HashSet<T>, fills: HashSet<T>) -> Result<()> {
    if holes == fills { return Ok(()) }
    let difference = holes.symmetric_difference(&fills);
    Err(Error::new_spanned(var, format!("{:?}", difference.collect::<HashSet<&T>>())))
}