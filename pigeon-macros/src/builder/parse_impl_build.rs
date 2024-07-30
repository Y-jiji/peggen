use crate::*;

pub trait ParserImplBuild {
    fn parse_impl_build(&self) -> Result<TokenStream>;
}

impl ParserImplBuild for Builder {
    fn parse_impl_build(&self) -> Result<TokenStream> {
        let mut output = TokenStream::new();
        for group in 0..=self.group {
            let mut body = TokenStream::new();
            let _crate = parse_str::<Ident>(CRATE).unwrap();
            for (num, rule) in self.rules.iter().enumerate() {
                if rule.group < group { continue; }
                let opt = quote! {
                    // Each rule either succeeds or don't change
                    if let Ok(end) = <Self as #_crate::RuleImpl<#num, ERROR>>::rule_impl(input, end, last, trace, stack) {
                        last = end;
                        continue;
                    };
                };
                let opt = if rule.error {
                    quote! { if ERROR { #opt } }
                } else {
                    quote! { #opt }
                };
                body.extend(quote! { #opt });
            }
            let this = &self.ident;
            let generics = &self.generics.params;
            output.extend(quote! {
                impl<#generics const ERROR: bool> #_crate::ParseImpl<#group, ERROR> for #this<#generics> {
                    fn parse_impl(
                        input: &str, end: usize,
                        trace: &mut Vec<(usize, usize)>,
                        stack: &mut Vec<#_crate::Tag>,
                    ) -> Result<usize, ()> {
                        let mut last = end;
                        if stack.last().map(|top| top.span.start == end).unwrap_or(false) {
                            return Ok(stack.last().unwrap().span.end);
                        }
                        for &(begin, symb) in trace.iter().rev() {
                            if begin < end { break }
                            if symb != <Self as #_crate::Num>::num(#group) { continue }
                            Err(())?
                        }
                        trace.push((end, <Self as #_crate::Num>::num(#group)));
                        loop { #body; break }
                        trace.pop();
                        if last != end { Ok(last) }
                        else           { Err(())  }
                    }
                }
            })
        }
        Ok(output)
    }
}