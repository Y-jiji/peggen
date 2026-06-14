use crate::*;
use crate::rule_ast::*;

pub trait AstImplBuild {
    fn ast_impl_build(&self) -> Result<TokenStream>;
}

enum FieldKind {
    Value,
    Collection,
}

fn normalize(arg: &str) -> Result<Ident> {
    let dig = arg.chars().all(|arg| arg.is_digit(10));
    let arg = if dig { format!("_{arg}") } else { arg.to_string() };
    parse_str::<Ident>(&arg)
}

fn collect_fields_from_expr(
    expr: &RuleExpr,
    inside_rep: bool,
    variant_fields: &std::collections::HashMap<String, syn::Type>,
) -> Vec<(String, FieldKind)> {
    match expr {
        RuleExpr::Field(f) | RuleExpr::FieldRegex(f, _) | RuleExpr::FieldTag(f, _) => {
            if inside_rep {
                vec![]
            } else {
                vec![(f.key(), FieldKind::Value)]
            }
        }
        RuleExpr::FieldMulti(frefs, _) => {
            if inside_rep {
                vec![]
            } else {
                frefs.iter().map(|f| (f.key(), FieldKind::Value)).collect()
            }
        }
        RuleExpr::SubruleRef(_) | RuleExpr::Literal(_) => vec![],
        RuleExpr::Seq(elems) => {
            let mut out = vec![];
            for e in elems {
                out.extend(collect_fields_from_expr(e, inside_rep, variant_fields));
            }
            out
        }
        RuleExpr::Choice(a, b) => {
            let mut out = collect_fields_from_expr(a, inside_rep, variant_fields);
            out.extend(collect_fields_from_expr(b, inside_rep, variant_fields));
            out
        }
        RuleExpr::Rep(e, _) | RuleExpr::SepRep { expr: e, .. } => {
            if e.has_field_refs() {
                let resolved = crate::builder::resolve_rep_fields(e, variant_fields);
                if let Some((key, _)) = resolved {
                    vec![(key, FieldKind::Collection)]
                } else {
                    collect_fields_from_expr(e, true, variant_fields)
                }
            } else {
                vec![]
            }
        }
        RuleExpr::Not(_) | RuleExpr::And(_) => vec![],
    }
}

impl AstImplBuild for Builder {
    fn ast_impl_build(&self) -> Result<TokenStream> {
        let this = &self.ident;
        let generics = &self.generics;
        let comma = generics.to_token_stream().into_iter().last().map(|x: TokenTree| x.to_string() == ",").unwrap_or(false);
        let generics =
            if !comma && !generics.is_empty() { quote! { #generics, } }
            else                              { quote! { #generics  } };
        let (front, with) = if let Some(with) = self.with.clone() {
            (generics.clone(), with)
        } else {
            (quote! { #generics Extra: Copy }, quote! { Extra })
        };
        let mut arms = TokenStream::new();
        for (num, rule) in self.rules.iter().enumerate() {
            let variant = &rule.variant;
            let (argb, argv) = build_ast(&rule.body, &rule.fields, &with)?;
            let argv = {
                if rule.named { quote! { {#(#argv)*} } }
                else          { quote! { (#(#argv)*) } }
            };
            let trace = rule.trace;
            arms.extend(if self.is_enum {
                let trace =
                    if trace { quote!{ println!("AST\t{}::{}\t{stack:?}", stringify!(#this), stringify!(#variant)); } }
                    else { quote! {} };
                quote! { #num => {
                    #trace
                    #argb;
                    (stack, {Self::#variant #argv})
                } }
            } else {
                let trace =
                    if trace { quote!{ println!("AST\t{}\t{stack:?}", stringify!(#this)); } }
                    else { quote! {} };
                quote! { #num => {
                    #trace
                    #argb;
                    (stack, {Self #argv})
                } }
            });
        }
        Ok(quote!{
            impl<#front> #CRATE::AstImpl<#with> for #this<#generics> {
                fn peggen_ast<'lifetime>(
                    input: &'lifetime str,
                    stack: &'lifetime [#CRATE::Tag],
                    with: #with
                ) -> (&'lifetime [#CRATE::Tag], Self) {
                    let tag = stack[stack.len()-1].rule - <Self as #CRATE::Num>::num(0);
                    let stack = &stack[..stack.len()-1];
                    match tag {
                        #arms
                        _ => unreachable!()
                    }
                }
            }
        })
    }
}

fn build_ast(expr: &RuleExpr, fields: &std::collections::HashMap<String, Type>, with: &TokenStream) -> Result<(TokenStream, Vec<TokenStream>)> {
    let field_infos = collect_fields_from_expr(expr, false, fields);
    let mut seen = std::collections::HashSet::new();
    let mut unique_fields: Vec<(String, FieldKind)> = vec![];
    for (key, kind) in field_infos {
        if seen.insert(key.clone()) {
            unique_fields.push((key, kind));
        }
    }

    let mut argb = TokenStream::new();
    let mut argv = Vec::new();
    for (key, kind) in unique_fields.iter().rev() {
        let typ = fields.get(key)
            .ok_or_else(|| Error::new(proc_macro2::Span::call_site(),
                format!("field '{key}' not found in variant")))?;
        let arg = normalize(key)?;
        match kind {
            FieldKind::Value => {
                argb.extend(quote! {
                    let (stack, #arg) = <#typ as AstImpl<#with>>::peggen_ast(input, stack, with);
                });
            }
            FieldKind::Collection => {
                argb.extend(quote! {
                    let (stack, #arg) = <#typ as PushImpl<#with>>::peggen_ast(input, stack, with);
                });
            }
        }
        argv.push(quote! { #arg, });
    }
    argv.reverse();
    Ok((argb, argv))
}
