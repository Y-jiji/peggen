use std::collections::HashMap;
use crate::*;
use crate::rule_ast::*;
use crate::builder::first_set::*;

fn is_infix_rule(rule: &Rule) -> bool {
    match &rule.body {
        RuleExpr::Seq(elems) if !elems.is_empty() => {
            matches!(&elems[0], RuleExpr::FieldTag(..))
        }
        _ => false,
    }
}

fn body_references_tag(expr: &RuleExpr, tag_name: &str) -> bool {
    match expr {
        RuleExpr::FieldTag(_, tag) => tag == tag_name,
        RuleExpr::Seq(elems) => elems.iter().any(|e| body_references_tag(e, tag_name)),
        RuleExpr::Choice(a, b) => body_references_tag(a, tag_name) || body_references_tag(b, tag_name),
        RuleExpr::Rep(e, _) => body_references_tag(e, tag_name),
        RuleExpr::SepRep { expr, sep, .. } => body_references_tag(expr, tag_name) || body_references_tag(sep, tag_name),
        RuleExpr::Not(e) | RuleExpr::And(e) => body_references_tag(e, tag_name),
        _ => false,
    }
}

fn infix_left_tag(rule: &Rule) -> Option<&str> {
    match &rule.body {
        RuleExpr::Seq(elems) if !elems.is_empty() => {
            match &elems[0] {
                RuleExpr::FieldTag(_, tag) => Some(tag.as_str()),
                _ => None,
            }
        }
        _ => None,
    }
}

impl Builder {
    pub fn parse_impl_build(&self) -> Result<TokenStream> {
        self.parse_impl_build_with(&ImplMode::normal())
    }

    pub fn ref_parse_impl_build(&self) -> Result<TokenStream> {
        self.parse_impl_build_with(&ImplMode::reference())
    }

    fn parse_impl_build_with(&self, mode: &ImplMode) -> Result<TokenStream> {
        let mut impls = TokenStream::new();
        let parse_trait = &mode.parse_trait;
        let parse_method = &mode.parse_method;
        let ident = &self.ident;
        let generics = &self.generics;

        let build_impl = |group: usize, initial_body: TokenStream, loop_body: Option<TokenStream>, patt: TokenStream, needs_guard: bool| {
            let loop_code = match &loop_body {
                Some(lb) => quote! {
                    let first = true;
                    loop {match {let end = start; #lb} {
                        Ok(end_) if end_ > end => { end = end_; continue }
                        _ => { break }
                    }};
                },
                None => quote! {},
            };
            let has_loop = loop_body.is_some();
            let guard_push = if needs_guard { quote! {
                for &node in &ctx.trace[ctx.trace.len().max(depth)-depth..] {
                    if node == symb + #group { Err(())? }
                }
                ctx.trace.push(symb + #group);
            }} else { quote! {} };
            let guard_pop = if needs_guard { quote! { ctx.trace.pop(); } } else { quote! {} };
            let start_bind = if has_loop { quote! { let start = end; } } else { quote! {} };
            quote! {
                impl<#generics const ERROR: bool> #parse_trait<#group, ERROR> for #ident<#generics> {
                    fn #parse_method(
                        input: &str, end: usize,
                        depth: usize,
                        first: bool,
                        ctx: &mut #CRATE::ParseContext,
                    ) -> Result<usize, ()> {
                        let symb = <Self as #CRATE::Num>::num(0);
                        if first && ctx.tags.last().map(|tag| tag.rule >= symb && matches!(tag.rule - symb, #patt)).unwrap_or(false) {
                            return Ok(ctx.tags.last().map(|tag| tag.span.end).unwrap());
                        }
                        #guard_push
                        #start_bind
                        let end = #initial_body;
                        #guard_pop
                        let mut end = end?;
                        #loop_code
                        Ok(end)
                    }
                }
            }
        };

        if self.all_tags.is_empty() {
            for group in 0..=self.group {
                let body = if mode.optimized {
                    self.parse_impl_group_dispatch(group, mode)?
                } else {
                    self.parse_impl_group_legacy(group, mode)?
                };
                let patt = self.parse_patt_group_legacy(group)?;
                impls.extend(build_impl(group, body.clone(), Some(body), patt, true));
            }
        } else {
            for (tag_idx, tag_name) in self.all_tags.iter().enumerate() {
                let patt = self.parse_patt_group_tagged(tag_idx)?;
                let needs_guard = self.rules.iter()
                    .filter(|rule| rule.tags.contains(tag_name))
                    .any(|rule| body_references_tag(&rule.body, tag_name));
                if mode.optimized {
                    let (initial_body, loop_body) = self.parse_impl_group_tagged_split(tag_idx, mode)?;
                    impls.extend(build_impl(tag_idx, initial_body, loop_body, patt, needs_guard));
                } else {
                    let body = self.parse_impl_group_tagged(tag_idx, mode)?;
                    impls.extend(build_impl(tag_idx, body.clone(), Some(body), patt, needs_guard));
                }
            }
        }
        Ok(impls)
    }

