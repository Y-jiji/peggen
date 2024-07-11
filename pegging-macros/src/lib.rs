use proc_macro::TokenStream;
use syn::{self, Data, DeriveInput, Result};
mod variant;
mod fmt;
mod names;
use fmt::*;
use names::*;

#[proc_macro_derive(ParserImpl, attributes(parse))]
pub fn parser_impl_derive(input: TokenStream) -> TokenStream {
    match impl_parser(input) {
        Ok(out) => out,
        Err(err) => err.to_compile_error().into(),
    }
}

fn impl_parser(input: TokenStream) -> Result<TokenStream> {
    // Parse the input tokens into a syntax tree
    let ast = syn::parse::<DeriveInput>(input).unwrap();
    // Only allow enum
    let Data::Enum(data) = ast.data else {
        Err(syn::Error::new_spanned(&ast.ident, "#[derive(ParserImpl)] can only handle enum. "))?
    };
    // Get the enum name
    let name = &ast.ident;
    // Verify there is exactly one generic parameter
    if ast.generics.lifetimes().into_iter().count() != 1 {
        Err(syn::Error::new_spanned(&ast.ident, "#[derive(ParserImpl)] need the enum to have exactly one lifetime argument. "))?
    }
    // Implement parser_impl... for each variant arm
    let mut arm_names = Vec::new();
    let mut arm_impls = TokenStream::new();
    for arm in data.variants {
        let (name, func) = variant::impl_parser_variant(&name, arm)?;
        arm_names.push(name);
        arm_impls.extend(func);
    }
    let try_each_arm = impl_parser_try_variants(arm_names);
    let arm_impls = arm_impls.to_string();
    Err(syn::Error::new_spanned(&ast.ident, arm_impls.clone()))?;
    // Implement ParserImpl for enum
    Ok(quote::quote! {
        impl<'a> ParserImpl<'a> for #name<'a> {
            fn parser_impl(input: &'a str, p: usize, holder: Holder<'a>) -> Result<&'a Self, > {
                // Try each variant
                #try_each_arm
                Ok(out)
            }
        }
        #arm_impls
    }.into())
}

fn impl_parser_try_variants(arm_names: Vec<syn::Ident>) -> String {
    use std::fmt::Write;
    let mut body = String::new();
    for arm in arm_names {
        writeln!(&mut body, "let Ok((out, p)) = match {arm}({INPUT}, p, holder) else {{").unwrap();
        writeln!(&mut body, "    return holder.failure(p, );").unwrap();
        writeln!(&mut body, "}}").unwrap();
    }
    body
}