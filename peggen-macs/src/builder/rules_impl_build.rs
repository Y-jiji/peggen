use std::collections::HashMap;
use crate::*;
use crate::rule_ast::*;

impl Builder {
    pub fn rules_impl_build(&self) -> Result<TokenStream> {
        self.rules_impl_build_with(&ImplMode::normal())
    }

    pub fn ref_rules_impl_build(&self) -> Result<TokenStream> {
        self.rules_impl_build_with(&ImplMode::reference())
    }

    fn rules_impl_build_with(&self, mode: &ImplMode) -> Result<TokenStream> {
        let mut impls = TokenStream::new();
        let rule_trait = &mode.rule_trait;
        let rule_method = &mode.rule_method;
        let r#impl = |num, ident, generics, variant, trace, body| {
            let (trace_start, trace_end_ok, trace_end_err) =
                if trace { (
                    quote!{println!("TRY\t{}::{} @ {end}\t{}", stringify!(#ident), stringify!(#variant), &input[end..]);},
                    quote!{println!("OK\t{}::{} @ {start}..{end}\t{}", stringify!(#ident), stringify!(#variant), &input[start..end]);},
                    quote!{println!("ERR\t{}::{}", stringify!(#ident), stringify!(#variant)); },
                ) }
                else { (quote!{}, quote!{}, quote!{}) };
            quote! {
                impl<#generics const ERROR: bool> #rule_trait<#num, ERROR> for #ident<#generics> {
                    #[inline(always)]
                    fn #rule_method(
                        input: &str, end: usize,
                        depth: usize,
                        first: bool,
                        ctx: &mut #CRATE::ParseContext,
                    ) -> Result<usize, ()> {
                        #trace_start
                        let start = end;
                        let mut head = true;
                        let Ok(end) = (#body) else {
                            #trace_end_err
                            return Err(());
                        };
                        if start < end {
                            #trace_end_ok
                            #CRATE::stack_sanity_check(input, &ctx.tags, start..end);
                            ctx.tags.push(#CRATE::Tag { rule: <Self as Num>::num(#num), span: start..end });
                            Ok(end)
                        } else {
                            while ctx.tags.last().map(|tag| tag.span.start > start).unwrap_or(false) {
                                ctx.tags.pop();
                            }
                            #trace_end_err
                            Err(())
                        }
                    }
                }
            }
        };
        for (num, rule) in self.rules.iter().enumerate() {
            let body = self.rules_expr_build(&rule.body, &rule.fields, mode)?;
            impls.extend(r#impl(num, &self.ident, &self.generics, &rule.variant, rule.trace, body));
        };
        Ok(impls)
    }

    fn rules_expr_build(&self, expr: &RuleExpr, fields: &HashMap<String, Type>, mode: &ImplMode) -> Result<TokenStream> {
        let parse_trait = &mode.parse_trait;
        let parse_method = &mode.parse_method;
        match expr {
            RuleExpr::Literal(s) => Ok(quote! {{
                if head && first { Err(()) }
                else if input[end..].starts_with(#s) {
                    head &= #s.len() == 0;
                    Ok::<_, ()>(end + #s.len())
                }
                else { Err(()) }
            }}),

            RuleExpr::Field(fref) => {
                let key = fref.key();
                let typ = fields.get(&key)
                    .ok_or_else(|| Error::new(proc_macro2::Span::call_site(),
                        format!("unknown field '{}' in rule for '{}'", key, self.ident)))?;
                Ok(quote! {{
                    match <#typ as #parse_trait<0, ERROR>>::#parse_method(
                        input, end,
                        if head { depth + 1 } else { 0 },
                        head && first,
                        ctx
                    ) {
                        Ok(end_) if end_ > end => { head = false; Ok(end_) }
                        other => other
                    }
                }})
            }

            RuleExpr::FieldTag(fref, tag) => {
                let key = fref.key();
                let typ = fields.get(&key)
                    .ok_or_else(|| Error::new(proc_macro2::Span::call_site(),
                        format!("unknown field '{}' in rule for '{}'", key, self.ident)))?;
                let group = self.tag_index(tag)?;
                Ok(quote! {{
                    match <#typ as #parse_trait<#group, ERROR>>::#parse_method(
                        input, end,
                        if head { depth + 1 } else { 0 },
                        head && first,
                        ctx
                    ) {
                        Ok(end_) if end_ > end => { head = false; Ok(end_) }
                        other => other
                    }
                }})
            }

            RuleExpr::FieldRegex(_fref, name) => {
                if let Some(sub_expr) = self.context.subrules.get(name).cloned() {
                    let key = _fref.key();
                    let typ = fields.get(&key)
                        .ok_or_else(|| Error::new(proc_macro2::Span::call_site(),
                            format!("unknown field '{}' in rule for '{}'", key, self.ident)))?;
                    let sub_fields = crate::builder::build_item_fields(typ);
                    self.rules_expr_build(&sub_expr, &sub_fields, mode)
                } else if let Some(regex_pattern) = self.context.regexes.get(name).cloned() {
                    Ok(quote! {{(|| -> Result<usize, ()> {
                        if head && first { Err(())? }
                        static REGEX: #CRATE::LazyLock<#CRATE::Regex> = #CRATE::LazyLock::new(|| {
                            #CRATE::Regex::new(concat!("^(", #regex_pattern, ")")).unwrap()
                        });
                        let start = end;
                        let end = start + REGEX.find(&input[end..]).map(|mat| mat.as_str().len()).ok_or(())?;
                        #CRATE::stack_sanity_check(input, &ctx.tags, start..end);
                        ctx.tags.push(#CRATE::Tag { rule: 0, span: start..end });
                        head = false;
                        Ok::<_, ()>(end)
                    })()}})
                } else {
                    Err(Error::new(proc_macro2::Span::call_site(),
                        format!("unknown regex or subrule '{name}', declare with #[regex({name} = r\"...\")] or #[subrule({name} = ...)]")))
                }
            }

            RuleExpr::FieldMulti(frefs, name) => {
                let sub_expr = self.context.subrules.get(name)
                    .ok_or_else(|| Error::new(proc_macro2::Span::call_site(),
                        format!("${{...}}:{name} requires a subrule, not a regex; declare with #[subrule({name} = ...)]")))?;
                let remapped = crate::builder::remap_subrule_fields(sub_expr, frefs);
                self.rules_expr_build(&remapped, fields, mode)
            }

            RuleExpr::SubruleRef(name) => {
                if let Some(sub_expr) = self.context.subrules.get(name).cloned() {
                    if sub_expr.has_field_refs() {
                        return Err(Error::new(proc_macro2::Span::call_site(),
                            format!("subrule '{name}' has field captures ($0, $1, ...); use $field:{name} to capture or ${{f1,f2}}:{name} to unpack")));
                    }
                    self.rules_expr_build(&sub_expr, fields, mode)
                } else if let Some(regex_pattern) = self.context.regexes.get(name).cloned() {
                    Ok(quote! {{(|| -> Result<usize, ()> {
                        if head && first { Err(())? }
                        static REGEX: #CRATE::LazyLock<#CRATE::Regex> = #CRATE::LazyLock::new(|| {
                            #CRATE::Regex::new(concat!("^(", #regex_pattern, ")")).unwrap()
                        });
                        let end = end + REGEX.find(&input[end..]).map(|mat| mat.as_str().len()).ok_or(())?;
                        head = false;
                        Ok::<_, ()>(end)
                    })()}})
                } else {
                    Err(Error::new(proc_macro2::Span::call_site(),
                        format!("unknown subrule or regex '{name}', declare with #[subrule({name} = ...)] or #[regex({name} = r\"...\")]")))
                }
            }

            RuleExpr::Seq(elems) => {
                let parts = elems.iter()
                    .map(|elem| self.rules_expr_build(elem, fields, mode))
                    .collect::<Result<Vec<_>>>()?;
                Ok(quote! {{(|| -> Result<usize, ()> {
                    let size = ctx.tags.len();
                    #(let Ok(end) = (#parts) else {
                        ctx.tags.resize_with(size, || unreachable!());
                        Err(())?
                    };)*
                    Ok::<_, ()>(end)
                })()}})
            }

            RuleExpr::Choice(a, b) => {
                let code_a = self.rules_expr_build(a, fields, mode)?;
                let code_b = self.rules_expr_build(b, fields, mode)?;
                Ok(quote! {{(|| -> Result<usize, ()> {
                    let size = ctx.tags.len();
                    match (#code_a) {
                        Ok(end) => Ok(end),
                        Err(()) => {
                            ctx.tags.resize_with(size, || unreachable!());
                            #code_b
                        }
                    }
                })()}})
            }

            RuleExpr::Rep(inner, kind) => {
                let expanded = crate::builder::expand_subrule_refs(inner, &self.context.subrules);
                let has_fields = expanded.has_field_refs();
                if has_fields {
                    let resolved = crate::builder::resolve_rep_fields(&expanded, fields);
                    let sub_fields = resolved.as_ref().map(|(_, f)| f).unwrap_or(fields);
                    self.rules_collection_rep_build(&expanded, *kind, sub_fields, mode)
                } else {
                    self.rules_simple_rep_build(&expanded, *kind, fields, mode)
                }
            }

            RuleExpr::SepRep { expr, sep, at_least_one } => {
                let expanded = crate::builder::expand_subrule_refs(expr, &self.context.subrules);
                let has_fields = expanded.has_field_refs();
                if has_fields {
                    let resolved = crate::builder::resolve_rep_fields(&expanded, fields);
                    let sub_fields = resolved.as_ref().map(|(_, f)| f).unwrap_or(fields);
                    self.rules_sep_rep_build(&expanded, sep, *at_least_one, sub_fields, mode)
                } else {
                    self.rules_sep_rep_build(&expanded, sep, *at_least_one, fields, mode)
                }
            }

            RuleExpr::Not(inner) => {
                let code = self.rules_expr_build(inner, fields, mode)?;
                Ok(quote! {{
                    let saved = ctx.tags.len();
                    let result = #code;
                    ctx.tags.truncate(saved);
                    match result {
                        Ok(_) => Err(()),
                        Err(()) => Ok::<_, ()>(end),
                    }
                }})
            }

            RuleExpr::And(inner) => {
                let code = self.rules_expr_build(inner, fields, mode)?;
                Ok(quote! {{
                    let saved = ctx.tags.len();
                    let result = #code;
                    ctx.tags.truncate(saved);
                    match result {
                        Ok(_) => Ok::<_, ()>(end),
                        Err(()) => Err(()),
                    }
                }})
            }
        }
    }

    fn rules_simple_rep_build(&self, inner: &RuleExpr, kind: RepKind, fields: &HashMap<String, Type>, mode: &ImplMode) -> Result<TokenStream> {
        let inner_code = self.rules_expr_build(inner, fields, mode)?;
        match kind {
            RepKind::ZeroOrMore => Ok(quote! {{
                let mut end = end;
                while let Ok(end_) = (#inner_code) {
                    if end_ <= end { break; }
                    end = end_;
                }
                Ok::<_, ()>(end)
            }}),
            RepKind::OneOrMore => {
                let inner_code2 = self.rules_expr_build(inner, fields, mode)?;
                Ok(quote! {{(|| -> Result<usize, ()> {
                    let Ok(mut end) = (#inner_code) else { Err(())? };
                    while let Ok(end_) = (#inner_code2) {
                        if end_ <= end { break; }
                        end = end_;
                    }
                    Ok::<_, ()>(end)
                })()}})
            }
            RepKind::Optional => Ok(quote! {{
                match (#inner_code) {
                    Ok(end) => Ok::<_, ()>(end),
                    Err(()) => Ok::<_, ()>(end),
                }
            }}),
        }
    }

    fn rules_collection_rep_build(&self, inner: &RuleExpr, kind: RepKind, fields: &HashMap<String, Type>, mode: &ImplMode) -> Result<TokenStream> {
        let inner_code = self.rules_expr_build(inner, fields, mode)?;
        match kind {
            RepKind::ZeroOrMore => Ok(quote! {{(|| -> Result<usize, ()> {
                let size = ctx.tags.len();
                let mut cnt = 0usize;
                let start = end;
                let mut end = end;
                while let Ok(end_) = (#inner_code) {
                    if end_ <= end { break; }
                    end = end_;
                    cnt += 1;
                }
                #CRATE::stack_sanity_check(input, &ctx.tags, start..end);
                ctx.tags.push(#CRATE::Tag { rule: cnt, span: start..end });
                Ok::<_, ()>(end)
            })()}}),
            RepKind::OneOrMore => {
                let inner_code2 = self.rules_expr_build(inner, fields, mode)?;
                Ok(quote! {{(|| -> Result<usize, ()> {
                    let size = ctx.tags.len();
                    let mut cnt = 0usize;
                    let start = end;
                    let Ok(mut end) = (#inner_code) else {
                        ctx.tags.resize_with(size, || unreachable!());
                        Err(())?
                    };
                    cnt += 1;
                    while let Ok(end_) = (#inner_code2) {
                        if end_ <= end { break; }
                        end = end_;
                        cnt += 1;
                    }
                    #CRATE::stack_sanity_check(input, &ctx.tags, start..end);
                    ctx.tags.push(#CRATE::Tag { rule: cnt, span: start..end });
                    Ok::<_, ()>(end)
                })()}})
            }
            RepKind::Optional => {
                Ok(quote! {{(|| -> Result<usize, ()> {
                    let size = ctx.tags.len();
                    let mut cnt = 0usize;
                    let start = end;
                    let end = match (#inner_code) {
                        Ok(end) => { cnt = 1; end }
                        Err(()) => { ctx.tags.resize_with(size, || unreachable!()); end }
                    };
                    #CRATE::stack_sanity_check(input, &ctx.tags, start..end);
                    ctx.tags.push(#CRATE::Tag { rule: cnt, span: start..end });
                    Ok::<_, ()>(end)
                })()}})
            }
        }
    }

    fn rules_sep_rep_build(&self, expr: &RuleExpr, sep: &RuleExpr, at_least_one: bool, fields: &HashMap<String, Type>, mode: &ImplMode) -> Result<TokenStream> {
        let has_fields = expr.has_field_refs();
        let item_code = self.rules_expr_build(expr, fields, mode)?;
        let item_code2 = self.rules_expr_build(expr, fields, mode)?;
        let sep_code = self.rules_expr_build(sep, fields, mode)?;

        if has_fields {
            if at_least_one {
                Ok(quote! {{(|| -> Result<usize, ()> {
                    let size = ctx.tags.len();
                    let mut cnt = 0usize;
                    let start = end;
                    let Ok(mut end) = (#item_code) else {
                        ctx.tags.resize_with(size, || unreachable!());
                        Err(())?
                    };
                    cnt += 1;
                    loop {
                        let saved_end = end;
                        let saved_stack = ctx.tags.len();
                        let Ok(end_sep) = ((|| -> Result<usize, ()> { let end = end; #sep_code })()) else {
                            break;
                        };
                        match ((|| -> Result<usize, ()> { let end = end_sep; #item_code2 })()) {
                            Ok(end_) if end_ > saved_end => { end = end_; cnt += 1; }
                            _ => {
                                ctx.tags.resize_with(saved_stack, || unreachable!());
                                break;
                            }
                        }
                    }
                    #CRATE::stack_sanity_check(input, &ctx.tags, start..end);
                    ctx.tags.push(#CRATE::Tag { rule: cnt, span: start..end });
                    Ok::<_, ()>(end)
                })()}})
            } else {
                Ok(quote! {{(|| -> Result<usize, ()> {
                    let size = ctx.tags.len();
                    let mut cnt = 0usize;
                    let start = end;
                    let mut end = end;
                    if let Ok(end_) = (#item_code) {
                        end = end_;
                        cnt += 1;
                        loop {
                            let saved_end = end;
                            let saved_stack = ctx.tags.len();
                            let Ok(end_sep) = ((|| -> Result<usize, ()> { let end = end; #sep_code })()) else {
                                break;
                            };
                            match ((|| -> Result<usize, ()> { let end = end_sep; #item_code2 })()) {
                                Ok(end_) if end_ > saved_end => { end = end_; cnt += 1; }
                                _ => {
                                    ctx.tags.resize_with(saved_stack, || unreachable!());
                                    break;
                                }
                            }
                        }
                    }
                    #CRATE::stack_sanity_check(input, &ctx.tags, start..end);
                    ctx.tags.push(#CRATE::Tag { rule: cnt, span: start..end });
                    Ok::<_, ()>(end)
                })()}})
            }
        } else {
            if at_least_one {
                Ok(quote! {{(|| -> Result<usize, ()> {
                    let start = end;
                    let Ok(mut end) = (#item_code) else { Err(())? };
                    loop {
                        let saved_end = end;
                        let Ok(end_sep) = ((|| -> Result<usize, ()> { let end = end; #sep_code })()) else {
                            break;
                        };
                        match ((|| -> Result<usize, ()> { let end = end_sep; #item_code2 })()) {
                            Ok(end_) if end_ > saved_end => { end = end_; }
                            _ => { break; }
                        }
                    }
                    Ok::<_, ()>(end)
                })()}})
            } else {
                Ok(quote! {{
                    let mut end = end;
                    if let Ok(end_) = (#item_code) {
                        end = end_;
                        loop {
                            let saved_end = end;
                            let Ok(end_sep) = ((|| -> Result<usize, ()> { let end = end; #sep_code })()) else {
                                break;
                            };
                            match ((|| -> Result<usize, ()> { let end = end_sep; #item_code2 })()) {
                                Ok(end_) if end_ > saved_end => { end = end_; }
                                _ => { break; }
                            }
                        }
                    }
                    Ok::<_, ()>(end)
                }})
            }
        }
    }
}
