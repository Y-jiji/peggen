use crate::*;

impl Builder {
    pub fn parse_impl_build(&self) -> Result<TokenStream> {
        let mut impls = TokenStream::new();
        let r#impl = |group, ident, generics, body, patt| quote! {
            impl<#generics const ERROR: bool> #CRATE::ParseImpl<#group, ERROR> for #ident<#generics> {
                fn parse_impl(
                    input: &str, end: usize,        // input[end..] represents the unparsed source
                    depth: usize,                   // left recursion depth
                    first: bool,                    // whether stack top is considered a token
                    trace: &mut Vec<usize>,         // non-terminal symbols 
                    stack: &mut Vec<#CRATE::Tag>,   // stack of suffix code
                ) -> Result<usize, ()> {
                    // the symbol signature
                    let symb = <Self as #CRATE::Num>::num(0);
                    // if it is the matched first element
                    if first && stack.last().map(|tag| tag.rule >= symb && matches!(tag.rule - symb, #patt)).unwrap_or(false) {
                        return Ok(stack.last().map(|tag| tag.span.end).unwrap());
                    }
                    // if left recursion happened (amortized O(n*n), n = the number of symbols)
                    for &node in &trace[trace.len().max(depth)-depth..] {
                        if node == symb + #group { Err(())? }
                    }
                    // forbid symb + #group on the first element
                    trace.push(symb + #group);
                    let start = end;
                    let end = #body;
                    trace.pop();
                    // from here, we consider previously matched results as the first token. 
                    let mut end = end?;
                    let first = true;
                    // grow the results
                    loop {match {let end = start; #body} {
                        Ok(end_) if end_ > end => { end = end_; continue }
                        _ => { break }
                    }};
                    Ok(end)
                }
            }
        };
        for group in 0..=self.group {
            let body = self.parse_impl_group(group)?;
            let patt = self.parse_patt_group(group)?;
            impls.extend(r#impl(group, &self.ident, &self.generics, body, patt));
        };
        Ok(impls)
    }
    fn parse_patt_group(&self, group: usize) -> Result<TokenStream> {
        let rule = self.rules.iter().enumerate()
            .filter(|(_, rule)| rule.group >= group)
            .map(|(num, _)| num);
        Ok(quote! { #(#rule)|* })
    }
    fn parse_impl_group(&self, group: usize) -> Result<TokenStream> {
        let rule = self.rules.iter().enumerate()
            .filter(|(_, rule)| rule.group >= group)
            .map(|(num, rule)| (num, rule.error))
            .map(|(num, error)| quote! {
                // if this rule is marked as an error handler, but currently we are not parsing in error mode
                if #error && !ERROR { Err(()) }
                // proceed normally
                else { <Self as #CRATE::RuleImpl<#num, ERROR>>::rule_impl(input, end, depth, first, trace, stack) }
            });
        Ok(quote! {{(|| -> Result<usize, ()> {
            #(match #rule {
                // when any of the rule matches, choose it as the result
                Ok(end) => {return Ok(end)}
                // otherwise proceed to the next rule
                Err(()) => {}
            };)*
            // when no rule matches
            Err(())
        })()}})
    }
}