mod fmtparser;
mod builder;

use fmtparser::*;
use builder::*;
use syn::*;
use quote::{quote, ToTokens};
use proc_macro2::*;

// A simple macro that eject error variants into compile error. 
macro_rules! bail {
    ($x: expr) => {match $x {
        Ok(out)  => out,
        Err(err) => return err.into_compile_error().into()
    }};
}

pub(crate) const CRATE: &str = "pigeon";

/// Generate Parse<GROUP> trait(s) from rule attributes
#[proc_macro_derive(ParseImpl, attributes(rule))]
pub fn parse_impl_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = bail!(parse::<DeriveInput>(input));
    let builder = bail!(Builder::new(input));
    let mut output = TokenStream::new();
    output.extend(bail!(builder.parse_impl_build()));
    output.extend(bail!(builder.rules_impl_build()));
    output.into()
}

/// Generate AstImpl trait from rule attributes
#[proc_macro_derive(EnumAstImpl, attributes(rule, with))]
pub fn ast_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = bail!(parse::<DeriveInput>(input));
    let builder = bail!(Builder::new(input));
    let mut output = TokenStream::new();
    output.extend(bail!(builder.ast_impl_build()));
    output.into()
}

/// Generate AstImpl trait from Prepend trait
#[proc_macro_derive(PrependAstImpl)]
pub fn prepend_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = bail!(parse::<DeriveInput>(input));
    let _crate = parse_str::<Ident>(CRATE).unwrap();
    let generics = input.generics.params;
    let ident = input.ident;
    quote! {
        impl<#generics, Extra: Copy> AstImpl<Extra> for #ident<#generics> where
            Self: Prepend<Extra>,
            <Self as Prepend<Extra>>::Item: AstImpl<Extra>
        {
            fn ast<'lifetime>(
                input: &'lifetime str, 
                stack: &'lifetime [#_crate::Tag], 
                with: Extra
            ) -> (&'lifetime [#_crate::Tag], Self) {
                let tag = &stack[stack.len()-1];
                let mut stack = &stack[..stack.len()-1];
                let mut this = <Self as Prepend<Extra>>::empty(with);
                for i in 0..tag.rule {
                    let (stack_, value) = <<Self as Prepend<Extra>>::Item as AstImpl<Extra>>::ast(input, stack, with);
                    this.prepend(value, with);
                    stack = stack_;
                }
                (stack, this)
            }
        }
    }.into()
}

/// Generate Num trait from rule attributes
#[proc_macro_derive(Num, attributes(rule))]
pub fn num_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = bail!(parse::<DeriveInput>(input));
    let builder = bail!(Builder::new(input));
    bail!(builder.num_build()).into()
}

/// Generate Space trait the handles '\n\t '
#[proc_macro_derive(Space)]
pub fn space_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = bail!(syn::parse::<DeriveInput>(input));
    let ident = input.ident;
    let generics = input.generics.params;
    let comma = generics.to_token_stream().into_iter().last().map(|x| x.to_string() == ",").unwrap_or(false);
    let generics = 
        if !comma && !generics.is_empty() { quote! { #generics, } }
        else                              { quote! { #generics  } };
    quote! {
        impl<#generics> Space for #ident<#generics> {}
    }.into()
}