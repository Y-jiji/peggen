use crate::*;

impl Builder {
    // build the rule trait(s) for the given type
    pub fn rules_impl_build(&self) -> Result<TokenStream> {
        let mut impls = TokenStream::new();
        let r#impl = |num, ident, generics, variant, trace, body| {
            let (trace_start, trace_end_ok, trace_end_err) = 
                if trace { (
                    quote!{println!("START\t{}::{}\t{}", stringify!(#ident), stringify!(#variant), &input[end..]);}, 
                    quote!{println!("ERR\t{}::{}", stringify!(#ident), stringify!(#variant)); },
                    quote!{println!("OK\t{}::{}\t{}", stringify!(#ident), stringify!(#variant), &input[start..end]);}
                ) }
                else { (quote!{}, quote!{}, quote!{}) };
            quote! {
                impl<#generics const ERROR: bool> #CRATE::RuleImpl<#num, ERROR> for #ident<#generics> {
                    #[inline(always)]
                    fn rule_impl(
                        input: &str, end: usize,        // input[end..] represents the unparsed source
                        depth: usize,                   // left recursion depth
                        first: bool,                    // whether stack top is considered a token
                        trace: &mut Vec<usize>,         // non-terminal symbols 
                        stack: &mut Vec<#CRATE::Tag>,   // stack of suffix code
                    ) -> Result<usize, ()> {
                        // TODO: enforce a rule to be non-empty
                        #trace_start
                        let start = end;        // this is the starting point of this parsing pass
                        let mut head = true;    // tracks if it is the first sub-peg (might be modified by sub pegs)
                        let Ok(end) = (#body) else {
                            #trace_end_err
                            return Err(());
                        };
                        if start < end {
                            #trace_end_ok
                            #CRATE::stack_sanity_check(input, stack, start..end);
                            stack.push(#CRATE::Tag { rule: <Self as Num>::num(#num), span: start..end });
                            Ok(end)
                        } else {
                            while stack.last().map(|tag| tag.span.start > start).unwrap_or(false) {
                                stack.pop();
                            }
                            #trace_end_err
                            Err(())
                        }
                    }
                }
            }
        };
        for (num, rule) in self.rules.iter().enumerate() {
            let body = self.rules_vect_build(&rule.exprs)?;
            impls.extend(r#impl(num, &self.ident, &self.generics, &rule.variant, rule.trace, body));
        };
        Ok(impls)
    }
    // build a vector of rules
    fn rules_vect_build(&self, seq: &[Fmt]) -> Result<TokenStream> {    
        let seq = seq.iter().map(|fmt: &Fmt| {
            let result = self.rules_item_build(fmt)?;
            Ok(result)
        }).fold(Ok(vec![]), |v: Result<_>, x: Result<_> | {
            let mut v = v?;
            v.push(x?); Ok(v)
        })?;
        Ok(quote!{{(|| -> Result<usize, ()> {
            let size = stack.len();
            #(let Ok(end) = (#seq) else {
                stack.resize_with(size, || unreachable!());
                Err(())?
            };)*
            Ok::<_, ()>(end)
        })()}})
    }
    // fmt: the format string to translate
    // head: if the format string is the first symbol in this rule
    fn rules_item_build(&self, fmt: &Fmt) -> Result<TokenStream> {
        use Fmt::*;
        match fmt {
            Space => {Ok(quote! {
                if head && first { Err(()) }
                else if let Ok(end_) = Self::space(input, end) {
                    head &= end_ == end;
                    Ok(end_)
                }
                else { Err(()) }
            })}
            Token { token } => {Ok(quote! {{
                // if regex is the first token, and the stack top is considered as a token
                // then certainly this will not match
                if head && first { Err(()) }
                // if the prefix matches the token, 
                // remove token from input stream and proceed
                else if input[end..].starts_with(#token) {
                    head &= #token.len() == 0;
                    Ok::<_, ()>(end + #token.len())
                }
                else { Err(()) }
            }})}
            Symbol { typ, group, .. } => {Ok(quote! {{
                match <#typ as #CRATE::ParseImpl<#group, ERROR>>::parse_impl(
                    // pass 'input' and 'end' as-is
                    input, end,
                    // if it is the head, add 1 to left recursion depth
                    // otherwise, depth should be cleared (since it cannot recurse on the same pos)
                    if head { depth + 1 } else { 0 },
                    // the stack top can only used as the leftmost token
                    // so the 'first' proposition only holds when it is the head element
                    head && first,
                    // pass 'trace' and 'stack' as-is
                    trace, stack
                ) {
                    Ok(end_) if end_ > end => { head = false; Ok(end_) }
                    other => other
                }
            }})}
            RegExp { regex, refute, .. } => {Ok(quote! {{(|| -> Result<usize, ()> {
                // if regex is the first token, and the stack top is considered as a token
                // then certainly this will not match
                if head && first { Err(())? }
                // when the prefix matches the regex, 
                // remove the matched part from input stream and proceed
                static REGEX: #CRATE::LazyLock<#CRATE::Regex> = #CRATE::LazyLock::new(|| {
                    #CRATE::Regex::new(concat!("^(", #regex, ")")).unwrap()
                });
                let start = end;
                let end = start + REGEX.find(&input[end..]).map(|mat| mat.as_str().len()).ok_or(())?;
                // check if the matched string matches the refute pattern
                {
                    static REGEX: #CRATE::LazyLock<#CRATE::Regex> = #CRATE::LazyLock::new(|| 
                        #CRATE::Regex::new(concat!("^(", #refute, ")$")).unwrap()
                    );
                    if #refute.len() != 0 && REGEX.is_match(&input[start..end]) {
                        Err(())
                    }
                    else {
                        #CRATE::stack_sanity_check(input, stack, start..end);
                        stack.push(#CRATE::Tag { rule: 0, span: start..end });
                        head = false;
                        Ok::<_, ()>(end)
                    }
                }
            })()}})}
            SeqExp { children, .. } => {
                let children = children.iter().map(|(subfmt, flag)| {
                    let subfmt = self.rules_vect_build(subfmt)?;
                    match flag {
                        Flag::Repeat => {Ok(quote! {{
                            let start = end;
                            let mut end = end;
                            while let Ok(end_) = #subfmt {
                                end = end_;
                                cnt = cnt + 1;
                            }
                            Ok::<_, ()>(end)
                        }})}
                        Flag::OrNot => {Ok(quote! {{
                            match #subfmt {
                                Ok(end) => {cnt += 1; Ok::<_, ()>(end)}
                                Err(()) => {cnt += 0; Ok::<_, ()>(end)}
                            }
                        }})}
                        Flag::Just => {Ok(quote! {{
                            match #subfmt {
                                Ok(end) => {cnt += 1; Ok::<_, ()>(end)}
                                Err(()) => {Err(())}
                            }
                        }})}
                    }
                }).fold(Ok(vec![]), |v: Result<_>, x: Result<_> | {
                    let mut v = v?;
                    v.push(x?); Ok(v)
                })?;
                Ok(quote!{{(|| -> Result<usize, ()> {
                    let size = stack.len();
                    let mut cnt = 0;
                    let start = end;
                    #(let Ok(end) = (#children) else {
                        stack.resize_with(size, || unreachable!());
                        Err(())?
                    };)*
                    #CRATE::stack_sanity_check(input, stack, start..end);
                    stack.push(#CRATE::Tag { rule: cnt, span: start..end });
                    Ok::<_, ()>(end)
                })()}})
            }
        }
    }    
}