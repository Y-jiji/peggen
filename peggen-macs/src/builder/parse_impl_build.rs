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
                if let Ok(end_) = <Self as #_crate::RuleImpl<#num, ERROR>>::rule_impl(input, begin, trace, stack) {
                    let grow = end_ > end && trace.last().map(|(_, _, leftrec)| *leftrec).unwrap();
                    end = end_;
                    trace.last_mut().map(|(_, _, leftrec)| *leftrec = false);
                    if grow { continue }
                    else    { break true }
                }
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
        let patt = self.rules.iter().enumerate().filter(|(_, rule)| rule.group >= group).map(|(num, _)| num);
        let patt = patt.fold(None, |x, y| {
            if let Some(x) = x { Some(quote! { #x | #y }) }
            else { Some(quote! { #y }) }
        }).ok_or(syn::Error::new_spanned(&self.ident, &format!("no element in group {group}")))?;
        println!("{:?}", patt.to_string());
        Ok(quote! {
            impl<#generics const ERROR: bool> #_crate::ParseImpl<#group, ERROR> for #this<#generics> {
                fn parse_impl(
                    input: &str, mut end: usize,
                    trace: &mut Vec<(usize, usize, bool)>,
                    stack: &mut Vec<#_crate::Tag>,
                ) -> Result<usize, ()> {
                    #_crate::stacker::maybe_grow(32*1024, 1024*1024, || {
                        // if this is cached && rule is in group
                        if stack.last().map(|top| 
                            top.span.start == end && 
                            matches!(top.rule, #patt)
                        ).unwrap_or(false) {
                            return Ok(stack.last().unwrap().span.end);
                        }
                        // if find a symbol at current position on the path, incur recursion error
                        for (begin, symb, leftrec) in trace.iter_mut().rev() {
                            if *begin < end { break }
                            if *symb != <Self as #_crate::Num>::num(#group) { continue }
                            *leftrec = true;
                            Err(())?
                        }
                        let begin = end;
                        // Try each rule repeatedly until nothing new occurs
                        trace.push((end, <Self as #_crate::Num>::num(#group), false));
                        let ok = loop {
                            println!("LOOP\t{}", stringify!(#this));
                            #body;
                            break false
                        };
                        trace.pop();
                        if ok { Ok(end) }
                        else  { Err(()) }
                    })
                }
            }
        })
    }
}