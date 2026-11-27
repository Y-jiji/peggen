mod syntax;
mod check;
mod matching;
mod building;

use proc_macro::TokenStream;

/// `#[peggen(space = r"…", extra = T)]` on an enum whose variants carry `#[parse(…)]` or `#[error("…" : …)]`,
/// or on a struct that carries one `#[parse(…)]` after `#[peggen]`.
/// Emits the item without those attributes, then `Match`, `Make` per variant and `Build`,
/// inside one `const _: () = { … };` block.
#[proc_macro_attribute]
pub fn peggen(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut item = syn::parse_macro_input!(item as syn::Item);
    let output = syntax::grammar(attr.into(), &mut item).and_then(|mut grammar| {
        check::check(&mut grammar)?;
        Ok((matching::emit(&grammar), building::emit(&grammar)))
    });
    match output {
        Ok((phase1, phase2)) => quote::quote! { #item const _: () = { #phase1 #phase2 }; }.into(),
        Err(error) => {
            let error = error.to_compile_error();
            quote::quote! { #item #error }.into()
        }
    }
}
