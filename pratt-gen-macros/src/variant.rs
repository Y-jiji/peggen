use std::{collections::{HashMap, HashSet}, fmt::Debug, hash::Hash};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{self, punctuated::Punctuated, token::Comma, Attribute, Error, Field, FieldsNamed, FieldsUnnamed, Ident, Meta, Result};
use crate::*;

// Implement parser_impl_ ... for a variant
pub fn impl_parser_variant(name: &syn::Ident, var: syn::Variant) -> Result<(Ident, TokenStream)> {
    let func = prepare_func_name(&var);
    let (attr, precedence) = prepare_attribute(&var)?;
    let body = prepare_func_body(&var, &attr, precedence)?;
    Ok((func.clone(), quote::quote! {
        impl<'a> #name<'a> {
            #[inline(always)]
            fn #func(
                source: Source<'a>, 
                out_arena: &'a Arena,
                err_arena: &'a Arena,
                precedence: u16,
            ) -> Result<(Self, Source<'a>), Error<'a>> {
                let name = stringify!{#name};
                #body
            }
        }
    }.into()))
}

// Get function name from variant name: e.g. CamelCaseA -> parser_impl_camel_case_a
fn prepare_func_name(var: &syn::Variant) -> Ident {
    let mut name_fn = var.ident.to_string();
    for i in 'A'..='Z' {
        name_fn = name_fn.replace(i, &format!("_{}", i.to_lowercase()));
    }
    let name_fn = name_fn.trim_start_matches("_").to_string();
    let name_fn = format!("parser_impl_{name_fn}");
    return syn::parse_str::<syn::Ident>(&name_fn).unwrap();
}

// Remove unwanted cases in attribute
fn prepare_attribute(var: &syn::Variant) -> Result<(String, u16)> {
    if matches!(var.fields, syn::Fields::Unit) {
        Err(Error::new_spanned(
            var, 
            "#[derive(ParseImpl)] can only handle non-unit variants, e.g. A(...) or A{...}"
        ))?
    }
    let Some(Attribute { meta: Meta::List(meta), .. }) = var.attrs.last() else {
        Err(Error::new_spanned(
            var, 
            format!("#[derive(ParseImpl)] can only handle variants marked with attribute #[parse(pat=\"...\", precedence=...)]")
        ))?
    };
    let mut meta = meta.tokens.clone().into_iter();
    let Some(proc_macro2::TokenTree::Literal(fmt)) = meta.next() else {
        Err(Error::new_spanned(
            var, 
            format!("#[derive(ParseImpl)] can only handle variants marked with attribute #[parse(\"...\")]")
        ))?
    };
    let fmt = fmt.to_string();
    let fmt = &fmt[1..fmt.len()-1];
    let mut meta = meta.map(|x| x.to_string());
    let mut precedence = u16::MAX;
    if meta.next() == Some(String::from(",")) {
        if meta.next() == Some(String::from("precedence")) {
            if meta.next() == Some(String::from("=")) {
                if let Some(p) = meta.next().map(|x| x.parse().ok()).flatten() {
                    precedence = p;
                }
            }
        }
    }
    Ok((fmt.to_string(), precedence))
}

// Prepare function body from format string
fn prepare_func_body(var: &syn::Variant, fmt: &str, precedence: u16) -> Result<TokenStream> {
    let fmt = parse_fmt_string(fmt)
        .map_err(|_| Error::new_spanned(var, format!("cannot parse the format string")))?;
    if let syn::Fields::Named(FieldsNamed { named, .. }) = &var.fields {
        return prepare_func_body_named(var, fmt, named, precedence);
    }
    else if let syn::Fields::Unnamed(FieldsUnnamed { unnamed, .. }) = &var.fields {
        return prepare_func_body_unnamed(var, fmt, unnamed, precedence);
    }
    unreachable!()
}

/// Generate function body for unnamed fields
fn prepare_func_body_unnamed(var: &syn::Variant, fmt: Vec<Token>, unnamed: &Punctuated<Field, Comma>, precedence: u16) -> Result<TokenStream> {
    use Token::*;
    // Check holes and fills match
    let holes = fmt.iter().try_fold(HashSet::new(), |mut set, token| match token {
        Space{..} | Literal{..} => Ok(set),
        Format((fmt, ..)) | Regex((fmt, ..)) => match fmt.parse::<usize>() {
            Ok(out) if !set.contains(&out) => {set.insert(out); Ok(set)},
            Ok(out) => Err(Error::new_spanned(var, format!("repeated format symbol {{{}}}", out))),
            Err(_) => Err(Error::new_spanned(var, format!("the variant is named, but we get {{{}}}", fmt))),
        },
    })?;
    let fills = (0..var.fields.len()).collect::<HashSet<usize>>();
    difference(var, holes, fills)?;
    let fields = unnamed.iter().map(|x| x.ty.clone()).collect::<Vec<_>>();
    // Write function body and variant arguments
    let mut body = TokenStream::new();
    let mut args = TokenStream::new();
    body.extend(quote! {if precedence >= #precedence { return Err(Error::Precedence); }});
    {
        let var = var.ident.to_string();
        body.extend(quote!{
            #[cfg(feature="trace")]
            println!("rule {name}::{} @ {}; rest {}", #var, source.split, &source[..source[..].chars().next().map(|x| x.len_utf8()).unwrap_or(0)]);
        });
    }
    // For each token, write a rule
    for token in fmt {match token {
        Literal(literal) => 
            body.extend(quote! {let source = token(source, &err_arena, #literal)?;}),
        Space => 
            body.extend(quote! {let source = Self::space(source)?;}),
        Format((fmt, bp)) => {
            let id = syn::parse_str::<Ident>(&format!("_{fmt}")).unwrap();
            let ty = fields[fmt.parse::<usize>().unwrap()].clone();
            body.extend(quote! {
                let (#id, source) = <#ty>::parser_impl(source, out_arena, err_arena, #bp)?;
            });
            args.extend(quote! { #id, });
        }
        Regex((fmt, regex)) => {
            let id = syn::parse_str::<Ident>(&format!("_{fmt}")).unwrap();
            // TODO: raise error if ty is not &'a str
            let _ty = fields[fmt.parse::<usize>().unwrap()].clone();
            body.extend(quote! {
                let (#id, source) = {
                    static REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(#regex).unwrap());
                    let Some(by) = REGEX.shortest_match(&source[..]) else {
                        Err(Error::Mismatch {
                            range: (source.split, source.split+2), 
                            token: "<str>", 
                            piece: unsafe{err_arena.alloc_str(&source[..source[..].chars().next().map(|x| x.len_utf8()).unwrap_or(0)])}
                        })?
                    };
                    let s = unsafe{ out_arena.alloc_str(&source[..by]) };
                    (s, source.proceed(by))
                };
            });
            args.extend(quote! { #id, });
        }
    } }
    // Write result into function body
    let ident = var.ident.clone();
    body.extend(quote! {Ok((Self::#ident(#args), source))});
    Ok(body)
}

/// Generate function body for named fields
fn prepare_func_body_named(var: &syn::Variant, fmt: Vec<Token>, named: &Punctuated<Field, Comma>, precedence: u16) -> Result<TokenStream> {
    use Token::*;
    let holes = fmt.iter().try_fold(HashSet::new(), |mut set, token| match token {
        Space{..} | Literal{..} => Ok(set),
        Format((fmt, _)) | Regex((fmt, _)) if set.contains(fmt) => Err(Error::new_spanned(var, format!("repeated format symbol {{{}}}", fmt))),
        Format((fmt, _)) | Regex((fmt, _)) => match fmt.parse::<usize>() {
            Err(_) => {set.insert(fmt.clone()); Ok(set)},
            Ok(_) => Err(Error::new_spanned(var, format!("the variant is named, but we get {{{}}}", fmt))),
        },
    })?;
    let fills = named.iter().map(|x| x.ident.as_ref().unwrap().to_string()).collect::<HashSet<String>>();
    difference(var, holes, fills)?;
    let fields = named.iter().map(|x| (x.ident.as_ref().unwrap().to_string(), x.ty.clone())).collect::<HashMap<_, _>>();
    // Write function body and variant arguments
    let mut body = TokenStream::new();
    let mut args = TokenStream::new();
    body.extend(quote! {if precedence >= #precedence { return Err(Error::Precedence); }});
    {
        let var = var.ident.to_string();
        body.extend(quote!{
            #[cfg(feature="trace")]
            println!("rule {name}::{} @ {}; rest {}", #var, source.split, &source[..source[..].chars().next().map(|x| x.len_utf8()).unwrap_or(0)]);
        });
    }
    for token in fmt {match token {
        Literal(literal) => 
            body.extend(quote! {let source = token(source, &err_arena, #literal)?;}),
        Space => 
            body.extend(quote! {let source = Self::space(source)?;}),
        Format((fmt, bp)) => {
            let id = syn::parse_str::<Ident>(&format!("{fmt}")).unwrap();
            let ty = fields[&fmt].clone();
            body.extend(quote! {
                let (#id, source) = <#ty>::parser_impl(source, out_arena, err_arena, #bp)?;
            });
            args.extend(quote! { #id, });
        },
        Regex((fmt, regex)) => {
            let id = syn::parse_str::<Ident>(&format!("{fmt}")).unwrap();
            // TODO: raise error if ty is not &'a str
            let _ty = fields[&fmt].clone();
            body.extend(quote! {
                let (#id, source) = {
                    use pratt_gen::*;
                    static REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(#regex).unwrap());
                    let Some(by) = REGEX.shortest_match(&source[..]) else {
                        Err(Error::Mismatch {
                            range: (source.split, source.split+2), 
                            token: #regex, 
                            piece: unsafe{err_arena.alloc_str(&source[..source[..].chars().next().map(|x| x.len_utf8()).unwrap_or(0)])}
                        })?
                    };
                    let s = unsafe{ out_arena.alloc_str(&source[..by]) };
                    (s, source.proceed(by))
                };
            });
            args.extend(quote! { #id, });
        }
    } }
    // Write result into function body
    let ident = var.ident.clone();
    body.extend(quote! {Ok((Self::#ident{#args}, source))});
    Ok(body)
}

/// Compute the difference between holes in fmt string and the fields
fn difference<T: Eq + Hash + Debug>(var: &syn::Variant, holes: HashSet<T>, fills: HashSet<T>) -> Result<()> {
    if holes == fills { return Ok(()) }
    let difference = holes.symmetric_difference(&fills);
    Err(Error::new_spanned(var, format!("{:?}", difference.collect::<HashSet<&T>>())))
}