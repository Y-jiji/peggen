mod format;
mod parse_impl;
use proc_macro::*;

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