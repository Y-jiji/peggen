use proc_macro2::TokenStream;
use quote::quote;
use syn::{self, Data, DeriveInput, Result};
mod variant;
mod fmt;
use fmt::*;

#[proc_macro_derive(Space)]
pub fn space_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // Parse the input tokens into a syntax tree
    let ast = syn::parse::<DeriveInput>(input).unwrap();
    // Get the enum name
    let name = &ast.ident;
    quote! {impl<'a> Space<'a> for #name<'a> {}}.into()
}

#[proc_macro_derive(ParserImpl, attributes(parse))]
pub fn parser_impl_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    match impl_parser(input.into()) {
        Ok(out) => out.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn impl_parser(input: TokenStream) -> Result<TokenStream> {
    // Parse the input tokens into a syntax tree
    let ast = syn::parse2::<DeriveInput>(input).unwrap();
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
    let body = impl_parser_body(arm_names);
    // Implement ParserImpl for enum
    Ok(quote! {
        impl<'a> ParserImpl<'a> for #name<'a> {
            fn parser_impl(
                source: Source<'a>, 
                arena: &'a Arena,
                precedence: u16,
            ) -> Result<(Self, Source<'a>), Error<'a>> {
                #body
            }
        }
        #arm_impls
    }.into())
}

fn impl_parser_body(arm_names: Vec<syn::Ident>) -> TokenStream {
    let mut body = TokenStream::new();
    body.extend(quote! {
        let out_len = out_arena.size();
        let chain = List::new();
    });
    for arm in arm_names {body.extend(quote! {
        let chain = match Self::#arm(source, out_arena, err_arena, precedence) {
            Ok(out) => unsafe {err_arena.shrink_to(err_len); return Ok(out)},
            Err(e) => unsafe {out_arena.shrink_to(out_len); chain.push(&err_arena, e)},
        };
    })}
    body.extend(quote! {
        Err(Error::List(unsafe { err_arena.alloc(chain) }))
    });
    body
}