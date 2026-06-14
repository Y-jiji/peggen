use std::collections::HashMap;
use crate::*;
use crate::builder::first_set::*;

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
        let r#impl = |group, ident, generics, body, patt| quote! {
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
                    for &node in &ctx.trace[ctx.trace.len().max(depth)-depth..] {
                        if node == symb + #group { Err(())? }
                    }
                    ctx.trace.push(symb + #group);
                    let start = end;
                    let end = #body;
                    ctx.trace.pop();
                    let mut end = end?;
                    let first = true;
                    loop {match {let end = start; #body} {
                        Ok(end_) if end_ > end => { end = end_; continue }
                        _ => { break }
                    }};
                    Ok(end)
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
                impls.extend(r#impl(group, &self.ident, &self.generics, body, patt));
            }
        } else {
            for (tag_idx, _tag_name) in self.all_tags.iter().enumerate() {
                let body = if mode.optimized {
                    self.parse_impl_group_tagged_dispatch(tag_idx, mode)?
                } else {
                    self.parse_impl_group_tagged(tag_idx, mode)?
                };
                let patt = self.parse_patt_group_tagged(tag_idx)?;
                impls.extend(r#impl(tag_idx, &self.ident, &self.generics, body, patt));
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

fn compress_byte_ranges(bytes: &[u8]) -> Vec<TokenStream> {
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
