use crate::syntax::{Apply, Attr, Elem, Grammar, Seq};
use proc_macro2::{Literal, TokenStream};
use quote::{format_ident, quote};
use syn::Ident;

/// Phase 1: the regex statics, `Match` / `Fault` per variant, `Choice` and `Rescue`.
pub fn emit(grammar: &Grammar) -> TokenStream {
    let space = grammar.setup.space.as_ref().map(|space| {
        let pattern = format!("^(?:{})", space.value());
        quote! {
            static PEGGEN_SPACE: ::std::sync::LazyLock<::peggen::Regex> =
                ::std::sync::LazyLock::new(|| ::peggen::Regex::new(#pattern).unwrap());
        }
    });
    let rules = (0..grammar.cases.len()).map(|case| rules(grammar, case));
    let choice = choice(grammar);
    quote! { #space #(#rules)* #choice }
}

/// `rule` and `patt` for one case; `skip` surrounds its sequence unless `_` stands at an edge,
/// and `patt` ends with `.map_err(|fail| fail.back(snap))`.
fn rules(grammar: &Grammar, case: usize) -> TokenStream {
    let (kind, note, body) = match &grammar.cases[case].attrs {
        Attr::Empty => return TokenStream::new(),
        Attr::Parse(body) => (quote!(Match), None, body),
        Attr::Error(message, body) => (quote!(Fault), Some(quote!(let m = m.note(#message);)), body),
    };
    let title = &grammar.title;
    let (ig, tg, wc) = grammar.param.split_for_impl();
    let index = Literal::usize_unsuffixed(case);
    let steps = sequence(grammar, &body.items, true);
    quote! {
        impl #ig ::peggen::#kind<#index> for #title #tg #wc {
            fn rule() -> ::peggen::Rule {
                static RULE: ::std::sync::LazyLock<::peggen::Rule> = ::std::sync::LazyLock::new(::peggen::Rule::next);
                *RULE
            }
            fn patt<'peggen>(
                m: ::peggen::Matcher<'peggen>,
                snap: ::peggen::Snapshot<'peggen>,
            ) -> ::core::result::Result<::peggen::Matcher<'peggen>, ::peggen::Fail<'peggen>> {
                #note
                let seq = |m: ::peggen::Matcher<'peggen>|
                    -> ::core::result::Result<::peggen::Matcher<'peggen>, ::peggen::Fail<'peggen>> {
                    #steps
                    ::core::result::Result::Ok(m)
                };
                seq(m).map_err(|fail| fail.back(snap))
            }
        }
    }
}

/// One statement per element; `skip` between elements, and also before the first and after the last when `edges`,
/// unless `_` stands there; elements after `%` add `.map_err(Fail::rest)`.
fn sequence(grammar: &Grammar, seq: &Seq, edges: bool) -> TokenStream {
    let skip = grammar.setup.space.as_ref().map(|_| quote! { let m = m.skip(&PEGGEN_SPACE); });
    let mut glued = !edges;
    let mut steps = TokenStream::new();
    let items = seq.front.iter().map(|elem| (elem, false)).chain(seq.after.iter().flatten().map(|elem| (elem, true)));
    for (elem, after) in items {
        if let Elem::Glued = elem {
            glued = true;
            continue;
        }
        if !glued {
            steps.extend(skip.clone());
        }
        steps.extend(element(grammar, elem, after));
        glued = false;
    }
    if edges && !glued {
        steps.extend(skip);
    }
    steps
}

/// One element as one `let m = …;` statement; `after` picks `expect` and `repair` for types.
/// SADDLER: SHAME(TALLFUNC)
fn element(grammar: &Grammar, elem: &Elem, after: bool) -> TokenStream {
    let out = quote!(::core::result::Result<::peggen::Matcher<'peggen>, ::peggen::Fail<'peggen>>);
    let step = match elem {
        Elem::Glued => return TokenStream::new(),
        Elem::Plain(text) => quote!(m.literal(#text)),
        Elem::Regex(text, number) => {
            let name = format_ident!("PEGGEN_REGEX_{}", number);
            let pattern = format!("^(?:{})", text.value());
            quote!({
                static #name: ::std::sync::LazyLock<::peggen::Regex> =
                    ::std::sync::LazyLock::new(|| ::peggen::Regex::new(#pattern).unwrap());
                m.regex(&#name)
            })
        }
        Elem::Field(_, ty) if after => quote!(::peggen::expect::<#ty>(m)),
        Elem::Field(_, ty) => quote!(<#ty as ::peggen::Choice>::choice(m)),
        Elem::Typed(path, names) => {
            let own = path.is_ident(&grammar.title);
            let ty = if own { quote!(Self) } else { quote!(#path) };
            let index = |name: &Ident| -> TokenStream {
                match grammar.cases.iter().position(|entry| entry.title == *name) {
                    Some(place) if own => {
                        let place = Literal::usize_unsuffixed(place);
                        quote!(#place)
                    }
                    _ => {
                        let name = name.to_string();
                        quote!({ ::peggen::index::<#ty>(#name) })
                    }
                }
            };
            match names.split_first() {
                None if after => quote!(::peggen::expect::<#ty>(m)),
                None => quote!(<#ty as ::peggen::Choice>::choice(m)),
                Some((head, tail)) => {
                    let all = names.iter().map(&index);
                    let head = index(head);
                    let tries = tail.iter().map(&index).map(|i| quote! {
                        .or_else(|fail| fail.or(::peggen::attempt::<#ty, #i>))
                    });
                    let fixes = names.iter().filter(|_| after).map(&index).map(|i| quote! {
                        .or_else(|fail| fail.or(|m| {
                            <#ty as ::peggen::Rescue>::repair(m, <#ty as ::peggen::Match<#i>>::rule())
                        }))
                    });
                    quote! {
                        ::peggen::enter::<#ty>(m, &[#(#all),*], |m: ::peggen::Matcher<'peggen>| -> #out {
                            ::peggen::attempt::<#ty, #head>(m) #(#tries)*
                        }) #(#fixes)*
                    }
                }
            }
        }
        Elem::Bound(_, group) | Elem::Group(group) => {
            let inner = sequence(grammar, &group.inner, false);
            let item = quote!(|m: ::peggen::Matcher<'peggen>| -> #out { #inner ::core::result::Result::Ok(m) });
            match &group.apply {
                Apply::Plain | Apply::Close(_) => quote!((#item)(m)),
                Apply::Maybe(_) => quote!(m.repeat(0, 1, ::core::result::Result::Ok, #item)),
                Apply::Folds(fold) => {
                    let skip = grammar.setup.space.as_ref().map(|_| quote!(let m = m.skip(&PEGGEN_SPACE);));
                    let delim = fold.delim.as_deref().map(|delim| element(grammar, delim, false));
                    let least = Literal::usize_unsuffixed(fold.least);
                    let sep = quote! {
                        |m: ::peggen::Matcher<'peggen>| -> #out { #skip #delim #skip ::core::result::Result::Ok(m) }
                    };
                    quote!(m.repeat(#least, ::core::primitive::usize::MAX, #sep, #item))
                }
            }
        }
    };
    let rest = after.then(|| quote!(.map_err(::peggen::Fail::rest)));
    quote! { let m = #step #rest?; }
}

/// `Choice::key`, `Choice::space`, `Choice::choice` through `enter`, `Choice::round`, `Rescue::repair` and
/// `Rescue::rescue`.
/// SADDLER: SHAME(TALLFUNC)
fn choice(grammar: &Grammar) -> TokenStream {
    let title = &grammar.title;
    let (ig, tg, wc) = grammar.param.split_for_impl();
    let out = quote!(::core::result::Result<::peggen::Matcher<'peggen>, ::peggen::Fail<'peggen>>);
    let names = grammar.cases.iter().map(|case| case.title.to_string());
    let kinds = |error: bool| -> Vec<usize> {
        (0..grammar.cases.len()).filter(|&case| match grammar.cases[case].attrs {
            Attr::Parse(_) => !error,
            Attr::Error(..) => error,
            Attr::Empty => false,
        }).collect()
    };
    let chain = |call: TokenStream, list: &[usize]| -> TokenStream {
        match list.split_first() {
            None => quote!(::core::result::Result::Err(::peggen::Fail::Miss(m))),
            Some((head, tail)) => {
                let head = Literal::usize_unsuffixed(*head);
                let tail = tail.iter().map(|case| Literal::usize_unsuffixed(*case));
                quote!(::peggen::#call::<Self, #head>(m) #(.or_else(|fail| fail.or(::peggen::#call::<Self, #tail>)))*)
            }
        }
    };
    let choose = chain(quote!(attempt), &kinds(false));
    let rescue = chain(quote!(fault), &kinds(true));
    let repairs = kinds(false).into_iter().map(|case| {
        let group: Vec<usize> = (case + 1..grammar.cases.len())
            .take_while(|&next| !matches!(grammar.cases[next].attrs, Attr::Parse(_)))
            .filter(|&next| matches!(grammar.cases[next].attrs, Attr::Error(..)))
            .collect();
        let body = chain(quote!(fault), &group);
        let case = Literal::usize_unsuffixed(case);
        quote!(if rule == <Self as ::peggen::Match<#case>>::rule() { return #body; })
    });
    let space = match grammar.setup.space {
        Some(_) => quote!(::core::option::Option::Some(&*PEGGEN_SPACE)),
        None => quote!(::core::option::Option::None),
    };
    let parse = kinds(false).into_iter().map(Literal::usize_unsuffixed);
    let arms: Vec<TokenStream> = kinds(false).into_iter().filter_map(|case| {
        let Attr::Parse(body) = &grammar.cases[case].attrs else { return None };
        let (heads, tail) = left(grammar, &body.items)?;
        let steps = sequence(grammar, &tail, true);
        let heads = heads.into_iter().map(Literal::usize_unsuffixed);
        let case = Literal::usize_unsuffixed(case);
        Some(quote! {
            #case if false #(|| seed == <Self as ::peggen::Match<#heads>>::rule())* => ::peggen::grow::<Self, #case>(
                m, call, |m: ::peggen::Matcher<'peggen>| -> #out { #steps ::core::result::Result::Ok(m) },
            ),
        })
    }).collect();
    let round = if arms.is_empty() {
        quote!(::core::result::Result::Err(::peggen::Fail::Miss(m)))
    } else {
        quote! {
            let mut m = m;
            for &case in set {
                let step = match case {
                    #(#arms)*
                    _ => continue,
                };
                match step {
                    ::core::result::Result::Err(::peggen::Fail::Miss(back)) => m = back,
                    other => return other,
                }
            }
            ::core::result::Result::Err(::peggen::Fail::Miss(m))
        }
    };
    quote! {
        impl #ig ::peggen::Choice for #title #tg #wc {
            const VARIANTS: &'static [&'static str] = &[#(#names),*];
            fn key() -> ::peggen::Key {
                static KEY: ::std::sync::LazyLock<::peggen::Key> = ::std::sync::LazyLock::new(::peggen::Key::next);
                *KEY
            }
            fn space() -> ::core::option::Option<&'static ::peggen::Regex> {
                #space
            }
            fn choice<'peggen>(m: ::peggen::Matcher<'peggen>) -> #out {
                ::peggen::enter::<Self>(m, &[#(#parse),*], |m: ::peggen::Matcher<'peggen>| -> #out { #choose })
            }
            #[allow(unused_variables)]
            fn round<'peggen>(
                m: ::peggen::Matcher<'peggen>,
                call: ::peggen::Snapshot<'peggen>,
                seed: ::peggen::Rule,
                set: &[usize],
            ) -> #out {
                #round
            }
        }
        impl #ig ::peggen::Rescue for #title #tg #wc {
            #[allow(unused_variables)]
            fn repair<'peggen>(m: ::peggen::Matcher<'peggen>, rule: ::peggen::Rule) -> #out {
                #(#repairs)*
                ::core::result::Result::Err(::peggen::Fail::Miss(m))
            }
            fn rescue<'peggen>(m: ::peggen::Matcher<'peggen>) -> #out {
                #rescue
            }
        }
    }
}

/// If `seq` starts with this enum, directly or inside a plain or closure group: the variants that first element
/// accepts, and `seq` without it.
fn left(grammar: &Grammar, seq: &Seq) -> Option<(Vec<usize>, Seq)> {
    let mut rest = seq.clone();
    let list = if rest.front.iter().any(|elem| !matches!(elem, Elem::Glued)) {
        &mut rest.front
    } else {
        rest.after.as_mut()?
    };
    let at = list.iter().position(|elem| !matches!(elem, Elem::Glued))?;
    let (heads, drop) = match &mut list[at] {
        Elem::Typed(path, names) if path.is_ident(&grammar.title) => {
            let heads: Vec<usize> = if names.is_empty() {
                (0..grammar.cases.len()).filter(|&case| matches!(grammar.cases[case].attrs, Attr::Parse(_))).collect()
            } else {
                names.iter().filter_map(|name| grammar.cases.iter().position(|case| case.title == *name)).collect()
            };
            (heads, true)
        }
        Elem::Bound(_, group) | Elem::Group(group) if matches!(group.apply, Apply::Plain | Apply::Close(_)) => {
            let (heads, inner) = left(grammar, &group.inner)?;
            group.inner = inner;
            (heads, false)
        }
        _ => return None,
    };
    if drop {
        list.remove(at);
    }
    Some((heads, rest))
}
