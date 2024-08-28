use quote::ToTokens;

use crate::*;

pub trait ParserImplBuild {
    fn parse_impl_build(&self) -> Result<TokenStream>;
    fn parse_impl_group(&self, group: usize) -> Result<TokenStream>;
}

impl ParserImplBuild for Builder {
    fn parse_impl_build(&self) -> Result<TokenStream> {
        let mut output = TokenStream::new();
        for group in 0..=self.group {
            output.extend(self.parse_impl_group(group));
        }
        Ok(output)
    }
    fn parse_impl_group(&self, group: usize) -> Result<TokenStream> {
        let mut body = TokenStream::new();
        let _crate = parse_str::<Ident>(CRATE).unwrap();
        for (num, rule) in self.rules.iter().enumerate() {
            if rule.group < group { continue; }
            let opt = quote! {
                // Each rule either succeeds or don't proceed
                if let Ok(end) = <Self as #_crate::RuleImpl<#num, ERROR>>::rule_impl(input, end, trace, stack) {
                    if last == end {
                        break;
                    } else {
                        last = end;
                        continue;
                    }
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
        let comma = generics.to_token_stream().into_iter().last().map(|x| x.to_string() == ",").unwrap_or(false);
        let generics = 
            if !comma && !generics.is_empty() { quote! { #generics, } }
            else                              { quote! { #generics  } };
        Ok(quote! {
            impl<#generics const ERROR: bool> #_crate::ParseImpl<#group, ERROR> for #this<#generics> {
                fn parse_impl(
                    input: &str, end: usize,
                    trace: &mut Vec<(usize, usize)>,
                    stack: &mut Vec<#_crate::Tag>,
                ) -> Result<usize, ()> {
                    #_crate::stacker::maybe_grow(32*1024, 1024*1024, || {
                        let mut last = end;
                        // if find a symbol at current position on the path, incur recursion error
                        for &(begin, symb) in trace.iter().rev() {
                            if begin < end { break }
                            if symb != <Self as #_crate::Num>::num(#group) { continue }
                            Err(())?
                        }
                        // Try each rule repeatedly until nothing new occurs
                        // This should happen on each rule, not each symbol
                        trace.push((end, <Self as #_crate::Num>::num(#group)));
                        loop { #body; break }
                        trace.pop();
                        if last != end { Ok(last) }
                        else           { Err(())  }
                    })
                }
            }
        })
    }
}