use crate::*;

impl Builder {
    pub fn parse_impl_build(&self) -> Result<TokenStream> {
        let mut impls = TokenStream::new();
        let r#impl = |group, ident, generics, body, patt| quote! {
            impl<#generics const ERROR: bool> #CRATE::ParseImpl<#group, ERROR> for #ident<#generics> {
                fn parse_impl(
                    input: &str, end: usize,
                    depth: usize,
                    first: bool,
                    trace: &mut Vec<usize>,
                    stack: &mut Vec<#CRATE::Tag>,
                ) -> Result<usize, ()> {
                    let symb = <Self as #CRATE::Num>::num(0);
                    if first && stack.last().map(|tag| tag.rule >= symb && matches!(tag.rule - symb, #patt)).unwrap_or(false) {
                        return Ok(stack.last().map(|tag| tag.span.end).unwrap());
                    }
                    for &node in &trace[trace.len().max(depth)-depth..] {
                        if node == symb + #group { Err(())? }
                    }
                    trace.push(symb + #group);
                    let start = end;
                    let end = #body;
                    trace.pop();
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
                let body = self.parse_impl_group_legacy(group)?;
                let patt = self.parse_patt_group_legacy(group)?;
                impls.extend(r#impl(group, &self.ident, &self.generics, body, patt));
            }
        } else {
            for (tag_idx, _tag_name) in self.all_tags.iter().enumerate() {
                let body = self.parse_impl_group_tagged(tag_idx)?;
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

    fn parse_impl_group_legacy(&self, group: usize) -> Result<TokenStream> {
        let rule = self.rules.iter().enumerate()
            .filter(|(_, rule)| rule.group >= group)
            .map(|(num, rule)| (num, rule.error))
            .map(|(num, error)| quote! {
                if #error && !ERROR { Err(()) }
                else { <Self as #CRATE::RuleImpl<#num, ERROR>>::rule_impl(input, end, depth, first, trace, stack) }
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

    fn parse_impl_group_tagged(&self, tag_idx: usize) -> Result<TokenStream> {
        let tag_name = &self.all_tags[tag_idx];
        let rule = self.rules.iter().enumerate()
            .filter(|(_, rule)| rule.tags.contains(tag_name))
            .map(|(num, rule)| (num, rule.error))
            .map(|(num, error)| quote! {
                if #error && !ERROR { Err(()) }
                else { <Self as #CRATE::RuleImpl<#num, ERROR>>::rule_impl(input, end, depth, first, trace, stack) }
            });
        Ok(quote! {{(|| -> Result<usize, ()> {
            #(match #rule {
                Ok(end) => {return Ok(end)}
                Err(()) => {}
            };)*
            Err(())
        })()}})
    }
}
