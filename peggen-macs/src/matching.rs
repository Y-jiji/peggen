use crate::syntax::{Apply, Attr, Elem, Grammar, Seq};
use proc_macro2::{Literal, TokenStream};
use quote::{format_ident, quote};
use syn::Ident;

/// How a step handles a part that did not return `Success`.
#[derive(Clone, Copy)]
enum Mode {
    /// Before `%` in a `#[parse]` rule: returns the state as it is.
    Front,
    /// After `%` in a `#[parse]` rule: `Mismatch` becomes `Failure`.
    After,
    /// In an `#[error]` rule: every state becomes `Mismatch`.
    Error,
}

/// Phase 1: the space regex static and `impl Match`.
pub fn emit(grammar: &Grammar) -> TokenStream {
    let space = grammar.setup.space.as_ref().map(|space| {
        let pattern = format!("^(?:{})", space.value());
        quote! {
            static PEGGEN_SPACE: ::std::sync::LazyLock<::peggen::Regex> =
                ::std::sync::LazyLock::new(|| ::peggen::Regex::new(#pattern).unwrap());
        }
    });
    let implement = implement(grammar);
    quote! { #space #implement }
}

/// The body of `variant` for one case. `#[parse]`: `sequence` with `skip` at both edges. `#[error]`: takes
/// `m.input()` at its start, runs `sequence` in `Mode::Error`, then records its message over what it matched with
/// `MessageKind::Error`.
fn arm(grammar: &Grammar, case: usize) -> TokenStream {
    match &grammar.cases[case].attrs {
        Attr::Empty => TokenStream::new(),
        Attr::Parse(body) => {
            let steps = sequence(grammar, &body.items, true, false);
            quote!({ #steps ::peggen::Failable::Success(m) })
        }
        Attr::Error(message, body) => {
            let steps = sequence(grammar, &body.items, true, true);
            quote!({
                let from = m.input();
                #steps
                let span = &from[..from.len() - m.input().len()];
                ::peggen::Failable::Success(m.message(span, ::peggen::MessageKind::Error, #message))
            })
        }
    }
}

/// One statement per element; `skip` between elements, and also before the first and after the last when `edges`,
/// unless `_` stands there. Elements before `%` take `Mode::Front` and after it `Mode::After`, or all `Mode::Error`
/// when `error`.
fn sequence(grammar: &Grammar, seq: &Seq, edges: bool, error: bool) -> TokenStream {
    let skip = grammar.setup.space.as_ref().map(|_| quote! { let m = m.skip(&PEGGEN_SPACE); });
    let (front, after) = if error { (Mode::Error, Mode::Error) } else { (Mode::Front, Mode::After) };
    let mut glued = !edges;
    let mut steps = TokenStream::new();
    let items = seq.front.iter().map(|elem| (elem, front)).chain(seq.after.iter().flatten().map(|elem| (elem, after)));
    for (elem, mode) in items {
        if let Elem::Glued = elem {
            glued = true;
            continue;
        }
        if !glued {
            steps.extend(skip.clone());
        }
        steps.extend(element(grammar, elem, mode));
        glued = false;
    }
    if edges && !glued {
        steps.extend(skip);
    }
    steps
}

/// One element as `let m = match … { Failable::Success(m) => m, … };`, returning any other state from the arm or
/// closure as `mode` says.
fn element(grammar: &Grammar, elem: &Elem, mode: Mode) -> TokenStream {
    if let Elem::Glued = elem {
        return TokenStream::new();
    }
    let step = express(grammar, elem, mode);
    let other = match mode {
        Mode::Front => quote!(other => return other,),
        Mode::After => quote! {
            ::peggen::Failable::Mismatch(m) => return ::peggen::Failable::Failure(m),
            other => return other,
        },
        Mode::Error => quote! {
            ::peggen::Failable::Mismatch(m) | ::peggen::Failable::Failure(m) | ::peggen::Failable::Fatal(m) => {
                return ::peggen::Failable::Mismatch(m)
            }
        },
    };
    quote! {
        let m = match #step {
            ::peggen::Failable::Success(m) => m,
            #other
        };
    }
}

/// The `Failable` expression of one element: `literal`; `regex` with its own static; `choice` for a bare type;
/// `variants` for a listed one, with `::peggen::VIx(n)` for this enum's names, `const { vix::<Ty>(…) }` for another
/// type's, and its own `static` `OnceLock<TGIx>` for the caller obligation of `variants`; `repeat`, or a closure call,
/// for a group, whose inner sequence keeps `mode` only when it is `Mode::Error`.
fn express(grammar: &Grammar, elem: &Elem, mode: Mode) -> TokenStream {
    let out = quote!(::peggen::Failable<'peggen>);
    let error = matches!(mode, Mode::Error);
    match elem {
        Elem::Glued => quote!(::peggen::Failable::Success(m)),
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
        Elem::Field(_, ty) => quote!(m.choice::<#ty>()),
        Elem::Typed(path, names) => {
            let own = path.is_ident(&grammar.title);
            let ty = if own { quote!(Self) } else { quote!(#path) };
            if names.is_empty() {
                return quote!(m.choice::<#ty>());
            }
            let list = names.iter().map(|name: &Ident| {
                match grammar.cases.iter().position(|entry| entry.title == *name) {
                    Some(place) if own => {
                        let place = Literal::u16_unsuffixed(place as u16);
                        quote!(::peggen::VIx(#place))
                    }
                    _ => {
                        let name = name.to_string();
                        quote!(const { ::peggen::vix::<#path>(#name) })
                    }
                }
            });
            quote!({
                static PEGGEN_GROUP: ::std::sync::OnceLock<::peggen::TGIx> = ::std::sync::OnceLock::new();
                m.variants::<#ty>(&[#(#list),*], &PEGGEN_GROUP)
            })
        }
        Elem::Bound(_, group) | Elem::Group(group) => {
            let inner = sequence(grammar, &group.inner, false, error);
            let item = quote!(|m: ::peggen::Matcher<'peggen>| -> #out { #inner ::peggen::Failable::Success(m) });
            match &group.apply {
                Apply::Plain | Apply::Close(_) => quote!((#item)(m)),
                Apply::Maybe(_) => quote!(m.repeat(0, 1, ::peggen::Failable::Success, #item)),
                Apply::Folds(fold) => {
                    let skip = grammar.setup.space.as_ref().map(|_| quote!(let m = m.skip(&PEGGEN_SPACE);));
                    let inner = if error { Mode::Error } else { Mode::Front };
                    let delim = fold.delim.as_deref().map(|delim| element(grammar, delim, inner));
                    let least = Literal::usize_unsuffixed(fold.least);
                    let sep = quote! {
                        |m: ::peggen::Matcher<'peggen>| -> #out { #skip #delim #skip ::peggen::Failable::Success(m) }
                    };
                    quote!(m.repeat(#least, ::core::primitive::usize::MAX, #sep, #item))
                }
            }
        }
    }
}

/// `impl Match`: `VARIANTS`, `PARSE`, `tix` (a `LazyLock` of `TIx::next`), `space`, `errors` (one `match` over the
/// `#[parse]` cases), and `variant`: one `match` over `vix.0` and `first`, where `None` runs the case's `arm`, and
/// `Some(seed)` runs the case without its first element for the cases `left` accepts, guarded by the seeds that element
/// takes; any other `Some` is `Mismatch`.
/// SADDLER: SHAME(TALLFUNC)
fn implement(grammar: &Grammar) -> TokenStream {
    let title = &grammar.title;
    let name = title.to_string();
    let (ig, tg, wc) = grammar.param.split_for_impl();
    let number = |case: usize| Literal::u16_unsuffixed(case as u16);
    let names = grammar.cases.iter().map(|case| case.title.to_string());
    let parse: Vec<usize> = (0..grammar.cases.len())
        .filter(|&case| matches!(grammar.cases[case].attrs, Attr::Parse(_)))
        .collect();
    let every = parse.iter().map(|&case| number(case));
    let errors = parse.iter().filter_map(|&case| {
        let fixes: Vec<Literal> = (case + 1..grammar.cases.len())
            .take_while(|&next| !matches!(grammar.cases[next].attrs, Attr::Parse(_)))
            .filter(|&next| matches!(grammar.cases[next].attrs, Attr::Error(..)))
            .map(number)
            .collect();
        if fixes.is_empty() {
            return None;
        }
        let case = number(case);
        Some(quote!(#case => &[#(::peggen::VIx(#fixes)),*],))
    });
    let space = match grammar.setup.space {
        Some(_) => quote!(::core::option::Option::Some(&*PEGGEN_SPACE)),
        None => quote!(::core::option::Option::None),
    };
    let grows = parse.iter().filter_map(|&case| {
        let Attr::Parse(body) = &grammar.cases[case].attrs else { return None };
        let (heads, tail) = left(grammar, &body.items)?;
        if heads.is_empty() {
            return None;
        }
        let steps = sequence(grammar, &tail, true, false);
        let heads = heads.into_iter().map(number);
        let case = number(case);
        Some(quote! {
            (#case, ::core::option::Option::Some(seed)) if matches!(seed.0, #(#heads)|*) => {
                #steps
                ::peggen::Failable::Success(m)
            }
        })
    });
    let arms = (0..grammar.cases.len()).filter(|&case| !matches!(grammar.cases[case].attrs, Attr::Empty)).map(|case| {
        let body = arm(grammar, case);
        let case = number(case);
        quote!((#case, ::core::option::Option::None) => #body,)
    });
    quote! {
        impl #ig ::peggen::Match for #title #tg #wc {
            const VARIANTS: &'static [&'static str] = &[#(#names),*];
            const PARSE: &'static [::peggen::VIx] = &[#(::peggen::VIx(#every)),*];
            fn tix() -> ::peggen::TIx {
                static PEGGEN_TIX: ::std::sync::LazyLock<::peggen::TIx> =
                    ::std::sync::LazyLock::new(::peggen::TIx::next);
                *PEGGEN_TIX
            }
            fn space() -> ::core::option::Option<&'static ::peggen::Regex> {
                #space
            }
            fn errors(vix: ::peggen::VIx) -> &'static [::peggen::VIx] {
                match vix.0 {
                    #(#errors)*
                    _ => &[],
                }
            }
            fn variant<'peggen>(
                m: ::peggen::Matcher<'peggen>,
                vix: ::peggen::VIx,
                first: ::core::option::Option<::peggen::VIx>,
            ) -> ::peggen::Failable<'peggen> {
                match (vix.0, first) {
                    #(#grows)*
                    (_, ::core::option::Option::Some(_)) => ::peggen::Failable::Mismatch(m),
                    #(#arms)*
                    _ => ::core::panic!("peggen: {:?} of {} has no `#[parse]` or `#[error]`", vix, #name),
                }
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
