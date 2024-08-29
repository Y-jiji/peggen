use crate::*;

impl Builder {
    // build the rule trait(s) for the given type
    pub fn rules_impl_build(&self) -> Result<TokenStream> {
        let mut impls = TokenStream::new();
        let r#impl = |num, ident, generics, body| quote! {
            impl<const ERROR: bool, #generics> #CRATE::RuleImpl<#num, ERROR> for #ident<#generics> {
                fn rule_impl(
                    input: &str, end: usize,    // input[end..] represents the unparsed source
                    depth: usize,               // left recursion depth
                    first: bool,                // whether stack top is considered a token
                    trace: &mut Vec<usize>,     // non-terminal symbols 
                    stack: &mut Vec<Tag>,       // stack of suffix code
                ) -> Result<usize, ()> {
                    #body
                }
            }
        };
        for (num, rule) in self.rules.iter().enumerate() {
            let body = self.rules_vect_build(&rule.exprs, true)?;
            impls.extend(r#impl(num, &self.ident, &self.generics, body));
        };
        Ok(impls)
    }
    // build a vector of rules
    fn rules_vect_build(&self, seq: &[Fmt], mut head: bool) -> Result<TokenStream> {    
        let seq = seq.iter().map(|fmt| {
            let result = self.rules_item_build(fmt, head)?;
            head = false;
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
            Ok(end)
        })()}})
    }
    // fmt: the format string to translate
    // head: if the format string is the first symbol in this rule
    fn rules_item_build(&self, fmt: &Fmt, mut head: bool) -> Result<TokenStream> {
        use Fmt::*;
        match fmt {
            Space => {Ok(quote! {Self::space()})}
            Token { token } => {Ok(quote! {{
                // if regex is the first token, and the stack top is considered as a token
                // then certainly this will not match
                if #head && first { Err(()) }
                // if the prefix matches the token, 
                // remove token from input stream and proceed
                else if input[end..].starts_with(#token) {
                    Ok(end + token.len())
                } else { Err() }
            }})}
            Symbol { typ, group, .. } => {Ok(quote! {
                <#typ as #CRATE::ParseImpl<#group>>::parse_impl(
                    // pass 'input' and 'end' as-is
                    input, end,
                    // if it is the head, add 1 to left recursion depth
                    // otherwise, depth should be cleared (since it cannot recurse on the same pos)
                    if #head { depth + 1 } else { 0 },
                    // the stack top can only used as the leftmost token
                    // so the 'first' proposition only holds when it is the head element
                    first && #head,
                    // pass 'trace' and 'stack' as-is
                    trace, stack
                )
            })}
            RegExp { regex, refute, .. } => {Ok(quote! {{(|| -> Result<usize, ()> {
                // if regex is the first token, and the stack top is considered as a token
                // then certainly this will not match
                if #head && first { Err(())? }
                // when the prefix matches the regex, 
                // remove the matched part from input stream and proceed
                static REGEX: #CRATE::LazyLock<#CRATE::Regex> = #CRATE::LazyLock::new(|| 
                    #CRATE::Regex::new(concat!("^(", #regex, ")")).unwrap()
                );
                let begin = end;
                let end = REGEX.find(&input[end..]).map(|mat| mat.range().end).ok_or(())?;
                // if the matched string matches the refute pattern
                {
                    static REGEX: #CRATE::LazyLock<#CRATE::Regex> = #CRATE::LazyLock::new(|| 
                        #CRATE::Regex::new(concat!("^(", #refute, ")$")).unwrap()
                    );
                    if #refute.len() != 0 && REGEX.is_match(&input[begin..end]) {
                        Err(())
                    } else {
                        stack.push(Tag { rule: 0, span: begin..end });
                        Ok(end)
                    }
                }
            })()}})}
            SeqExp { children, .. } => {
                let children = children.iter().map(|(seq, flag)| {
                    let result = self.rules_vect_build(seq, head)?;
                    head = false;
                    match flag {
                        Flag::Repeat => {Ok(quote! {{
                            let mut end = end;
                            while let Ok(end_) = #result {
                                end = end_;
                            }
                            Ok(end)
                        }})}
                        Flag::OrNot => {Ok(quote! {{
                            match #result {
                                Ok(end) => Ok(end),
                                Err(()) => Ok(end),
                            }
                        }})}
                        Flag::Just => {Ok(result)}
                    }
                }).fold(Ok(vec![]), |v: Result<_>, x: Result<_> | {
                    let mut v = v?;
                    v.push(x?); Ok(v)
                })?;
                Ok(quote!{{(|| -> Result<usize, ()> {
                    let size = stack.len();
                    #(let Ok(end) = (#children) else {
                        stack.resize_with(size, || unreachable!());
                        Err(())?
                    };)*
                    Ok(end)
                })()}})
            }
        }
    }    
}