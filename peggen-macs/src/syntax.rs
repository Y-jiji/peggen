use syn::parse::{Parse, ParseStream};
use syn::{Attribute, Expr, ExprClosure, Fields, Generics, Ident, Item, LitStr, Path, Token, Type};

/// `#[peggen(…)]` arguments.
pub struct Setup {
    /// `space = r"…"`: the regex skipped before, between and after the elements of each rule.
    pub space: Option<LitStr>,
    /// `extra = T`: closures get `&mut T` as their last argument.
    pub extra: Option<Type>,
}

/// The annotated enum or struct.
pub struct Grammar {
    pub title: Ident,
    pub param: Generics,
    pub setup: Setup,
    pub cases: Vec<Case>,
    /// True for a struct: its one case is the whole type and is built as `Self`.
    pub alone: bool,
}

/// One variant.
pub struct Case {
    pub title: Ident,
    pub shape: Fields,
    pub attrs: Attr,
}

/// A variant's peggen attribute.
pub enum Attr {
    /// Neither attribute: never matched.
    Empty,
    /// `#[parse(…)]`.
    Parse(Body),
    /// `#[error("message" : …)]`.
    Error(LitStr, Body),
}

/// A variant's pattern and its optional end-of-rule closure.
pub struct Body {
    pub items: Seq,
    pub close: Option<ExprClosure>,
}

/// A sequence; `%` splits it into the part before and the part after.
pub struct Seq {
    pub front: Vec<Elem>,
    pub after: Option<Vec<Elem>>,
}

/// One element of a sequence.
pub enum Elem {
    /// `"abc"`: plain text.
    Plain(LitStr),
    /// `r"abc"`: a regex and the index of its static.
    Regex(LitStr, usize),
    /// `_`: no space here.
    Glued,
    /// `Type` or `Type@{V1, V2}`; an empty list means every `#[parse]` variant.
    Typed(Path, Vec<Ident>),
    /// `var` and its field's type: parsed as that type; stored in `var` at the top of a rule, passed on inside a group.
    Field(Ident, Type),
    /// `var = (…)`: the group's value, stored in `var`.
    Bound(Ident, Group),
    /// `(…)` without `var =`: its value goes to the enclosing closure.
    Group(Group),
}

/// A parenthesized group and what is done with its values.
pub struct Group {
    pub inner: Seq,
    pub apply: Apply,
}

/// What a group does with its values.
pub enum Apply {
    /// `(exp)`: passes them on.
    Plain,
    /// `(exp ; closure)`: calls `closure` on them.
    Close(Expr),
    /// `(exp ; closure)?`: calls `closure` on `Some(values)` or `None`.
    Maybe(Expr),
    /// `(exp ; init ; fold) sep *` or `+`.
    Folds(Fold),
}

/// `(exp ; init ; fold) sep *` or `+`.
pub struct Fold {
    pub start: Expr,
    pub merge: Expr,
    pub delim: Option<Box<Elem>>,
    pub least: usize,
}

/// Reads the `#[peggen]` arguments and the enum or struct, and removes `#[parse]` / `#[error]` from it.
/// A struct takes exactly one `#[parse(…)]`, which becomes its one case, named after the struct.
pub fn grammar(attr: proc_macro2::TokenStream, item: &mut Item) -> syn::Result<Grammar> {
    let ours = |attrs: &mut Vec<Attribute>| -> Vec<Attribute> {
        let (ours, rest): (Vec<_>, Vec<_>) = attrs.drain(..)
            .partition(|attr| attr.path().is_ident("parse") || attr.path().is_ident("error"));
        *attrs = rest;
        ours
    };
    let (title, param, alone, found) = match item {
        Item::Enum(item) => {
            let found: Vec<_> = item.variants.iter_mut()
                .map(|variant| (variant.ident.clone(), variant.fields.clone(), ours(&mut variant.attrs)))
                .collect();
            (item.ident.clone(), item.generics.clone(), false, found)
        }
        Item::Struct(item) => {
            let found = vec![(item.ident.clone(), item.fields.clone(), ours(&mut item.attrs))];
            (item.ident.clone(), item.generics.clone(), true, found)
        }
        other => return Err(syn::Error::new_spanned(other, "`#[peggen]` goes on an enum or a struct")),
    };
    let setup: Setup = syn::parse2(attr)?;
    let mut cases = Vec::new();
    for (name, shape, ours) in found {
        let mut attrs = Attr::Empty;
        for attr in ours {
            if !matches!(attrs, Attr::Empty) {
                return Err(syn::Error::new_spanned(attr, "a variant takes one `#[parse]` or `#[error]`"));
            }
            attrs = if attr.path().is_ident("parse") {
                Attr::Parse(attr.parse_args()?)
            } else {
                attr.parse_args_with(|input: ParseStream| {
                    let message: LitStr = input.parse()?;
                    input.parse::<Token![:]>()?;
                    Ok(Attr::Error(message, input.parse()?))
                })?
            };
        }
        if alone && !matches!(attrs, Attr::Parse(_)) {
            return Err(syn::Error::new(name.span(), "a peggen struct takes exactly one `#[parse(…)]`"));
        }
        cases.push(Case { title: name, shape, attrs });
    }
    Ok(Grammar { title, param, setup, cases, alone })
}

