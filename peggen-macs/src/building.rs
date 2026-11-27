use crate::syntax::{Apply, Attr, Elem, Grammar};
use proc_macro2::{Literal, Span, TokenStream};
use quote::{format_ident, quote};
use syn::{Expr, Fields, Ident, Lifetime};

/// Phase 2: `Make` per variant and `Build`.
pub fn emit(grammar: &Grammar) -> TokenStream {
    let makes = (0..grammar.cases.len()).map(|case| make(grammar, case));
    let dispatch = dispatch(grammar);
    quote! { #(#makes)* #dispatch }
}

/// Phase 2 impl generics, type generics with the where clause, the input lifetime, and the extra type.
/// The input lifetime is the enum's first lifetime or a fresh `'input`; the extra type is `extra`'s or a fresh `Ex`.
fn header(grammar: &Grammar) -> (TokenStream, TokenStream, Lifetime, TokenStream) {
    let mut param = grammar.param.clone();
    let input = match grammar.param.lifetimes().next() {
        Some(found) => found.lifetime.clone(),
        None => {
            let fresh = Lifetime::new("'input", Span::call_site());
            param.params.insert(0, syn::GenericParam::Lifetime(syn::LifetimeParam::new(fresh.clone())));
            fresh
        }
    };
    let extra = match &grammar.setup.extra {
        Some(ty) => quote!(#ty),
        None => {
            param.params.push(syn::parse_quote!(Ex));
            quote!(Ex)
        }
    };
    let (ig, _, _) = param.split_for_impl();
    let (_, tg, wc) = grammar.param.split_for_impl();
    (quote!(#ig), quote!(#tg #wc), input, extra)
}

/// `Build::build`: takes the `Rule` tag, panics unless its `TIx` is this one, and calls `Make<…, VIX>` for its `VIx`
/// through one `match` over `vix().0`. Generated code reads through `assembler: &mut ::peggen::Assembler`.
fn dispatch(grammar: &Grammar) -> TokenStream {
    let (ig, tg, input, extra) = header(grammar);
    let title = &grammar.title;
    let arms = grammar.cases.iter().enumerate().filter_map(|(case, entry)| {
        if matches!(entry.attrs, Attr::Empty) {
            return None;
        }
        let case = Literal::usize_unsuffixed(case);
        Some(quote! { #case => <Self as ::peggen::Make<#input, #extra, #case>>::make(assembler, &mut *extra), })
    });
    let name = title.to_string();
    quote! {
        impl #ig ::peggen::Build<#input, #extra> for #title #tg {
            fn build(assembler: &mut ::peggen::Assembler<#input>, extra: &mut #extra) -> Self {
                let rule = assembler.rule();
                if rule.tix() != <Self as ::peggen::Match>::tix() {
                    ::core::panic!("peggen: {:?} is not a variant of {}", rule, #name);
                }
                match rule.vix().0 {
                    #(#arms)*
                    _ => ::core::panic!("peggen: {:?} is not a variant of {}", rule, #name),
                }
            }
        }
    }
}

/// `Make::make` for one case: reads its parts in order, then builds the variant, or the struct when `alone`,
/// from bindings or the closure.
fn make(grammar: &Grammar, case: usize) -> TokenStream {
    let entry = &grammar.cases[case];
    let body = match &entry.attrs {
        Attr::Parse(body) | Attr::Error(_, body) => body,
        Attr::Empty => return TokenStream::new(),
    };
    let (ig, tg, input, extra) = header(grammar);
    let (title, variant) = (&grammar.title, &entry.title);
    let mut names = Vec::new();
    let reads: TokenStream = body.items.front.iter().chain(body.items.after.iter().flatten())
        .map(|elem| element(grammar, elem, false, &mut names))
        .collect();
    let fields: Vec<Ident> = entry.shape.iter().enumerate()
        .map(|(place, field)| field.ident.clone().unwrap_or_else(|| format_ident!("_{}", place)))
        .collect();
    let values = body.close.as_ref().map(|close| {
        let values: Vec<TokenStream> = names.iter().map(|name| quote!(#name)).collect();
        let call = apply(grammar, &Expr::Closure(close.clone()), &values);
        match fields.as_slice() {
            [field] => quote!(let #field = #call;),
            many => quote!(let (#(#many),*) = #call;),
        }
    });
    let path = if grammar.alone { quote!(Self) } else { quote!(Self::#variant) };
    let build = match &entry.shape {
        Fields::Named(_) => quote!(#path { #(#fields),* }),
        Fields::Unnamed(_) => quote!(#path(#(#fields),*)),
        Fields::Unit => quote!(#path),
    };
    let case = Literal::usize_unsuffixed(case);
    quote! {
        impl #ig ::peggen::Make<#input, #extra, #case> for #title #tg {
            #[allow(unused_variables)]
            fn make(assembler: &mut ::peggen::Assembler<#input>, extra: &mut #extra) -> Self {
                #reads
                #values
                #build
            }
        }
    }
}

/// Statements that read one element's tags; pushes the names of the values it passes onto `names`.
/// SADDLER: SHAME(TALLFUNC)
fn element(grammar: &Grammar, elem: &Elem, group: bool, names: &mut Vec<Ident>) -> TokenStream {
    let (_, _, input, extra) = header(grammar);
    let fresh = format_ident!("_v{}", names.len());
    let value = match elem {
        Elem::Glued => return TokenStream::new(),
        Elem::Plain(_) if group => return TokenStream::new(),
        Elem::Plain(text) => quote!(#text),
        Elem::Regex(..) => quote!(assembler.text()),
        Elem::Typed(path, _) => {
            let ty = if path.is_ident(&grammar.title) { quote!(Self) } else { quote!(#path) };
            quote!(<#ty as ::peggen::Build<#input, #extra>>::build(assembler, &mut *extra))
        }
        Elem::Field(name, ty) => {
            let read = quote!(<#ty as ::peggen::Build<#input, #extra>>::build(assembler, &mut *extra));
            if !group {
                return quote! { let #name = #read; };
            }
            read
        }
        Elem::Bound(_, inner) | Elem::Group(inner) => {
            let mut local = Vec::new();
            let reads: TokenStream = inner.inner.front.iter().chain(inner.inner.after.iter().flatten())
                .map(|elem| element(grammar, elem, true, &mut local))
                .collect();
            let values: Vec<TokenStream> = local.iter().map(|name| quote!(#name)).collect();
            let tuple = match values.as_slice() {
                [one] => one.clone(),
                many => quote!((#(#many),*)),
            };
            let value = match &inner.apply {
                Apply::Plain => quote!({ #reads #tuple }),
                Apply::Close(close) => {
                    let call = apply(grammar, close, &values);
                    quote!({ #reads #call })
                }
                Apply::Maybe(close) => {
                    let call = apply(grammar, close, &[quote!(found)]);
                    quote!({
                        let found = if assembler.many() == 1 {
                            #reads
                            ::core::option::Option::Some(#tuple)
                        } else {
                            ::core::option::Option::None
                        };
                        #call
                    })
                }
                Apply::Folds(fold) => {
                    let mut dropped = Vec::new();
                    let delim = fold.delim.as_deref().map(|delim| element(grammar, delim, true, &mut dropped));
                    let (start, merge) = (&fold.start, &fold.merge);
                    let mut args = vec![quote!(&mut acc)];
                    args.extend(values.iter().cloned());
                    let call = apply(grammar, merge, &args);
                    quote!({
                        let count = assembler.many();
                        let mut acc = #start;
                        for index in 0..count {
                            if index > 0 { #delim }
                            #reads
                            #call;
                        }
                        acc
                    })
                }
            };
            if let Elem::Bound(name, _) = elem {
                return quote! { let #name = #value; };
            }
            value
        }
    };
    names.push(fresh.clone());
    quote! { let #fresh = #value; }
}

/// `callN(closure, values…)`, with `&mut *extra` last when `extra` is declared.
fn apply(grammar: &Grammar, closure: &Expr, values: &[TokenStream]) -> TokenStream {
    let mut args = values.to_vec();
    if grammar.setup.extra.is_some() {
        args.push(quote!(&mut *extra));
    }
    let call = format_ident!("call{}", args.len());
    quote!(::peggen::#call(#closure, #(#args),*))
}
