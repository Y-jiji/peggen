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

pub(crate) struct CRATE;

impl ToTokens for CRATE {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(quote! { peggen })
    }
}

#[proc_macro_derive(Parse, attributes(rule, with))]
pub fn parse_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = bail!(parse::<DeriveInput>(input));
    let builder = bail!(Builder::new(input));
    let mut output = TokenStream::new();
    output.extend(bail!(builder.parse_impl_build()));
    output.extend(bail!(builder.rules_impl_build()));
    output.extend(bail!(builder.ast_impl_build()));
    output.extend(bail!(builder.num_build()));
    output.into()
}

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

/// Generate AstImpl trait from PushImpl trait
#[proc_macro_derive(FromStrAstImpl)]
pub fn from_str_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = bail!(parse::<DeriveInput>(input));
    let generics = &input.generics.params;
    let comma = generics.to_token_stream().into_iter().last().map(|x: TokenTree| x.to_string() == ",").unwrap_or(false);
    let generics = 
        if !comma && !generics.is_empty() { quote! { #generics, } }
        else                              { quote! { #generics  } };
    let ident = input.ident;
    quote! {
        impl<#generics Extra: Copy> AstImpl<Extra> for #ident<#generics> where 
            Self: #CRATE::FromStr<Extra>
        {
            fn peggen_ast<'a>(
                input: &'a str, 
                stack: &'a [#CRATE::Tag], 
                extra: Extra
            ) -> (&'a [#CRATE::Tag], Self) {
                let tag = stack.last().unwrap();
                (&stack[..stack.len()-1], <Self as #CRATE::FromStr<Extra>>::from_str_with(&input[tag.span.clone()], extra))
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

/// Generate SkipSpace trait the handles '\n\t '
#[proc_macro_derive(SkipSpace)]
pub fn space_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = bail!(syn::parse::<DeriveInput>(input));
    let ident = input.ident;
    let generics = input.generics.params;
    let comma = generics.to_token_stream().into_iter().last().map(|x| x.to_string() == ",").unwrap_or(false);
    let generics = 
        if !comma && !generics.is_empty() { quote! { #generics, } }
        else                              { quote! { #generics  } };
    quote! {
        impl<#generics> SkipSpace for #ident<#generics> {}
    }.into()
}