impl Parse for Setup {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut setup = Setup { space: None, extra: None };
        while !input.is_empty() {
            let key: Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            if key == "space" {
                setup.space = Some(input.parse()?);
            } else if key == "extra" {
                setup.extra = Some(input.parse()?);
            } else {
                return Err(syn::Error::new(key.span(), "expected `space` or `extra`"));
            }
            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }
        Ok(setup)
    }
}

impl Parse for Body {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let items: Seq = input.parse()?;
        let close = if input.is_empty() { None } else { Some(input.parse::<ExprClosure>()?) };
        if !input.is_empty() {
            return Err(input.error("expected the end of the rule"));
        }
        Ok(Body { items, close })
    }
}

impl Parse for Seq {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut seq = Seq { front: Vec::new(), after: None };
        while !input.is_empty() && !input.peek(Token![;]) && !input.peek(Token![|]) && !input.peek(Token![||]) {
            if input.peek(Token![%]) {
                let split = input.parse::<Token![%]>()?;
                if seq.after.is_some() {
                    return Err(syn::Error::new_spanned(split, "a sequence takes one `%`"));
                }
                seq.after = Some(Vec::new());
                continue;
            }
            let elem: Elem = input.parse()?;
            match &mut seq.after {
                Some(after) => after.push(elem),
                None => seq.front.push(elem),
            }
        }
        Ok(seq)
    }
}

impl Parse for Elem {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(LitStr) {
            let text: LitStr = input.parse()?;
            let raw = text.token().to_string().starts_with('r');
            return Ok(if raw { Elem::Regex(text, 0) } else { Elem::Plain(text) });
        }
        if input.peek(Token![_]) {
            input.parse::<Token![_]>()?;
            return Ok(Elem::Glued);
        }
        if input.peek(syn::token::Paren) {
            return Ok(Elem::Group(group(input)?));
        }
        if input.peek(Ident) && input.peek2(Token![=]) {
            let name: Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            if !input.peek(syn::token::Paren) {
                return Err(input.error("expected `(` after `var =`"));
            }
            return Ok(Elem::Bound(name, group(input)?));
        }
        if !input.peek(Ident) && !input.peek(Token![::]) {
            return Err(input.error("expected an element: \"…\", r\"…\", `_`, a type, a field, or `(…)`"));
        }
        let path = Path::parse_mod_style(input)?;
        let mut names = Vec::new();
        if input.peek(Token![@]) {
            input.parse::<Token![@]>()?;
            let content;
            syn::braced!(content in input);
            names = content.parse_terminated(Ident::parse, Token![,])?.into_iter().collect();
        }
        Ok(Elem::Typed(path, names))
    }
}

/// `(…)` and its suffix: `?`, `sep *`, `sep +`, or nothing.
fn group(input: ParseStream) -> syn::Result<Group> {
    let content;
    syn::parenthesized!(content in input);
    let inner: Seq = content.parse()?;
    let mut exprs = Vec::new();
    while content.peek(Token![;]) {
        content.parse::<Token![;]>()?;
        exprs.push(content.parse::<Expr>()?);
    }
    if !content.is_empty() {
        return Err(content.error("expected `;` or `)`"));
    }
    let mut exprs = exprs.into_iter();
    let apply = match (exprs.next(), exprs.next(), exprs.next()) {
        (None, _, _) => Apply::Plain,
        (Some(close), None, _) if input.peek(Token![?]) => {
            input.parse::<Token![?]>()?;
            Apply::Maybe(close)
        }
        (Some(close), None, _) => Apply::Close(close),
        (Some(start), Some(merge), None) => {
            let bare = input.peek(Token![*]) || input.peek(Token![+]);
            let delim = if bare { None } else { Some(Box::new(input.parse::<Elem>()?)) };
            let least = if input.peek(Token![+]) {
                input.parse::<Token![+]>()?;
                1
            } else {
                input.parse::<Token![*]>()?;
                0
            };
            Apply::Folds(Fold { start, merge, delim, least })
        }
        _ => return Err(input.error("a group takes at most two `;`")),
    };
    Ok(Group { inner, apply })
}
