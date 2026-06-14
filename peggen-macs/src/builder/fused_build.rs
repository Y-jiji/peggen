use std::collections::HashMap;
use quote::format_ident;
use crate::*;
use crate::rule_ast::*;
use crate::builder::first_set::*;
use crate::builder::ast_impl_build::FieldKind;
use crate::builder::rules_impl_build::try_inline_regex;
use crate::builder::parse_impl_build::compress_byte_ranges;

impl Builder {
    pub fn fused_parse_impl_build(&self) -> Result<TokenStream> {
        if !self.all_tags.is_empty() {
            if self.is_linear_tag_chain() {
                return self.fused_pratt_parse_impl_build();
            }
            return Ok(TokenStream::new());
        }

        let ident = &self.ident;
        let generics = &self.generics;
        let (front, with) = self.fused_generics();

        let mut impls = TokenStream::new();
        for group in 0..=self.group {
            let body = self.fused_dispatch_body(group)?;
            impls.extend(quote! {
                impl<#front const ERROR: bool> #CRATE::FusedParseImpl<#group, ERROR, #with> for #ident<#generics> {
                    #[inline(always)]
                    fn fused_parse_impl(
                        input: &str, end: usize,
                        depth: usize,
                        first: bool,
                        ctx: &mut #CRATE::ParseContext,
                        extra: #with,
                    ) -> Result<(usize, Self), ()> {
                        #body
                    }
                }
            });
        }
        Ok(impls)
    }

    pub fn fused_rules_impl_build(&self) -> Result<TokenStream> {
        if !self.all_tags.is_empty() {
            if self.is_linear_tag_chain() {
                return self.fused_pratt_rules_impl_build();
            }
            return Ok(TokenStream::new());
        }

        let ident = &self.ident;
        let generics = &self.generics;
        let (front, with) = self.fused_generics();

        let mut impls = TokenStream::new();
        for (num, rule) in self.rules.iter().enumerate() {
            let body = self.fused_rule_body(num, rule)?;
            impls.extend(quote! {
                impl<#front const ERROR: bool> #CRATE::FusedRuleImpl<#num, ERROR, #with> for #ident<#generics> {
                    #[inline(always)]
                    fn fused_rule_impl(
                        input: &str, end: usize,
                        depth: usize,
                        first: bool,
                        ctx: &mut #CRATE::ParseContext,
                        extra: #with,
                    ) -> Result<(usize, Self), ()> {
                        #body
                    }
                }
            });
        }
        Ok(impls)
    }

    pub fn bracket_pairs_build(&self) -> Result<TokenStream> {
        let ident = &self.ident;
        let generics = &self.generics;
        let pairs = crate::builder::bracket_analysis::analyze_bracket_pairs(&self.rules);
        let pair_tuples: Vec<TokenStream> = pairs.iter().map(|p| {
            let open = &p.open;
            let close = &p.close;
            quote! { (#open, #close) }
        }).collect();

        Ok(quote! {
            impl<#generics> #CRATE::BracketPairs for #ident<#generics> {
                fn bracket_pairs() -> &'static [(&'static str, &'static str)] {
                    &[#(#pair_tuples),*]
                }
            }
        })
    }

    fn fused_generics(&self) -> (TokenStream, TokenStream) {
        let generics = &self.generics;
        if let Some(with) = self.with.clone() {
            let comma = generics.to_token_stream().into_iter().last()
                .map(|x: proc_macro2::TokenTree| x.to_string() == ",").unwrap_or(false);
            let front = if !comma && !generics.is_empty() {
                quote! { #generics, }
            } else {
                quote! { #generics }
            };
            (front, with)
        } else {
            (quote! { #generics Extra: Copy, }, quote! { Extra })
        }
    }

    fn is_linear_tag_chain(&self) -> bool {
        if self.all_tags.is_empty() { return false; }
        let mut sorted_rules: Vec<&Rule> = self.rules.iter().collect();
        sorted_rules.sort_by_key(|r| r.tags.len());
        for i in 0..sorted_rules.len() {
            for j in (i+1)..sorted_rules.len() {
                let smaller = &sorted_rules[i].tags;
                let larger = &sorted_rules[j].tags;
                if smaller.len() < larger.len() {
                    if !smaller.iter().all(|t| larger.contains(t)) {
                        return false;
                    }
                }
            }
        }
        true
    }

    fn extract_infix_parts(&self, rule: &Rule) -> Option<(String, String, Vec<RuleExpr>, Vec<RuleExpr>)> {
        let RuleExpr::Seq(elems) = &rule.body else { return None };
        let RuleExpr::FieldTag(_, left_tag) = &elems[0] else { return None };
        let mut op_idx = None;
        for (i, elem) in elems[1..].iter().enumerate() {
            if matches!(elem, RuleExpr::Literal(_)) {
                op_idx = Some(i + 1);
                break;
            }
        }
        let op_idx = op_idx?;
        let RuleExpr::Literal(op_lit) = &elems[op_idx] else { return None };
        let pre_op = elems[1..op_idx].to_vec();
        let post_op = elems[op_idx + 1..].to_vec();
        Some((left_tag.clone(), op_lit.clone(), pre_op, post_op))
    }

    fn fused_pratt_parse_impl_build(&self) -> Result<TokenStream> {
        let ident = &self.ident;
        let generics = &self.generics;
        let (front, with) = self.fused_generics();
        let highest_group = self.all_tags.len() - 1;

        let mut impls = TokenStream::new();
        for (tag_idx, _tag_name) in self.all_tags.iter().enumerate() {
            let body = self.fused_pratt_group_body(tag_idx, highest_group)?;
            let group = tag_idx;
            impls.extend(quote! {
                impl<#front const ERROR: bool> #CRATE::FusedParseImpl<#group, ERROR, #with> for #ident<#generics> {
                    #[inline(always)]
                    fn fused_parse_impl(
                        input: &str, end: usize,
                        depth: usize,
                        first: bool,
                        ctx: &mut #CRATE::ParseContext,
                        extra: #with,
                    ) -> Result<(usize, Self), ()> {
                        #body
                    }
                }
            });
        }
        Ok(impls)
    }

    fn fused_pratt_rules_impl_build(&self) -> Result<TokenStream> {
        let ident = &self.ident;
        let generics = &self.generics;
        let (front, with) = self.fused_generics();

        let mut impls = TokenStream::new();
        for (num, rule) in self.rules.iter().enumerate() {
            if self.is_infix(rule) { continue; }
            let body = self.fused_rule_body(num, rule)?;
            impls.extend(quote! {
                impl<#front const ERROR: bool> #CRATE::FusedRuleImpl<#num, ERROR, #with> for #ident<#generics> {
                    #[inline(always)]
                    fn fused_rule_impl(
                        input: &str, end: usize,
                        depth: usize,
                        first: bool,
                        ctx: &mut #CRATE::ParseContext,
                        extra: #with,
                    ) -> Result<(usize, Self), ()> {
                        #body
                    }
                }
            });
        }
        Ok(impls)
    }

    fn is_infix(&self, rule: &Rule) -> bool {
        match &rule.body {
            RuleExpr::Seq(elems) if !elems.is_empty() => {
                matches!(&elems[0], RuleExpr::FieldTag(..))
            }
            _ => false,
        }
    }

    fn fused_pratt_group_body(&self, group: usize, highest_group: usize) -> Result<TokenStream> {
        let (_, with) = self.fused_generics();

        if group == highest_group {
            let primary_rules: Vec<(usize, &Rule)> = self.rules.iter().enumerate()
                .filter(|(_, r)| !self.is_infix(r) && r.tags.contains(&self.all_tags[group]))
                .collect();
            let dispatch = self.fused_dispatch_body_for_rules(&primary_rules)?;
            return Ok(dispatch);
        }

        let highest = highest_group;
        let mut infix_arms = Vec::new();

        for (_num, rule) in self.rules.iter().enumerate() {
            if !self.is_infix(rule) { continue; }
            let Some((left_tag, op_lit, _pre_op, post_op)) = self.extract_infix_parts(rule) else { continue };
            let left_tag_idx = self.tag_index(&left_tag)?;
            if group > left_tag_idx { continue; }

            let op_bytes = op_lit.as_bytes();
            let op_len = op_bytes.len();

            let mut post_stmts = TokenStream::new();
            let mut right_field_var = None;
            for elem in &post_op {
                match elem {
                    RuleExpr::FieldTag(fref, tag) => {
                        let key = fref.key();
                        let tag_group = self.tag_index(tag)?;
                        let typ = rule.fields.get(&key)
                            .ok_or_else(|| Error::new(proc_macro2::Span::call_site(),
                                format!("field '{}' not found in infix rule", key)))?;
                        let var = format_ident!("__ff_{}", key);
                        right_field_var = Some((key.clone(), var.clone()));
                        post_stmts.extend(quote! {
                            let (__pratt_end, #var) = match <#typ as #CRATE::FusedParseImpl<#tag_group, ERROR, #with>>::fused_parse_impl(
                                input, __pratt_end, 0, false, ctx, extra
                            ) {
                                Ok(r) => r,
                                Err(()) => break,
                            };
                        });
                    }
                    other => {
                        let sub_fields = HashMap::new();
                        let stmt = self.fused_stmt_pratt(other, &sub_fields)?;
                        post_stmts.extend(stmt);
                    }
                }
            }

            let constructor = self.build_pratt_constructor(rule, right_field_var.as_ref())?;

            let op_match = if op_len == 1 {
                let byte = op_bytes[0];
                quote! { Some(&#byte) }
            } else {
                let bytes = proc_macro2::Literal::byte_string(op_bytes);
                quote! { Some(#bytes) }
            };

            let op_check = if op_len == 1 {
                quote! { input.as_bytes().get(__pratt_ws_end) }
            } else {
                quote! {
                    if __pratt_ws_end + #op_len <= input.len() {
                        Some(&input.as_bytes()[__pratt_ws_end..__pratt_ws_end + #op_len])
                    } else { None }
                }
            };

            infix_arms.push(quote! {
                if matches!(#op_check, #op_match) {
                    let mut __pratt_end = __pratt_ws_end + #op_len;
                    #post_stmts
                    left = #constructor;
                    end = __pratt_end;
                    continue;
                }
            });
        }

        if infix_arms.is_empty() {
            let dispatch = self.fused_pratt_group_body(highest_group, highest_group)?;
            return Ok(dispatch);
        }

        let ws_code = self.fused_pratt_whitespace_on_end()?;

        Ok(quote! {{
            let (mut end, mut left) = <Self as #CRATE::FusedParseImpl<#highest, ERROR, #with>>::fused_parse_impl(
                input, end, depth, first, ctx, extra
            )?;
            loop {
                let mut __pratt_ws_end = end;
                #ws_code
                #(#infix_arms)*
                break;
            }
            Ok((end, left))
        }})
    }

    fn fused_pratt_whitespace_on_end(&self) -> Result<TokenStream> {
        if let Some(ws_pattern) = self.context.regexes.get("_") {
            if let Some(inline) = try_inline_regex(ws_pattern) {
                Ok(quote! {
                    if let Ok(__ws_end) = { let end = __pratt_ws_end; #inline } {
                        __pratt_ws_end = __ws_end;
                    }
                })
            } else {
                Ok(quote! {
                    {
                        static REGEX: #CRATE::LazyLock<#CRATE::Regex> = #CRATE::LazyLock::new(|| {
                            #CRATE::Regex::new(concat!("^(", #ws_pattern, ")")).unwrap()
                        });
                        if let Some(mat) = REGEX.find(&input[__pratt_ws_end..]) {
                            __pratt_ws_end += mat.as_str().len();
                        }
                    }
                })
            }
        } else {
            Ok(TokenStream::new())
        }
    }

    fn fused_stmt_pratt(&self, expr: &RuleExpr, _fields: &HashMap<String, Type>) -> Result<TokenStream> {
        match expr {
            RuleExpr::SubruleRef(name) => {
                if let Some(regex_pattern) = self.context.regexes.get(name).cloned() {
                    if let Some(inline) = try_inline_regex(&regex_pattern) {
                        Ok(quote! {
                            let __pratt_end = match { let end = __pratt_end; #inline } {
                                Ok(e) => e,
                                Err(()) => __pratt_end,
                            };
                        })
                    } else {
                        Ok(quote! {
                            {
                                static REGEX: #CRATE::LazyLock<#CRATE::Regex> = #CRATE::LazyLock::new(|| {
                                    #CRATE::Regex::new(concat!("^(", #regex_pattern, ")")).unwrap()
                                });
                                if let Some(mat) = REGEX.find(&input[__pratt_end..]) {
                                    let __pratt_end = __pratt_end + mat.as_str().len();
                                }
                            }
                        })
                    }
                } else {
                    Ok(TokenStream::new())
                }
            }
            RuleExpr::Literal(s) if s.is_empty() => Ok(TokenStream::new()),
            RuleExpr::Literal(s) => {
                let bytes = proc_macro2::Literal::byte_string(s.as_bytes());
                let len = s.len();
                Ok(quote! {
                    if __pratt_end + #len <= input.len() && &input.as_bytes()[__pratt_end..__pratt_end + #len] == #bytes {
                        __pratt_end += #len;
                    } else {
                        break;
                    }
                })
            }
            _ => Ok(TokenStream::new()),
        }
    }

    fn build_pratt_constructor(&self, rule: &Rule, right_field: Option<&(String, Ident)>) -> Result<TokenStream> {
        let (_, with) = self.fused_generics();
        let variant = &rule.variant;
        let mut args = Vec::new();

        let mut sorted_keys: Vec<&String> = rule.fields.keys().collect();
        sorted_keys.sort();

        for key in &sorted_keys {
            let typ = &rule.fields[*key];
            if *key == "0" {
                args.push(quote! { <#typ as #CRATE::FusedWrap<Self, #with>>::fused_wrap(left, extra) });
            } else if let Some((rkey, rvar)) = right_field {
                if *key == rkey {
                    args.push(quote! { #rvar });
                }
            }
        }

        if self.is_enum {
            if rule.named {
                let named_args: Vec<TokenStream> = sorted_keys.iter().zip(args.iter()).map(|(key, val)| {
                    let field_name = syn::parse_str::<Ident>(key)
                        .unwrap_or_else(|_| format_ident!("_{}", key));
                    quote! { #field_name: #val }
                }).collect();
                Ok(quote! { Self::#variant { #(#named_args),* } })
            } else {
                Ok(quote! { Self::#variant(#(#args),*) })
            }
        } else {
            Ok(quote! { Self(#(#args),*) })
        }
    }

    fn fused_dispatch_body_for_rules(&self, rules: &[(usize, &Rule)]) -> Result<TokenStream> {
        let (_, with) = self.fused_generics();

        let first_sets: Vec<FirstSet> = rules.iter()
            .map(|(_, rule)| compute_first_set(&rule.body, &self.context))
            .collect();

        let mut known_bytes: std::collections::HashSet<u8> = std::collections::HashSet::new();
        let mut wildcard_indices: Vec<usize> = Vec::new();
        let mut known_indices: Vec<(usize, Vec<u8>)> = Vec::new();

        for (i, fs) in first_sets.iter().enumerate() {
            match fs {
                FirstSet::Known(bytes) => {
                    for &b in bytes { known_bytes.insert(b); }
                    known_indices.push((i, bytes.clone()));
                }
                FirstSet::Unknown => wildcard_indices.push(i),
            }
        }

        let make_call = |idx: usize| -> TokenStream {
            let (num, rule) = &rules[idx];
            let num = *num;
            let error = rule.error;
            quote! {
                match (if #error && !ERROR { Err(()) }
                       else { <Self as #CRATE::FusedRuleImpl<#num, ERROR, #with>>::fused_rule_impl(input, end, depth, first, ctx, extra) }) {
                    Ok(r) => { return Ok(r) }
                    Err(()) => {}
                }
            }
        };

        if known_bytes.is_empty() {
            let rule_calls: Vec<TokenStream> = (0..rules.len()).map(|i| make_call(i)).collect();
            return Ok(quote! {{
                #(#rule_calls)*
                Err(())
            }});
        }

        let mut byte_to_ordered_rules: HashMap<u8, Vec<usize>> = HashMap::new();
        for &b in &known_bytes {
            let mut indices: Vec<usize> = Vec::new();
            for i in 0..rules.len() {
                let hit = wildcard_indices.contains(&i)
                    || known_indices.iter().any(|(ki, kb)| *ki == i && kb.contains(&b));
                if hit { indices.push(i); }
            }
            byte_to_ordered_rules.insert(b, indices);
        }

        let wildcard_calls: Vec<TokenStream> = wildcard_indices.iter().map(|&i| make_call(i)).collect();

        let mut group_map: HashMap<Vec<usize>, Vec<u8>> = HashMap::new();
        for (&byte, rule_list) in &byte_to_ordered_rules {
            group_map.entry(rule_list.clone()).or_default().push(byte);
        }

        let mut match_arms = Vec::new();
        for (rule_list, mut bytes) in group_map {
            bytes.sort();
            let byte_patterns = compress_byte_ranges(&bytes);
            let calls: Vec<TokenStream> = rule_list.iter().map(|&i| make_call(i)).collect();
            match_arms.push(quote! {
                #(Some(#byte_patterns))|* => { #(#calls)* }
            });
        }

        Ok(quote! {{
            match input.as_bytes().get(end) {
                #(#match_arms)*
                _ => { #(#wildcard_calls)* }
            }
            Err(())
        }})
    }

    fn fused_dispatch_body(&self, group: usize) -> Result<TokenStream> {
        let rules: Vec<(usize, &Rule)> = self.rules.iter().enumerate()
            .filter(|(_, rule)| rule.group >= group)
            .collect();

        let (_, with) = self.fused_generics();

        let first_sets: Vec<FirstSet> = rules.iter()
            .map(|(_, rule)| compute_first_set(&rule.body, &self.context))
            .collect();

        let mut known_bytes: std::collections::HashSet<u8> = std::collections::HashSet::new();
        let mut wildcard_indices: Vec<usize> = Vec::new();
        let mut known_indices: Vec<(usize, Vec<u8>)> = Vec::new();

        for (i, fs) in first_sets.iter().enumerate() {
            match fs {
                FirstSet::Known(bytes) => {
                    for &b in bytes { known_bytes.insert(b); }
                    known_indices.push((i, bytes.clone()));
                }
                FirstSet::Unknown => wildcard_indices.push(i),
            }
        }

        let make_call_committed = |idx: usize| -> TokenStream {
            let (num, rule) = &rules[idx];
            let num = *num;
            let error = rule.error;
            quote! {
                match (if #error && !ERROR { Err(()) }
                       else { <Self as #CRATE::FusedRuleImpl<#num, ERROR, #with>>::fused_rule_impl(input, end, depth, first, ctx, extra) }) {
                    Ok(r) => { return Ok(r) }
                    Err(()) => {}
                }
            }
        };

        let make_call_probed = |idx: usize| -> TokenStream {
            let (num, rule) = &rules[idx];
            let num = *num;
            let error = rule.error;
            quote! {
                {
                    let __dsaved = ctx.tags.len();
                    let __dprobe = if #error && !ERROR { Err(()) }
                                   else { <Self as #CRATE::RuleImpl<#num, ERROR>>::rule_impl(input, end, depth, first, ctx) };
                    ctx.tags.truncate(__dsaved);
                    if let Ok(_) = __dprobe {
                        return <Self as #CRATE::FusedRuleImpl<#num, ERROR, #with>>::fused_rule_impl(input, end, depth, first, ctx, extra);
                    }
                }
            }
        };

        let rule_atomicity: Vec<bool> = rules.iter()
            .map(|(_, rule)| self.expr_is_fused_atomic(&rule.body))
            .collect();
        let make_calls = |rule_list: &[usize]| -> Vec<TokenStream> {
            rule_list.iter().enumerate().map(|(i, &idx)| {
                if rule_list.len() == 1 {
                    make_call_committed(idx)
                } else if i < rule_list.len() - 1 {
                    if rule_atomicity[idx] {
                        make_call_committed(idx)
                    } else {
                        make_call_probed(idx)
                    }
                } else {
                    make_call_committed(idx)
                }
            }).collect()
        };

        if known_bytes.is_empty() {
            let indices: Vec<usize> = (0..rules.len()).collect();
            let rule_calls = make_calls(&indices);
            return Ok(quote! {{
                #(#rule_calls)*
                Err(())
            }});
        }

        let mut byte_to_ordered_rules: HashMap<u8, Vec<usize>> = HashMap::new();
        for &b in &known_bytes {
            let mut indices: Vec<usize> = Vec::new();
            for i in 0..rules.len() {
                let hit = wildcard_indices.contains(&i)
                    || known_indices.iter().any(|(ki, kb)| *ki == i && kb.contains(&b));
                if hit { indices.push(i); }
            }
            byte_to_ordered_rules.insert(b, indices);
        }

        let wildcard_calls = make_calls(&wildcard_indices);

        let mut group_map: HashMap<Vec<usize>, Vec<u8>> = HashMap::new();
        for (&byte, rule_list) in &byte_to_ordered_rules {
            group_map.entry(rule_list.clone()).or_default().push(byte);
        }

        let all_same_as_wildcard = group_map.keys().all(|k| {
            k.len() == wildcard_indices.len()
                && k.iter().zip(wildcard_indices.iter()).all(|(a, b)| a == b)
        });

        if all_same_as_wildcard {
            let indices: Vec<usize> = (0..rules.len()).collect();
            let rule_calls = make_calls(&indices);
            return Ok(quote! {{
                #(#rule_calls)*
                Err(())
            }});
        }

        let mut match_arms = Vec::new();
        for (rule_list, mut bytes) in group_map {
            bytes.sort();
            let byte_patterns = compress_byte_ranges(&bytes);
            let calls = make_calls(&rule_list);
            match_arms.push(quote! {
                #(Some(#byte_patterns))|* => { #(#calls)* }
            });
        }

        Ok(quote! {{
            match input.as_bytes().get(end) {
                #(#match_arms)*
                _ => { #(#wildcard_calls)* }
            }
            Err(())
        }})
    }

    fn fused_rule_body(&self, _num: usize, rule: &Rule) -> Result<TokenStream> {
        let variant = &rule.variant;

        let field_infos = collect_fields_from_expr(&rule.body, false, &rule.fields);
        let mut seen = std::collections::HashSet::new();
        let mut unique_fields: Vec<(String, FieldKind)> = vec![];
        for (key, kind) in field_infos {
            if seen.insert(key.clone()) {
                unique_fields.push((key, kind));
            }
        }

        let body_stmts = self.fused_stmt(&rule.body, &rule.fields)?;

        let args: Vec<TokenStream> = unique_fields.iter().map(|(key, _)| {
            let var = format_ident!("__ff_{}", key);
            quote! { #var }
        }).collect();

        let constructor = if self.is_enum {
            if rule.named {
                let named_args: Vec<TokenStream> = unique_fields.iter().map(|(key, _)| {
                    let var = format_ident!("__ff_{}", key);
                    let field_name = syn::parse_str::<Ident>(key)
                        .unwrap_or_else(|_| format_ident!("_{}", key));
                    quote! { #field_name: #var }
                }).collect();
                quote! { Self::#variant { #(#named_args),* } }
            } else if args.is_empty() {
                quote! { Self::#variant }
            } else {
                quote! { Self::#variant(#(#args),*) }
            }
        } else {
            if rule.named {
                let named_args: Vec<TokenStream> = unique_fields.iter().map(|(key, _)| {
                    let var = format_ident!("__ff_{}", key);
                    let field_name = syn::parse_str::<Ident>(key)
                        .unwrap_or_else(|_| format_ident!("_{}", key));
                    quote! { #field_name: #var }
                }).collect();
                quote! { Self { #(#named_args),* } }
            } else if args.is_empty() {
                quote! { Self }
            } else {
                quote! { Self(#(#args),*) }
            }
        };

        Ok(quote! {{
            let __fused_start = end;
            let mut head = true;
            let end = end;
            #body_stmts
            if __fused_start < end || __fused_start == end {
                Ok((end, #constructor))
            } else {
                Err(())
            }
        }})
    }

    fn fused_stmt(&self, expr: &RuleExpr, fields: &HashMap<String, Type>) -> Result<TokenStream> {
        match expr {
            RuleExpr::Literal(s) => {
                if s.is_empty() {
                    Ok(TokenStream::new())
                } else if s.len() == 1 {
                    let byte = s.as_bytes()[0];
                    Ok(quote! {
                        let end = {
                            if head && first { Err::<usize, ()>(()) }
                            else if end < input.len() && input.as_bytes()[end] == #byte {
                                head = false;
                                Ok(end + 1)
                            }
                            else { Err(()) }
                        }?;
                    })
                } else {
                    let bytes = proc_macro2::Literal::byte_string(s.as_bytes());
                    let len = s.len();
                    Ok(quote! {
                        let end = {
                            if head && first { Err::<usize, ()>(()) }
                            else if end + #len <= input.len() && &input.as_bytes()[end..end + #len] == #bytes {
                                head = false;
                                Ok(end + #len)
                            }
                            else { Err(()) }
                        }?;
                    })
                }
            }

            RuleExpr::Field(fref) => {
                let key = fref.key();
                let typ = fields.get(&key)
                    .ok_or_else(|| Error::new(proc_macro2::Span::call_site(),
                        format!("field '{}' not found in fused build for '{}'", key, self.ident)))?;
                let var = format_ident!("__ff_{}", key);
                let (_, with) = self.fused_generics();
                Ok(quote! {
                    let (end, #var) = match <#typ as #CRATE::FusedParseImpl<0, ERROR, #with>>::fused_parse_impl(
                        input, end,
                        if head { depth + 1 } else { 0 },
                        head && first,
                        ctx, extra
                    ) {
                        Ok((end_, val)) if end_ > end => { head = false; (end_, val) }
                        Ok((end_, val)) => (end_, val),
                        Err(()) => return Err(()),
                    };
                })
            }

            RuleExpr::FieldTag(fref, tag) => {
                let key = fref.key();
                let typ = fields.get(&key)
                    .ok_or_else(|| Error::new(proc_macro2::Span::call_site(),
                        format!("field '{}' not found", key)))?;
                let group = self.tag_index(tag)?;
                let var = format_ident!("__ff_{}", key);
                let (_, with) = self.fused_generics();
                Ok(quote! {
                    let (end, #var) = match <#typ as #CRATE::FusedParseImpl<#group, ERROR, #with>>::fused_parse_impl(
                        input, end,
                        if head { depth + 1 } else { 0 },
                        head && first,
                        ctx, extra
                    ) {
                        Ok((end_, val)) if end_ > end => { head = false; (end_, val) }
                        Ok((end_, val)) => (end_, val),
                        Err(()) => return Err(()),
                    };
                })
            }

            RuleExpr::FieldRegex(fref, name) => {
                if let Some(sub_expr) = self.context.subrules.get(name).cloned() {
                    let key = fref.key();
                    let typ = fields.get(&key)
                        .ok_or_else(|| Error::new(proc_macro2::Span::call_site(),
                            format!("field '{}' not found", key)))?;
                    let sub_fields = crate::builder::build_item_fields(typ);
                    let var = format_ident!("__ff_{}", key);
                    let sub_stmts = self.fused_stmt(&sub_expr, &sub_fields)?;
                    let mut sorted_keys: Vec<String> = sub_fields.keys().cloned().collect();
                    sorted_keys.sort();
                    let sub_values: Vec<Ident> = sorted_keys.iter()
                        .map(|k| format_ident!("__ff_{}", k))
                        .collect();
                    let sub_value = if sub_values.len() == 1 {
                        quote! { #(#sub_values)* }
                    } else {
                        quote! { (#(#sub_values),*) }
                    };
                    Ok(quote! {
                        let (end, #var) = {
                            #sub_stmts
                            (end, #sub_value)
                        };
                    })
                } else if let Some(regex_pattern) = self.context.regexes.get(name).cloned() {
                    let key = fref.key();
                    let typ = fields.get(&key)
                        .ok_or_else(|| Error::new(proc_macro2::Span::call_site(),
                            format!("field '{}' not found", key)))?;
                    let var = format_ident!("__ff_{}", key);
                    let (_, with) = self.fused_generics();
                    let match_code = if let Some(inline) = try_inline_regex(&regex_pattern) {
                        quote! {
                            let __fused_cap_start = end;
                            let end = (#inline)?;
                        }
                    } else {
                        quote! {
                            static REGEX: #CRATE::LazyLock<#CRATE::Regex> = #CRATE::LazyLock::new(|| {
                                #CRATE::Regex::new(concat!("^(", #regex_pattern, ")")).unwrap()
                            });
                            let __fused_cap_start = end;
                            let end = __fused_cap_start + REGEX.find(&input[end..]).map(|mat| mat.as_str().len()).ok_or(())?;
                        }
                    };
                    Ok(quote! {
                        let (end, #var) = {
                            if head && first { return Err(()); }
                            #match_code
                            let __val = <#typ as #CRATE::FromStr<#with>>::from_str_with(&input[__fused_cap_start..end], extra);
                            head = false;
                            (end, __val)
                        };
                    })
                } else {
                    Err(Error::new(proc_macro2::Span::call_site(),
                        format!("unknown regex or subrule '{name}'")))
                }
            }

            RuleExpr::FieldMulti(frefs, name) => {
                let sub_expr = self.context.subrules.get(name)
                    .ok_or_else(|| Error::new(proc_macro2::Span::call_site(),
                        format!("${{...}}:{name} requires a subrule")))?;
                let remapped = crate::builder::remap_subrule_fields(sub_expr, frefs);
                self.fused_stmt(&remapped, fields)
            }

            RuleExpr::SubruleRef(name) => {
                if let Some(sub_expr) = self.context.subrules.get(name).cloned() {
                    if sub_expr.has_field_refs() {
                        return Err(Error::new(proc_macro2::Span::call_site(),
                            format!("subrule '{name}' has field captures; use $field:{name}")));
                    }
                    self.fused_stmt(&sub_expr, fields)
                } else if let Some(regex_pattern) = self.context.regexes.get(name).cloned() {
                    if let Some(inline) = try_inline_regex(&regex_pattern) {
                        Ok(quote! {
                            let end = {
                                if head && first { Err::<usize, ()>(()) }
                                else {
                                    let end = (#inline)?;
                                    head = false;
                                    Ok(end)
                                }
                            }?;
                        })
                    } else {
                        Ok(quote! {
                            let end = {
                                if head && first { Err::<usize, ()>(()) }
                                else {
                                    static REGEX: #CRATE::LazyLock<#CRATE::Regex> = #CRATE::LazyLock::new(|| {
                                        #CRATE::Regex::new(concat!("^(", #regex_pattern, ")")).unwrap()
                                    });
                                    let end = end + REGEX.find(&input[end..]).map(|mat| mat.as_str().len()).ok_or(())?;
                                    head = false;
                                    Ok(end)
                                }
                            }?;
                        })
                    }
                } else {
                    Err(Error::new(proc_macro2::Span::call_site(),
                        format!("unknown subrule or regex '{name}'")))
                }
            }

            RuleExpr::Seq(elems) => {
                if self.seq_needs_deferral(elems) {
                    return self.fused_stmt_seq_deferred(elems, fields);
                }
                let mut stmts = TokenStream::new();
                for elem in elems {
                    stmts.extend(self.fused_stmt(elem, fields)?);
                }
                Ok(stmts)
            }

            RuleExpr::Choice(a, b) => {
                let probe_a = self.fused_pos_only_code(a, fields)?;
                let code_a = self.fused_stmt(a, fields)?;
                let code_b = self.fused_stmt(b, fields)?;
                Ok(quote! {
                    if (#probe_a).is_ok() {
                        #code_a
                    } else {
                        #code_b
                    }
                })
            }

            RuleExpr::Rep(inner, kind) => {
                let expanded = crate::builder::expand_subrule_refs(inner, &self.context.subrules);
                if expanded.has_field_refs() {
                    let resolved = crate::builder::resolve_rep_fields(&expanded, fields);
                    if let Some((rep_key, sub_fields)) = resolved {
                        self.fused_collection_rep(&expanded, *kind, &rep_key, &sub_fields, fields)
                    } else {
                        self.fused_simple_rep(inner, *kind, fields)
                    }
                } else {
                    self.fused_simple_rep(inner, *kind, fields)
                }
            }

            RuleExpr::SepRep { expr, sep, at_least_one } => {
                let expanded = crate::builder::expand_subrule_refs(expr, &self.context.subrules);
                if expanded.has_field_refs() {
                    let resolved = crate::builder::resolve_rep_fields(&expanded, fields);
                    if let Some((rep_key, sub_fields)) = resolved {
                        self.fused_sep_rep(&expanded, sep, *at_least_one, &rep_key, &sub_fields, fields)
                    } else {
                        self.fused_simple_sep_rep(expr, sep, *at_least_one, fields)
                    }
                } else {
                    self.fused_simple_sep_rep(expr, sep, *at_least_one, fields)
                }
            }

            RuleExpr::Not(inner) => {
                let code = self.fused_pos_only_code(inner, fields)?;
                Ok(quote! {
                    match (#code) {
                        Ok(_) => return Err(()),
                        Err(()) => {}
                    };
                })
            }

            RuleExpr::And(inner) => {
                let code = self.fused_pos_only_code(inner, fields)?;
                Ok(quote! {
                    match (#code) {
                        Ok(_) => {}
                        Err(()) => return Err(()),
                    };
                })
            }
        }
    }

    fn fused_pos_only_code(&self, expr: &RuleExpr, fields: &HashMap<String, Type>) -> Result<TokenStream> {
        match expr {
            RuleExpr::Literal(s) if s.is_empty() => Ok(quote! { Ok::<usize, ()>(end) }),
            RuleExpr::Literal(s) if s.len() == 1 => {
                let byte = s.as_bytes()[0];
                Ok(quote! {{
                    if end < input.len() && input.as_bytes()[end] == #byte {
                        Ok::<usize, ()>(end + 1)
                    } else { Err(()) }
                }})
            }
            RuleExpr::Literal(s) => {
                let bytes = proc_macro2::Literal::byte_string(s.as_bytes());
                let len = s.len();
                Ok(quote! {{
                    if end + #len <= input.len() && &input.as_bytes()[end..end + #len] == #bytes {
                        Ok::<usize, ()>(end + #len)
                    } else { Err(()) }
                }})
            }
            RuleExpr::SubruleRef(name) => {
                if let Some(regex_pattern) = self.context.regexes.get(name).cloned() {
                    if let Some(inline) = try_inline_regex(&regex_pattern) {
                        Ok(inline)
                    } else {
                        Ok(quote! {{
                            static REGEX: #CRATE::LazyLock<#CRATE::Regex> = #CRATE::LazyLock::new(|| {
                                #CRATE::Regex::new(concat!("^(", #regex_pattern, ")")).unwrap()
                            });
                            Ok::<usize, ()>(end + REGEX.find(&input[end..]).map(|mat| mat.as_str().len()).ok_or(())?)
                        }})
                    }
                } else if let Some(sub_expr) = self.context.subrules.get(name).cloned() {
                    self.fused_pos_only_code(&sub_expr, fields)
                } else {
                    Err(Error::new(proc_macro2::Span::call_site(),
                        format!("unknown subrule/regex '{name}'")))
                }
            }
            RuleExpr::Seq(elems) => {
                let parts: Vec<TokenStream> = elems.iter()
                    .map(|e| self.fused_pos_only_code(e, fields))
                    .collect::<Result<Vec<_>>>()?;
                Ok(quote! {{(|| -> Result<usize, ()> {
                    #(let end = (#parts)?;)*
                    Ok(end)
                })()}})
            }
            RuleExpr::Choice(a, b) => {
                let ca = self.fused_pos_only_code(a, fields)?;
                let cb = self.fused_pos_only_code(b, fields)?;
                Ok(quote! {
                    match (#ca) {
                        Ok(end) => Ok(end),
                        Err(()) => #cb
                    }
                })
            }
            RuleExpr::Field(fref) => {
                let key = fref.key();
                let typ = fields.get(&key)
                    .ok_or_else(|| Error::new(proc_macro2::Span::call_site(),
                        format!("field '{}' not found in pos-only for '{}'", key, self.ident)))?;
                Ok(quote! {{
                    let __po_saved = ctx.tags.len();
                    let __po_r = <#typ as #CRATE::ParseImpl<0, ERROR>>::parse_impl(
                        input, end, if head { depth + 1 } else { 0 }, head && first, ctx
                    );
                    ctx.tags.truncate(__po_saved);
                    __po_r
                }})
            }

            RuleExpr::FieldRegex(fref, name) => {
                if let Some(sub_expr) = self.context.subrules.get(name).cloned() {
                    let key = fref.key();
                    let typ = fields.get(&key)
                        .ok_or_else(|| Error::new(proc_macro2::Span::call_site(),
                            format!("field '{}' not found in pos-only for '{}'", key, self.ident)))?;
                    let sub_fields = crate::builder::build_item_fields(typ);
                    self.fused_pos_only_code(&sub_expr, &sub_fields)
                } else if let Some(regex_pattern) = self.context.regexes.get(name).cloned() {
                    if let Some(inline) = try_inline_regex(&regex_pattern) {
                        Ok(inline)
                    } else {
                        Ok(quote! {{
                            static REGEX: #CRATE::LazyLock<#CRATE::Regex> = #CRATE::LazyLock::new(|| {
                                #CRATE::Regex::new(concat!("^(", #regex_pattern, ")")).unwrap()
                            });
                            Ok::<usize, ()>(end + REGEX.find(&input[end..]).map(|mat| mat.as_str().len()).ok_or(())?)
                        }})
                    }
                } else {
                    Err(Error::new(proc_macro2::Span::call_site(),
                        format!("unknown regex or subrule '{name}'")))
                }
            }

            RuleExpr::FieldTag(fref, tag) => {
                let key = fref.key();
                let typ = fields.get(&key)
                    .ok_or_else(|| Error::new(proc_macro2::Span::call_site(),
                        format!("field '{}' not found in pos-only for '{}'", key, self.ident)))?;
                let group = self.tag_index(tag)?;
                Ok(quote! {{
                    let __po_saved = ctx.tags.len();
                    let __po_r = <#typ as #CRATE::ParseImpl<#group, ERROR>>::parse_impl(
                        input, end, if head { depth + 1 } else { 0 }, head && first, ctx
                    );
                    ctx.tags.truncate(__po_saved);
                    __po_r
                }})
            }

            RuleExpr::FieldMulti(frefs, name) => {
                let sub_expr = self.context.subrules.get(name)
                    .ok_or_else(|| Error::new(proc_macro2::Span::call_site(),
                        format!("${{...}}:{name} requires a subrule")))?;
                let remapped = crate::builder::remap_subrule_fields(sub_expr, frefs);
                self.fused_pos_only_code(&remapped, fields)
            }

            RuleExpr::Rep(inner, kind) => {
                let inner_code = self.fused_pos_only_code(inner, fields)?;
                match kind {
                    RepKind::ZeroOrMore => Ok(quote! {{
                        let mut __po_end = end;
                        while let Ok(__po_e) = { let end = __po_end; #inner_code } {
                            if __po_e <= __po_end { break; }
                            __po_end = __po_e;
                        }
                        Ok::<usize, ()>(__po_end)
                    }}),
                    RepKind::OneOrMore => {
                        let inner_code2 = self.fused_pos_only_code(inner, fields)?;
                        Ok(quote! {{
                            let mut __po_end = { let end = end; (#inner_code)? };
                            while let Ok(__po_e) = { let end = __po_end; #inner_code2 } {
                                if __po_e <= __po_end { break; }
                                __po_end = __po_e;
                            }
                            Ok::<usize, ()>(__po_end)
                        }})
                    }
                    RepKind::Optional => Ok(quote! {{
                        match { let end = end; #inner_code } {
                            Ok(end) => Ok::<usize, ()>(end),
                            Err(()) => Ok::<usize, ()>(end),
                        }
                    }}),
                }
            }

            RuleExpr::SepRep { expr, sep, at_least_one } => {
                let item_code = self.fused_pos_only_code(expr, fields)?;
                let item_code2 = self.fused_pos_only_code(expr, fields)?;
                let sep_code = self.fused_pos_only_code(sep, fields)?;
                if *at_least_one {
                    Ok(quote! {{
                        let mut __po_end = { let end = end; (#item_code)? };
                        loop {
                            let __po_saved = __po_end;
                            let Ok(__po_sep_end) = ({ let end = __po_end; #sep_code }) else { break };
                            match { let end = __po_sep_end; #item_code2 } {
                                Ok(__po_e) if __po_e > __po_saved => { __po_end = __po_e; }
                                _ => { break }
                            }
                        }
                        Ok::<usize, ()>(__po_end)
                    }})
                } else {
                    Ok(quote! {{
                        let mut __po_end = end;
                        if let Ok(__po_e) = { let end = end; #item_code } {
                            __po_end = __po_e;
                            loop {
                                let __po_saved = __po_end;
                                let Ok(__po_sep_end) = ({ let end = __po_end; #sep_code }) else { break };
                                match { let end = __po_sep_end; #item_code2 } {
                                    Ok(__po_e) if __po_e > __po_saved => { __po_end = __po_e; }
                                    _ => { break }
                                }
                            }
                        }
                        Ok::<usize, ()>(__po_end)
                    }})
                }
            }

            RuleExpr::Not(inner) => {
                let code = self.fused_pos_only_code(inner, fields)?;
                Ok(quote! {
                    match (#code) {
                        Ok(_) => Err::<usize, ()>(()),
                        Err(()) => Ok::<usize, ()>(end),
                    }
                })
            }

            RuleExpr::And(inner) => {
                let code = self.fused_pos_only_code(inner, fields)?;
                Ok(quote! {
                    match (#code) {
                        Ok(_) => Ok::<usize, ()>(end),
                        Err(()) => Err::<usize, ()>(()),
                    }
                })
            }
        }
    }

    fn expr_may_construct(&self, expr: &RuleExpr) -> bool {
        match expr {
            RuleExpr::Literal(_) | RuleExpr::SubruleRef(_) => false,
            RuleExpr::Field(_) | RuleExpr::FieldTag(_, _) => true,
            RuleExpr::FieldRegex(_, _) | RuleExpr::FieldMulti(_, _) => true,
            RuleExpr::Seq(elems) => elems.iter().any(|e| self.expr_may_construct(e)),
            RuleExpr::Choice(a, b) => self.expr_may_construct(a) || self.expr_may_construct(b),
            RuleExpr::Rep(inner, _) | RuleExpr::SepRep { expr: inner, .. } => {
                let expanded = crate::builder::expand_subrule_refs(inner, &self.context.subrules);
                expanded.has_field_refs()
            }
            RuleExpr::Not(_) | RuleExpr::And(_) => false,
        }
    }

    fn expr_is_fused_atomic(&self, expr: &RuleExpr) -> bool {
        match expr {
            RuleExpr::Field(_) | RuleExpr::FieldTag(_, _) => true,
            RuleExpr::FieldRegex(_, name) => self.context.regexes.contains_key(name),
            RuleExpr::Literal(_) | RuleExpr::SubruleRef(_) => true,
            RuleExpr::Not(_) | RuleExpr::And(_) => true,
            RuleExpr::Choice(_, _) => true,
            RuleExpr::Rep(_, _) | RuleExpr::SepRep { .. } => true,
            RuleExpr::Seq(elems) => {
                if elems.is_empty() { return true; }
                for elem in &elems[..elems.len()-1] {
                    if self.expr_may_construct(elem) && !self.is_deferrable_field_regex(elem) {
                        return false;
                    }
                }
                self.expr_is_fused_atomic(elems.last().unwrap())
            }
            RuleExpr::FieldMulti(frefs, name) => {
                if let Some(sub_expr) = self.context.subrules.get(name) {
                    let remapped = crate::builder::remap_subrule_fields(sub_expr, frefs);
                    self.expr_is_fused_atomic(&remapped)
                } else {
                    false
                }
            }
        }
    }

    fn is_deferrable_field_regex(&self, expr: &RuleExpr) -> bool {
        matches!(expr, RuleExpr::FieldRegex(_, name) if self.context.regexes.contains_key(name))
    }

    fn seq_needs_deferral(&self, elems: &[RuleExpr]) -> bool {
        if elems.len() < 2 { return false; }
        let mut has_deferrable = false;
        for elem in &elems[..elems.len()-1] {
            if self.expr_may_construct(elem) {
                if self.is_deferrable_field_regex(elem) {
                    has_deferrable = true;
                } else {
                    return false;
                }
            }
        }
        has_deferrable && self.expr_is_fused_atomic(elems.last().unwrap())
    }

    fn fused_stmt_seq_deferred(&self, elems: &[RuleExpr], fields: &HashMap<String, Type>) -> Result<TokenStream> {
        let (_, with) = self.fused_generics();
        let mut stmts = TokenStream::new();
        let mut deferred = Vec::new();

        for elem in elems {
            if self.is_deferrable_field_regex(elem) {
                let RuleExpr::FieldRegex(fref, name) = elem else { unreachable!() };
                let key = fref.key();
                let typ = fields.get(&key)
                    .ok_or_else(|| Error::new(proc_macro2::Span::call_site(),
                        format!("field '{}' not found in deferred seq for '{}'", key, self.ident)))?;
                let var = format_ident!("__ff_{}", key);
                let start_var = format_ident!("__deferred_{}_start", key);
                let end_var = format_ident!("__deferred_{}_end", key);
                let regex_pattern = self.context.regexes.get(name).unwrap();

                if let Some(inline) = try_inline_regex(regex_pattern) {
                    stmts.extend(quote! {
                        let #start_var = end;
                        let end = {
                            if head && first { return Err(()); }
                            let end = (#inline)?;
                            head = false;
                            end
                        };
                        let #end_var = end;
                    });
                } else {
                    stmts.extend(quote! {
                        let #start_var = end;
                        let end = {
                            if head && first { return Err(()); }
                            static REGEX: #CRATE::LazyLock<#CRATE::Regex> = #CRATE::LazyLock::new(|| {
                                #CRATE::Regex::new(concat!("^(", #regex_pattern, ")")).unwrap()
                            });
                            let end = end + REGEX.find(&input[end..]).map(|mat| mat.as_str().len()).ok_or(())?;
                            head = false;
                            end
                        };
                        let #end_var = end;
                    });
                }

                deferred.push(quote! {
                    let #var = <#typ as #CRATE::FromStr<#with>>::from_str_with(
                        &input[#start_var..#end_var], extra
                    );
                });
            } else {
                stmts.extend(self.fused_stmt(elem, fields)?);
            }
        }

        for d in deferred {
            stmts.extend(d);
        }

        Ok(stmts)
    }

    fn fused_item_closure(&self, expr: &RuleExpr, sub_fields: &HashMap<String, Type>) -> Result<(TokenStream, TokenStream)> {
        let item_stmts = self.fused_stmt(expr, sub_fields)?;
        let mut sorted_keys: Vec<String> = sub_fields.keys().cloned().collect();
        sorted_keys.sort();
        let item_values: Vec<Ident> = sorted_keys.iter()
            .map(|k| format_ident!("__ff_{}", k))
            .collect();
        let item_expr = if item_values.len() == 1 {
            quote! { #(#item_values)* }
        } else {
            quote! { (#(#item_values),*) }
        };
        let closure = quote! {
            (|| -> Result<(usize, _), ()> {
                let mut head = true;
                #item_stmts
                Ok((end, #item_expr))
            })()
        };
        Ok((closure, item_expr.clone()))
    }

    fn fused_sep_pos_closure(&self, sep: &RuleExpr, fields: &HashMap<String, Type>) -> Result<TokenStream> {
        let sep_code = self.fused_pos_only_code(sep, fields)?;
        Ok(quote! {
            (|| -> Result<usize, ()> {
                let end = end;
                #sep_code
            })()
        })
    }

    fn fused_simple_rep(&self, inner: &RuleExpr, kind: RepKind, fields: &HashMap<String, Type>) -> Result<TokenStream> {
        let inner_code = self.fused_pos_only_code(inner, fields)?;
        match kind {
            RepKind::ZeroOrMore => Ok(quote! {
                let end = {
                    let mut end = end;
                    while let Ok(end_) = (#inner_code) {
                        if end_ <= end { break; }
                        end = end_;
                    }
                    end
                };
            }),
            RepKind::OneOrMore => {
                let inner_code2 = self.fused_pos_only_code(inner, fields)?;
                Ok(quote! {
                    let end = {
                        let mut end = (#inner_code)?;
                        while let Ok(end_) = (#inner_code2) {
                            if end_ <= end { break; }
                            end = end_;
                        }
                        end
                    };
                })
            }
            RepKind::Optional => Ok(quote! {
                let end = match (#inner_code) {
                    Ok(end) => end,
                    Err(()) => end,
                };
            }),
        }
    }

    fn fused_collection_rep(&self, inner: &RuleExpr, kind: RepKind, rep_key: &str, sub_fields: &HashMap<String, Type>, _outer_fields: &HashMap<String, Type>) -> Result<TokenStream> {
        let var = format_ident!("__ff_{}", rep_key);
        let atomic = self.expr_is_fused_atomic(inner);
        let (item_closure, _) = self.fused_item_closure(inner, sub_fields)?;

        match kind {
            RepKind::ZeroOrMore => {
                let (item_closure2, _) = self.fused_item_closure(inner, sub_fields)?;
                let body = quote! {
                    let Ok((end_, item)) = (#item_closure2) else { break };
                    if end_ <= end { break; }
                    __items.push(item);
                    end = end_;
                };
                let loop_code = if atomic {
                    quote! { loop { #body } }
                } else {
                    let item_probe = self.fused_pos_only_code(inner, sub_fields)?;
                    quote! { while (#item_probe).is_ok() { #body } }
                };
                Ok(quote! {
                    let (end, #var) = {
                        let mut __items = Vec::new();
                        let mut end = end;
                        #loop_code
                        (end, __items)
                    };
                })
            }
            RepKind::OneOrMore => {
                let (item_closure2, _) = self.fused_item_closure(inner, sub_fields)?;
                let body = quote! {
                    let Ok((end_, item)) = (#item_closure2) else { break };
                    if end_ <= end { break; }
                    __items.push(item);
                    end = end_;
                };
                let loop_code = if atomic {
                    quote! { loop { #body } }
                } else {
                    let item_probe = self.fused_pos_only_code(inner, sub_fields)?;
                    quote! { while (#item_probe).is_ok() { #body } }
                };
                Ok(quote! {
                    let (end, #var) = {
                        let mut __items = Vec::new();
                        let (mut end, item) = (#item_closure)?;
                        __items.push(item);
                        #loop_code
                        (end, __items)
                    };
                })
            }
            RepKind::Optional => {
                if atomic {
                    Ok(quote! {
                        let (end, #var) = match (#item_closure) {
                            Ok((end, item)) => (end, Some(item)),
                            Err(()) => (end, None),
                        };
                    })
                } else {
                    let item_probe = self.fused_pos_only_code(inner, sub_fields)?;
                    Ok(quote! {
                        let (end, #var) = if (#item_probe).is_ok() {
                            match (#item_closure) {
                                Ok((end, item)) => (end, Some(item)),
                                Err(()) => (end, None),
                            }
                        } else {
                            (end, None)
                        };
                    })
                }
            }
        }
    }

    fn fused_sep_rep(&self, expr: &RuleExpr, sep: &RuleExpr, at_least_one: bool, rep_key: &str, sub_fields: &HashMap<String, Type>, outer_fields: &HashMap<String, Type>) -> Result<TokenStream> {
        let var = format_ident!("__ff_{}", rep_key);
        let atomic = self.expr_is_fused_atomic(expr);
        let (item_closure, _) = self.fused_item_closure(expr, sub_fields)?;
        let (item_closure2, _) = self.fused_item_closure(expr, sub_fields)?;
        let sep_closure = self.fused_sep_pos_closure(sep, outer_fields)?;

        let loop_body = if atomic {
            quote! {
                let __saved_end = end;
                let Ok(__end_sep) = (#sep_closure) else { break };
                end = __end_sep;
                match (#item_closure2) {
                    Ok((__end_item, __item)) if __end_item > __saved_end => {
                        __items.push(__item);
                        end = __end_item;
                    }
                    _ => { end = __saved_end; break }
                }
            }
        } else {
            let loop_item_probe = self.fused_pos_only_code(expr, sub_fields)?;
            quote! {
                let __saved_end = end;
                let Ok(__end_sep) = (#sep_closure) else { break };
                end = __end_sep;
                if !(#loop_item_probe).is_ok() { end = __saved_end; break; }
                match (#item_closure2) {
                    Ok((__end_item, __item)) if __end_item > __saved_end => {
                        __items.push(__item);
                        end = __end_item;
                    }
                    _ => { end = __saved_end; break }
                }
            }
        };

        if at_least_one {
            Ok(quote! {
                let (end, #var) = {
                    let mut __items = Vec::new();
                    let (mut end, __item) = (#item_closure)?;
                    __items.push(__item);
                    loop { #loop_body }
                    (end, __items)
                };
            })
        } else {
            let first_check = if atomic {
                quote! {
                    if let Ok((__end_first, __item)) = (#item_closure) {
                        __items.push(__item);
                        end = __end_first;
                        loop { #loop_body }
                    }
                }
            } else {
                let first_item_probe = self.fused_pos_only_code(expr, sub_fields)?;
                quote! {
                    if (#first_item_probe).is_ok() {
                        if let Ok((__end_first, __item)) = (#item_closure) {
                            __items.push(__item);
                            end = __end_first;
                            loop { #loop_body }
                        }
                    }
                }
            };
            Ok(quote! {
                let (end, #var) = {
                    let mut __items = Vec::new();
                    let mut end = end;
                    #first_check
                    (end, __items)
                };
            })
        }
    }

    fn fused_simple_sep_rep(&self, expr: &RuleExpr, sep: &RuleExpr, at_least_one: bool, fields: &HashMap<String, Type>) -> Result<TokenStream> {
        let item_code = self.fused_pos_only_code(expr, fields)?;
        let item_code2 = self.fused_pos_only_code(expr, fields)?;
        let sep_code = self.fused_pos_only_code(sep, fields)?;

        if at_least_one {
            Ok(quote! {
                let end = {
                    let mut end = (#item_code)?;
                    loop {
                        let __saved_end = end;
                        let Ok(__end_sep) = (#sep_code) else { break };
                        end = __end_sep;
                        match (#item_code2) {
                            Ok(end_) if end_ > __saved_end => { end = end_; }
                            _ => { end = __saved_end; break }
                        }
                    }
                    end
                };
            })
        } else {
            Ok(quote! {
                let end = {
                    let mut end = end;
                    if let Ok(end_) = (#item_code) {
                        end = end_;
                        loop {
                            let __saved_end = end;
                            let Ok(__end_sep) = (#sep_code) else { break };
                            end = __end_sep;
                            match (#item_code2) {
                                Ok(end_) if end_ > __saved_end => { end = end_; }
                                _ => { end = __saved_end; break }
                            }
                        }
                    }
                    end
                };
            })
        }
    }
}

fn collect_fields_from_expr(
    expr: &RuleExpr,
    inside_rep: bool,
    variant_fields: &HashMap<String, syn::Type>,
) -> Vec<(String, FieldKind)> {
    match expr {
        RuleExpr::Field(f) | RuleExpr::FieldRegex(f, _) | RuleExpr::FieldTag(f, _) => {
            if inside_rep { vec![] }
            else { vec![(f.key(), FieldKind::Value)] }
        }
        RuleExpr::FieldMulti(frefs, _) => {
            if inside_rep { vec![] }
            else { frefs.iter().map(|f| (f.key(), FieldKind::Value)).collect() }
        }
        RuleExpr::SubruleRef(_) | RuleExpr::Literal(_) => vec![],
        RuleExpr::Seq(elems) => {
            elems.iter().flat_map(|e| collect_fields_from_expr(e, inside_rep, variant_fields)).collect()
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
            } else { vec![] }
        }
        RuleExpr::Not(_) | RuleExpr::And(_) => vec![],
    }
}
