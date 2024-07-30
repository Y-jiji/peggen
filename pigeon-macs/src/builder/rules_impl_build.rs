use crate::*;

pub trait RulesImplBuild {
    fn rules_impl_build(&self) -> Result<TokenStream>;
}

impl RulesImplBuild for Builder {
    fn rules_impl_build(&self) -> Result<TokenStream> {
        let mut output = TokenStream::new();
        for rule in 0..self.rules.len() {
            let mut body = TokenStream::new();
            let _crate = parse_str::<Ident>(CRATE).unwrap();
            for expr in &self.rules[rule].exprs {
                use Fmt::*;
                body.extend(match expr {
                    Token { token } => quote! {
                        let end = if input[end..].starts_with(#token) {
                            end + #token.len()
                        } else { Err(())? };
                    },
                    Regex { regex, .. } => {
                        let regex = 
                            if regex.starts_with("^") { regex.clone() } 
                            else { format!("^{regex}") };
                        quote! {
                            let end = {
                                let begin = end;
                                static REGEX: #_crate::Lazy<#_crate::Regex> = #_crate::Lazy::new(|| #_crate::Regex::new(#regex).unwrap());
                                let Some(mat) = REGEX.find(&input[end..]) else {
                                    Err(())?
                                };
                                let mat = mat.as_str();
                                let end = end + mat.len();
                                stack.push(#_crate::Tag { rule: 0, span: begin..end });
                                end
                            };
                        }
                    },
                    Space => quote! {
                        let end = Self::space(input, end)?;
                    },
                    Symbol { typ, group, .. } => quote! {
                        let end = <#typ as ParseImpl<#group, ERROR>>::parse_impl(input, end, trace, stack)?;
                    },
                })
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
                        let mut inner = || -> Result<usize, ()> {
                            let begin = end;
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