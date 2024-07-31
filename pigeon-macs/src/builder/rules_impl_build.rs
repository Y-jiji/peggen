use crate::*;

pub trait RulesImplBuild {
    fn rules_impl_build(&self) -> Result<TokenStream>;
    fn rules_body_build(&self, fmt: &Fmt) -> Result<TokenStream>;
}

impl RulesImplBuild for Builder {
    fn rules_body_build(&self, fmt: &Fmt) -> Result<TokenStream> {
        let _crate = parse_str::<Ident>(CRATE).unwrap();
        Ok(match fmt {
            Fmt::Space => quote! {
                let end = Self::space(input, end)?;
            },
            Fmt::Token { token } => quote! {
                let end = if input[end..].starts_with(#token) {
                    end + #token.len()
                } else { Err(())? };
            },
            Fmt::Symbol { typ, group, .. } => quote! {
                let end = <#typ as ParseImpl<#group, ERROR>>::parse_impl(input, end, trace, stack)?;
            },
            Fmt::RegExp { regex, .. } => {
                let regex = format!("^({regex})");
                quote! {
                    let end = {
                        let begin = end;
                        static REGEX: #_crate::Lazy<#_crate::Regex> = #_crate::Lazy::new(|| #_crate::Regex::new(#regex).unwrap());
                        let Some(mat) = REGEX.find(&input[begin..]) else {
                            Err(())?
                        };
                        let mat = mat.as_str();
                        let end = end + mat.len();
                        stack.push(#_crate::Tag { rule: 0, span: begin..end });
                        end
                    };
                }
            },
            Fmt::PushGroup { children, .. } => {
                let mut body = TokenStream::new();
                for (child, flag) in children {
                    let child = child.iter()
                        .map(|fmt| self.rules_body_build(fmt))
                        .try_fold(TokenStream::new(), |mut a, b| { a.extend(b?); Result::Ok(a) })?;
                    body.extend(match flag {
                        Flag::Null => quote! {
                            let end = { #child; end }; 
                        },
                        Flag::Repeat => quote! {
                            let end = {    
                                let mut end = end;
                                loop {
                                    // Use a closure to wrap `Err(...)?` to prevent exiting outer function. 
                                    let size = stack.len();
                                    if let Ok(end_) = (|| -> Result<usize, ()> { #child; Ok(end) })() {
                                        end = end_;
                                        count += 1;
                                    } else {
                                        stack.resize_with(size, || unreachable!());
                                        break end 
                                    }
                                }
                            };
                        },
                        Flag::OrNot => quote! {
                            let end = {
                                let size = stack.len();
                                // Use a closure to wrap `Err(...)?` to prevent exiting outer function. 
                                if let Ok(end_) = (|| -> Result<usize, ()> { #child; Ok(end) })() {
                                    count += 1;
                                    end_
                                } else {
                                    stack.resize_with(size, || unreachable!());
                                    end
                                }
                            };
                        }
                    });
                }
                quote! {
                    let end = {
                        let begin = end;
                        let mut count = 0usize;
                        #body;
                        stack.push(#_crate::Tag { rule: count, span: begin..end });
                        end
                    };
                }
            }
        })
    }
    fn rules_impl_build(&self) -> Result<TokenStream> {
        let mut output = TokenStream::new();
        for rule in 0..self.rules.len() {
            let mut body = TokenStream::new();
            let _crate = parse_str::<Ident>(CRATE).unwrap();
            for fmt in &self.rules[rule].exprs {
                body.extend(self.rules_body_build(fmt)?);
            }
            let this = &self.ident;
            let generics = &self.generics.params;
            output.extend(quote! {
                impl<#generics const ERROR: bool> #_crate::RuleImpl<#rule, ERROR> for #this<#generics> 
                    where Self: #_crate::Space,
                {
                    #[inline]
                    fn rule_impl(
                        input: &str, end: usize, last: usize,
                        trace: &mut Vec<(usize, usize)>,
                        stack: &mut Vec<#_crate::Tag>,
                    ) -> Result<usize, ()> {
                        let size = stack.len();
                        let rule = <Self as #_crate::Num>::num(#rule);
                        let begin = end;
                        let mut inner = || -> Result<usize, ()> {
                            #body
                            stack.push(#_crate::Tag { rule, span: begin..end });
                            return Ok(end);
                        };
                        match inner() {
                            Ok(end) if end > last => {
                                Ok(end)
                            },
                            Err(()) | Ok(..) => {
                                stack.resize_with(size, || unreachable!()); 
                                Err(())
                            }
                        }
                    }
                }
            })
        }
        Ok(output)
    }
}