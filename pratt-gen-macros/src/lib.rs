mod format;
mod parse_impl;
use proc_macro::*;
use quote::quote;
use syn::DeriveInput;

macro_rules! bail {
    ($x: expr) => {
        match $x {
            Ok(out)  => out,
            Err(err) => return err.into_compile_error().into()
        }
    };
}

#[proc_macro_derive(ParseImpl, attributes(parse))]
pub fn parse_derive(input: TokenStream) -> TokenStream {
    match parse_impl::parse_derive_impl(input.into()) {
        Ok(out) => out.into(),
        Err(err) => err.into_compile_error().into(),
    }
}

#[proc_macro_derive(Space)]
pub fn space_derive(input: TokenStream) -> TokenStream {
    todo!()
}

#[proc_macro_derive(ErrorImpl, attributes(error, parse))]
pub fn error_derive(input: TokenStream) -> TokenStream {
    todo!()
}

#[proc_macro_derive(Map)] 
pub fn map_derive(input: TokenStream) -> TokenStream {
    let input = bail!(syn::parse::<DeriveInput>(input));
    let ident = input.ident;
    quote! {
        impl<'a> Map<'a, Self> for #ident<'a> {
            fn map(
                _: &'a str, 
                _: usize,
                _: &'a Arena,
                value: Self,
                _: usize,
            ) -> Self {
                value
            }
        }        
    }.into()
}