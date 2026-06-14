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
            RuleExpr::Literal(s) => {
                if s.is_empty() {
                    Ok(quote! { Ok::<usize, ()>(end) })
                } else if s.len() == 1 {
                    let byte = s.as_bytes()[0];
                    Ok(quote! {{
                        if head && first { Err(()) }
                        else if end < input.len() && input.as_bytes()[end] == #byte {
                            head = false;
                            Ok::<_, ()>(end + 1)
                        }
                        else { Err(()) }
                    }})
                } else {
                    let bytes = proc_macro2::Literal::byte_string(s.as_bytes());
                    let len = s.len();
                    Ok(quote! {{
                        if head && first { Err(()) }
                        else if end + #len <= input.len() && &input.as_bytes()[end..end + #len] == #bytes {
                            head = false;
                            Ok::<_, ()>(end + #len)
                        }
                        else { Err(()) }
                    }})
                }
            },

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
                    if let Some(inline) = try_inline_regex(&regex_pattern) {
                        Ok(quote! {{(|| -> Result<usize, ()> {
                            if head && first { Err(())? }
                            let start = end;
                            let end = (#inline)?;
                            #CRATE::stack_sanity_check(input, &ctx.tags, start..end);
                            ctx.tags.push(#CRATE::Tag { rule: 0, span: start..end });
                            head = false;
                            Ok::<_, ()>(end)
                        })()}})
                    } else {
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
                    }
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
                    if let Some(inline) = try_inline_regex(&regex_pattern) {
                        Ok(quote! {{(|| -> Result<usize, ()> {
                            if head && first { Err(())? }
                            let end = (#inline)?;
                            head = false;
                            Ok::<_, ()>(end)
                        })()}})
                    } else {
                        Ok(quote! {{(|| -> Result<usize, ()> {
                            if head && first { Err(())? }
                            static REGEX: #CRATE::LazyLock<#CRATE::Regex> = #CRATE::LazyLock::new(|| {
                                #CRATE::Regex::new(concat!("^(", #regex_pattern, ")")).unwrap()
                            });
                            let end = end + REGEX.find(&input[end..]).map(|mat| mat.as_str().len()).ok_or(())?;
                            head = false;
                            Ok::<_, ()>(end)
                        })()}})
                    }
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

#[derive(Debug, Clone, Copy)]
enum InlineQuantifier {
    ExactlyOne,
    ZeroOrMore,
    OneOrMore,
}

pub(crate) fn try_inline_regex(pattern: &str) -> Option<TokenStream> {
    let (chars, negated, quantifier) = parse_simple_pattern(pattern)?;
    let match_arms = generate_byte_match_arms(&chars, negated);

    match quantifier {
        InlineQuantifier::ExactlyOne => Some(quote! {
            if end < input.len() && matches!(input.as_bytes()[end], #match_arms) {
                Ok::<usize, ()>(end + 1)
            } else {
                Err(())
            }
        }),
        InlineQuantifier::ZeroOrMore => Some(quote! {{
            let mut __pos = end;
            let __bytes = input.as_bytes();
            while __pos < __bytes.len() && matches!(__bytes[__pos], #match_arms) {
                __pos += 1;
            }
            Ok::<usize, ()>(__pos)
        }}),
        InlineQuantifier::OneOrMore => Some(quote! {{
            let __bytes = input.as_bytes();
            if end < __bytes.len() && matches!(__bytes[end], #match_arms) {
                let mut __pos = end + 1;
                while __pos < __bytes.len() && matches!(__bytes[__pos], #match_arms) {
                    __pos += 1;
                }
                Ok::<usize, ()>(__pos)
            } else {
                Err(())
            }
        }}),
    }
}

fn parse_simple_pattern(pattern: &str) -> Option<(Vec<u8>, bool, InlineQuantifier)> {
    let bytes = pattern.as_bytes();

    if bytes.starts_with(b"\\s") {
        let ws = vec![b' ', b'\t', b'\n', b'\r', 0x0b, 0x0c];
        match &bytes[2..] {
            b"*" => return Some((ws, false, InlineQuantifier::ZeroOrMore)),
            b"+" => return Some((ws, false, InlineQuantifier::OneOrMore)),
            b"" => return Some((ws, false, InlineQuantifier::ExactlyOne)),
            _ => return None,
        }
    }

    if bytes.first() != Some(&b'[') { return None; }
    let close = bytes.iter().rposition(|&b| b == b']')?;
    let rest = &bytes[close + 1..];
    let quantifier = match rest {
        b"*" => InlineQuantifier::ZeroOrMore,
        b"+" => InlineQuantifier::OneOrMore,
        b"" => InlineQuantifier::ExactlyOne,
        _ => return None,
    };

    let mut i = 1;
    let negated = i < close && bytes[i] == b'^';
    if negated { i += 1; }

    let mut chars = Vec::new();
    while i < close {
        if bytes[i] == b'\\' {
            i += 1;
            if i >= close { return None; }
            match bytes[i] {
                b'd' => chars.extend(b'0'..=b'9'),
                b'w' => {
                    chars.extend(b'0'..=b'9');
                    chars.extend(b'a'..=b'z');
                    chars.extend(b'A'..=b'Z');
                    chars.push(b'_');
                }
                b's' => chars.extend(&[b' ', b'\t', b'\n', b'\r']),
                ch => chars.push(ch),
            }
            i += 1;
        } else if i + 2 < close && bytes[i + 1] == b'-' {
            for ch in bytes[i]..=bytes[i + 2] { chars.push(ch); }
            i += 3;
        } else {
            chars.push(bytes[i]);
            i += 1;
        }
    }

    if chars.is_empty() && !negated { return None; }
    Some((chars, negated, quantifier))
}

fn generate_byte_match_arms(chars: &[u8], negated: bool) -> TokenStream {
    let target = if negated {
        (0u8..=127).filter(|b| !chars.contains(b)).collect::<Vec<_>>()
    } else {
        chars.to_vec()
    };
    let patterns = compress_inline_ranges(&target);
    quote! { #(#patterns)|* }
}

fn compress_inline_ranges(bytes: &[u8]) -> Vec<TokenStream> {
    if bytes.is_empty() { return vec![quote! { 0 if false }]; }
    let mut sorted = bytes.to_vec();
    sorted.sort();
    sorted.dedup();
    let mut patterns = Vec::new();
    let mut i = 0;
    while i < sorted.len() {
        let start = sorted[i];
        let mut end = start;
        while i + 1 < sorted.len() && sorted[i + 1] == end + 1 {
            end = sorted[i + 1];
            i += 1;
        }
        if start == end {
            patterns.push(quote! { #start });
        } else {
            patterns.push(quote! { #start..=#end });
        }
        i += 1;
    }
    patterns
}