    fn parse_patt_group_legacy(&self, group: usize) -> Result<TokenStream> {
        let rule = self.rules.iter().enumerate()
            .filter(|(_, rule)| rule.group >= group)
            .map(|(num, _)| num);
        Ok(quote! { #(#rule)|* })
    }

    fn parse_impl_group_legacy(&self, group: usize, mode: &ImplMode) -> Result<TokenStream> {
        let rule_trait = &mode.rule_trait;
        let rule_method = &mode.rule_method;
        let rule = self.rules.iter().enumerate()
            .filter(|(_, rule)| rule.group >= group)
            .map(|(num, rule)| (num, rule.error))
            .map(|(num, error)| quote! {
                if #error && !ERROR { Err(()) }
                else { <Self as #rule_trait<#num, ERROR>>::#rule_method(input, end, depth, first, ctx) }
            });
        Ok(quote! {{(|| -> Result<usize, ()> {
            #(match #rule {
                Ok(end) => {return Ok(end)}
                Err(()) => {}
            };)*
            Err(())
        })()}})
    }

    fn parse_patt_group_tagged(&self, tag_idx: usize) -> Result<TokenStream> {
        let tag_name = &self.all_tags[tag_idx];
        let rule = self.rules.iter().enumerate()
            .filter(|(_, rule)| rule.tags.contains(tag_name))
            .map(|(num, _)| num);
        Ok(quote! { #(#rule)|* })
    }

    fn parse_impl_group_tagged(&self, tag_idx: usize, mode: &ImplMode) -> Result<TokenStream> {
        let rule_trait = &mode.rule_trait;
        let rule_method = &mode.rule_method;
        let tag_name = &self.all_tags[tag_idx];
        let rule = self.rules.iter().enumerate()
            .filter(|(_, rule)| rule.tags.contains(tag_name))
            .map(|(num, rule)| (num, rule.error))
            .map(|(num, error)| quote! {
                if #error && !ERROR { Err(()) }
                else { <Self as #rule_trait<#num, ERROR>>::#rule_method(input, end, depth, first, ctx) }
            });
        Ok(quote! {{(|| -> Result<usize, ()> {
            #(match #rule {
                Ok(end) => {return Ok(end)}
                Err(()) => {}
            };)*
            Err(())
        })()}})
    }

    fn parse_impl_group_dispatch(&self, group: usize, mode: &ImplMode) -> Result<TokenStream> {
        let rules: Vec<(usize, &Rule)> = self.rules.iter().enumerate()
            .filter(|(_, rule)| rule.group >= group)
            .collect();
        self.build_dispatch_body(&rules, mode)
    }

    fn parse_impl_group_tagged_dispatch(&self, tag_idx: usize, mode: &ImplMode) -> Result<TokenStream> {
        let tag_name = &self.all_tags[tag_idx];
        let rules: Vec<(usize, &Rule)> = self.rules.iter().enumerate()
            .filter(|(_, rule)| rule.tags.contains(tag_name))
            .collect();
        self.build_dispatch_body(&rules, mode)
    }

    fn parse_impl_group_tagged_split(&self, tag_idx: usize, mode: &ImplMode) -> Result<(TokenStream, Option<TokenStream>)> {
        let tag_name = &self.all_tags[tag_idx];
        let all_rules: Vec<(usize, &Rule)> = self.rules.iter().enumerate()
            .filter(|(_, rule)| rule.tags.contains(tag_name))
            .collect();

        let primary_rules: Vec<(usize, &Rule)> = all_rules.iter()
            .filter(|(_, rule)| !is_infix_rule(rule))
            .cloned()
            .collect();
        let infix_rules: Vec<(usize, &Rule)> = all_rules.iter()
            .filter(|(_, rule)| is_infix_rule(rule))
            .cloned()
            .collect();

        let initial_body = if primary_rules.is_empty() {
            quote! { Err(()) }
        } else {
            self.build_dispatch_body(&primary_rules, mode)?
        };

        let loop_body = if infix_rules.is_empty() {
            None
        } else {
            Some(self.build_infix_loop_body(&infix_rules, mode)?)
        };

        Ok((initial_body, loop_body))
    }

    fn build_infix_loop_body(&self, infix_rules: &[(usize, &Rule)], mode: &ImplMode) -> Result<TokenStream> {
        let rule_trait = &mode.rule_trait;
        let rule_method = &mode.rule_method;

        let mut blocks: Vec<(&str, Vec<(usize, bool)>)> = Vec::new();
        for &(num, rule) in infix_rules {
            let left_tag = infix_left_tag(rule).unwrap();
            if let Some(last) = blocks.last_mut() {
                if last.0 == left_tag {
                    last.1.push((num, rule.error));
                    continue;
                }
            }
            blocks.push((left_tag, vec![(num, rule.error)]));
        }

        let mut block_codes = Vec::new();
        for (left_tag, block_rules) in &blocks {
            let tag_idx = self.tag_index(left_tag)?;
            let tag_name = &self.all_tags[tag_idx];
            let valid_rules: Vec<usize> = self.rules.iter().enumerate()
                .filter(|(_, rule)| rule.tags.contains(tag_name))
                .map(|(num, _)| num)
                .collect();

            let rule_calls: Vec<TokenStream> = block_rules.iter()
                .map(|(num, error)| {
                    let num = *num;
                    let error = *error;
                    quote! {
                        match (if #error && !ERROR { Err(()) }
                               else { <Self as #rule_trait<#num, ERROR>>::#rule_method(input, end, depth, first, ctx) }) {
                            Ok(end) => { return Ok(end) }
                            Err(()) => {}
                        }
                    }
                }).collect();

            block_codes.push(quote! {
                if matches!(__last_rule, #(#valid_rules)|*) {
                    #(#rule_calls)*
                }
            });
        }

        Ok(quote! {{(|| -> Result<usize, ()> {
            let __last_rule = ctx.tags.last().map(|tag| {
                if tag.rule >= symb { tag.rule - symb } else { usize::MAX }
            }).unwrap_or(usize::MAX);
            #(#block_codes)*
            Err(())
        })()}})
    }

    fn build_dispatch_body(&self, rules: &[(usize, &Rule)], mode: &ImplMode) -> Result<TokenStream> {
        let rule_trait = &mode.rule_trait;
        let rule_method = &mode.rule_method;

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
                FirstSet::Unknown => {
                    wildcard_indices.push(i);
                }
            }
        }

        if known_bytes.is_empty() {
            let rule_calls: Vec<TokenStream> = rules.iter()
                .map(|(num, rule)| {
                    let num = *num;
                    let error = rule.error;
                    quote! {
                        if #error && !ERROR { Err(()) }
                        else { <Self as #rule_trait<#num, ERROR>>::#rule_method(input, end, depth, first, ctx) }
                    }
                }).collect();
            return Ok(quote! {{(|| -> Result<usize, ()> {
                #(match #rule_calls {
                    Ok(end) => {return Ok(end)}
                    Err(()) => {}
                };)*
                Err(())
            })()}});
        }

        let make_call = |idx: usize| -> TokenStream {
            let (num, rule) = &rules[idx];
            let num = *num;
            let error = rule.error;
            quote! {
                match (if #error && !ERROR { Err(()) }
                       else { <Self as #rule_trait<#num, ERROR>>::#rule_method(input, end, depth, first, ctx) }) {
                    Ok(end) => { return Ok(end) }
                    Err(()) => {}
                }
            }
        };

        // For each byte, determine which rules to try (in original order).
        // A rule is included if: (a) it's a wildcard, or (b) its FIRST set contains the byte.
        let mut byte_to_ordered_rules: HashMap<u8, Vec<usize>> = HashMap::new();
        for &b in &known_bytes {
            let mut indices: Vec<usize> = Vec::new();
            for i in 0..rules.len() {
                let dominated = wildcard_indices.contains(&i)
                    || known_indices.iter().any(|(ki, kb)| *ki == i && kb.contains(&b));
                if dominated { indices.push(i); }
            }
            byte_to_ordered_rules.insert(b, indices);
        }

        // Wildcard-only fallback (bytes not in any known FIRST set)
        let wildcard_only_calls: Vec<TokenStream> = wildcard_indices.iter()
            .map(|&idx| make_call(idx)).collect();

        // Group bytes with identical rule lists to merge match arms
        let mut group_map: HashMap<Vec<usize>, Vec<u8>> = HashMap::new();
        for (&byte, rule_list) in &byte_to_ordered_rules {
            group_map.entry(rule_list.clone()).or_default().push(byte);
        }

        // Check if all dispatch arms are identical to the wildcard-only fallback
        let all_same_as_wildcard = group_map.values().all(|_| {
            group_map.keys().all(|k| {
                k.len() == wildcard_indices.len()
                    && k.iter().zip(wildcard_indices.iter()).all(|(a, b)| a == b)
            })
        });

        if all_same_as_wildcard {
            let rule_calls: Vec<TokenStream> = rules.iter()
                .map(|(num, rule)| {
                    let num = *num;
                    let error = rule.error;
                    quote! {
                        if #error && !ERROR { Err(()) }
                        else { <Self as #rule_trait<#num, ERROR>>::#rule_method(input, end, depth, first, ctx) }
                    }
                }).collect();
            return Ok(quote! {{(|| -> Result<usize, ()> {
                #(match #rule_calls {
                    Ok(end) => {return Ok(end)}
                    Err(()) => {}
                };)*
                Err(())
            })()}});
        }

        let mut match_arms = Vec::new();
        for (rule_list, mut bytes) in group_map {
            bytes.sort();
            let byte_patterns = compress_byte_ranges(&bytes);
            let calls: Vec<TokenStream> = rule_list.iter().map(|&idx| make_call(idx)).collect();
            match_arms.push(quote! {
                #(Some(#byte_patterns))|* => { #(#calls)* }
            });
        }

        Ok(quote! {{(|| -> Result<usize, ()> {
            match input.as_bytes().get(end) {
                #(#match_arms)*
                _ => { #(#wildcard_only_calls)* }
            }
            Err(())
        })()}})
    }
}

pub(crate) fn compress_byte_ranges(bytes: &[u8]) -> Vec<TokenStream> {
    if bytes.is_empty() { return vec![]; }
    let mut patterns = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let start = bytes[i];
        let mut end = start;
        while i + 1 < bytes.len() && bytes[i + 1] == end + 1 {
            end = bytes[i + 1];
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
