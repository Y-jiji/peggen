mod format;
mod builder;


use format::*;
use builder::*;
use syn::*;
use quote::quote;
use proc_macro2::*;

macro_rules! bail {
    ($x: expr) => {
        match $x {
            Ok(out)  => out,
            Err(err) => return err.into_compile_error().into()
        }
    };
}

#[proc_macro_derive(ParseImpl, attributes(rule))]
pub fn parse_impl_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = bail!(parse::<DeriveInput>(input));
    let builder = bail!(Builder::new(input));
    let mut output = TokenStream::new();
    output.extend(bail!(builder.parse_impl_build()));
    output.extend(bail!(builder.rules_impl_build()));
    output.into()
}

#[proc_macro_derive(Ast, attributes(rule))]
pub fn ast_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = bail!(parse::<DeriveInput>(input));
    let builder = bail!(Builder::new(input));
    let mut output = TokenStream::new();
    output.extend(bail!(builder.ast_build()));
    output.into()
}

#[proc_macro_derive(Num, attributes(rule))]
pub fn num_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = bail!(parse::<DeriveInput>(input));
    let builder = bail!(Builder::new(input));
    bail!(builder.num_build()).into()
}

#[proc_macro_derive(Space)]
pub fn space_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = bail!(syn::parse::<DeriveInput>(input));
    let ident = input.ident;
    quote! {
        impl Space for #ident {}    
    }.into()
}