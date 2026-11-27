use crate::syntax::{Apply, Attr, Elem, Grammar, Seq};
use quote::format_ident;
use syn::Ident;

/// Turns a bare name that names a field into `Elem::Field`, numbers the regexes, and rejects with a spanned error:
/// a regex that does not compile; an `@{…}` name that is not a `#[parse]` variant of this enum in a `#[parse]` rule,
/// or not a `#[parse]` or `#[error]` variant in an `#[error]` rule; a field bound twice or never; `var =` next to an
/// end-of-rule closure. Left recursion is handled at runtime.
pub fn check(grammar: &mut Grammar) -> syn::Result<()> {
    if let Some(param) = grammar.param.type_params().next() {
        return Err(syn::Error::new(param.ident.span(), "a peggen enum takes lifetime parameters only"));
    }
    if let Some(param) = grammar.param.const_params().next() {
        return Err(syn::Error::new(param.ident.span(), "a peggen enum takes lifetime parameters only"));
    }
    if let Some(space) = &grammar.setup.space {
        regex::Regex::new(&format!("^(?:{})", space.value())).map_err(|error| syn::Error::new(space.span(), error))?;
    }
    let mut count = 0;
    for case in 0..grammar.cases.len() {
        let mut items = match &mut grammar.cases[case].attrs {
            Attr::Parse(body) | Attr::Error(_, body) => {
                std::mem::replace(&mut body.items, Seq { front: Vec::new(), after: None })
            }
            Attr::Empty => continue,
        };
        let done = resolve(grammar, case, &mut items, true, &mut count);
        if let Attr::Parse(body) | Attr::Error(_, body) = &mut grammar.cases[case].attrs {
            body.items = items;
        }
        done?;
        let entry = &grammar.cases[case];
        let (Attr::Parse(body) | Attr::Error(_, body)) = &entry.attrs else { continue };
        let bound: Vec<&Ident> = body.items.front.iter().chain(body.items.after.iter().flatten())
            .filter_map(|elem| match elem {
                Elem::Field(name, _) | Elem::Bound(name, _) => Some(name),
                _ => None,
            })
            .collect();
        if body.close.is_some() {
            if let Some(name) = bound.first() {
                return Err(syn::Error::new(name.span(), "a rule with an end-of-rule closure takes no `var` bindings"));
            }
            continue;
        }
        let fields: Vec<Ident> = entry.shape.iter().enumerate()
            .map(|(place, field)| field.ident.clone().unwrap_or_else(|| format_ident!("_{}", place)))
            .collect();
        for name in &bound {
            if !fields.contains(*name) {
                return Err(syn::Error::new(name.span(), format!("`{name}` is not a field of `{}`", entry.title)));
            }
        }
        for field in &fields {
            let hits: Vec<&&Ident> = bound.iter().filter(|name| ***name == *field).collect();
            match hits.as_slice() {
                [] => return Err(syn::Error::new(entry.title.span(), format!("field `{field}` is never bound"))),
                [_] => {}
                [_, again, ..] => return Err(syn::Error::new(again.span(), format!("field `{field}` is bound twice"))),
            }
        }
    }
    Ok(())
}

/// Resolves one sequence of case `case`: a bare name that names a field becomes `Elem::Field` with its type,
/// regexes get their numbers and must compile, `@{…}` names of this enum must be `#[parse]` variants, or also
/// `#[error]` variants when `case` is an `#[error]` variant. `top` is false inside groups, where `var = (…)` is
/// rejected.
fn resolve(grammar: &Grammar, case: usize, seq: &mut Seq, top: bool, count: &mut usize) -> syn::Result<()> {
    for elem in seq.front.iter_mut().chain(seq.after.iter_mut().flatten()) {
        if let (false, Elem::Bound(name, _)) = (top, &*elem) {
            return Err(syn::Error::new(name.span(), "`var = (…)` belongs at the top of a rule, not inside a group"));
        }
        let field = match &*elem {
            Elem::Typed(path, names) => path.get_ident().and_then(|name| {
                grammar.cases[case].shape.iter().enumerate()
                    .find(|(place, field)| field.ident.as_ref().map_or(*name == format!("_{place}"), |id| id == name))
                    .map(|(_, field)| (name.clone(), field.ty.clone(), names.is_empty()))
            }),
            _ => None,
        };
        if let Some((name, ty, plain)) = field {
            if !plain {
                return Err(syn::Error::new(name.span(), "a field takes no `@{…}`"));
            }
            *elem = Elem::Field(name, ty);
            continue;
        }
        match elem {
            Elem::Regex(text, number) => {
                regex::Regex::new(&format!("^(?:{})", text.value()))
                    .map_err(|error| syn::Error::new(text.span(), error))?;
                *number = *count;
                *count += 1;
            }
            Elem::Typed(path, names) if path.is_ident(&grammar.title) => {
                let error = matches!(grammar.cases[case].attrs, Attr::Error(..));
                for name in names.iter() {
                    let known = grammar.cases.iter().any(|entry| {
                        entry.title == *name
                            && match entry.attrs {
                                Attr::Parse(_) => true,
                                Attr::Error(..) => error,
                                Attr::Empty => false,
                            }
                    });
                    if !known {
                        let kinds = if error { "`#[parse]` or `#[error]`" } else { "`#[parse]`" };
                        let text = format!("`{name}` is not a {kinds} variant of `{}`", grammar.title);
                        return Err(syn::Error::new(name.span(), text));
                    }
                }
            }
            Elem::Bound(_, group) | Elem::Group(group) => {
                resolve(grammar, case, &mut group.inner, false, count)?;
                if let Apply::Folds(fold) = &mut group.apply {
                    if let Some(delim) = fold.delim.take() {
                        let mut single = Seq { front: vec![*delim], after: None };
                        resolve(grammar, case, &mut single, false, count)?;
                        fold.delim = single.front.pop().map(Box::new);
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
}