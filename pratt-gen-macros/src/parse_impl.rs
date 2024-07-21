use std::collections::HashMap;

use crate::format::*;
use syn::*;
use proc_macro2::*;

pub fn parse_derive_impl(input: TokenStream) -> Result<TokenStream> {
    let input = parse2::<DeriveInput>(input).unwrap();
    match input.data {
        Data::Struct(r#struct) => parse_derive_impl_struct(r#struct),
        Data::Enum(r#enum) => parse_derive_impl_enum(r#enum),
        Data::Union(_) => Err(Error::new_spanned(input, "expect derive(ParseImpl) to work on enum or struct, but we get union. ")),
    }
}

pub fn parse_derive_impl_struct(r#struct: DataStruct) -> Result<TokenStream> {
    todo!()
}

pub fn parse_derive_impl_enum(r#enum: DataEnum) -> Result<TokenStream> {
    // Only handle variants with `parse` marks
    let variants = r#enum.variants.into_iter()
        .filter(|var| var.attrs.iter().filter(|x| x.path().is_ident("parse")).next().is_some())
        .collect::<Vec<_>>();
    // Parse meta
    
    todo!()
}

pub fn build_rule(
    name: Ident,
    format: Vec<Format>,
    fields: HashMap<String, Type>,
) -> Result<TokenStream> {
    for f in format {
        
    }
    todo!()
}