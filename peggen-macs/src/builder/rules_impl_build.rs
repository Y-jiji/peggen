use quote::ToTokens;

use crate::*;

pub trait RulesImplBuild {
    fn rules_impl_build(&self) -> Result<TokenStream>;
    fn rules_item_build(&self, fmt: &Fmt) -> Result<TokenStream>;
}

impl RulesImplBuild for Builder {
    fn rules_item_build(&self, fmt: &Fmt) -> Result<TokenStream> {
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
            Fmt::RegExp { regex, refute, .. } => {
                let regex = format!("^({regex})");
                let refex = refute.as_ref().map(|r| format!("^({r})")).unwrap_or(String::new());
                let refute = refute.is_some();
                quote! {
                    let end = {
                        let begin = end;
                        static REGEX: #_crate::LazyLock<#_crate::Regex> = #_crate::LazyLock::new(|| #_crate::Regex::new(#regex).unwrap());
                        let Some(mat) = REGEX.find(&input[begin..]) else {
                            Err(())?
                        };
                        let mat = mat.as_str();
                        let end = end + mat.len();
                        if #refute {
                            static REGEX: #_crate::LazyLock<#_crate::Regex> = #_crate::LazyLock::new(|| #_crate::Regex::new(#refex).unwrap());
                            if REGEX.is_match(&input[begin..end]) { Err(())? }
                        }
                        stack.push(#_crate::Tag { rule: 0, span: begin..end });
                        end
                    };
                }
            },
            Fmt::SeqExp { children, .. } => {
                let mut body = TokenStream::new();
                for (child, flag) in children {
                    let child = child.iter()
                        .map(|fmt| self.rules_item_build(fmt))
                        .try_fold(TokenStream::new(), |mut a, b| { a.extend(b?); Result::Ok(a) })?;
                    body.extend(match flag {
                        Flag::Just => quote! {
                            let end = {
                                let size = stack.len();
                                if let Ok(end_) = (|| -> Result<usize, ()> { #child; Ok(end) })() {
                                    count += 1; end_
                                } else {
                                    stack.resize_with(size, || unreachable!());
                                    return Err(());
                                }
                            };
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
        // The merged output stream
        let mut output = TokenStream::new();
        // For each rule, build a Rule<N> trait
        for rule in 0..self.rules.len() {
            // Add every hole in every rule
            let mut body = TokenStream::new();
            for fmt in &self.rules[rule].exprs {
                body.extend(self.rules_item_build(fmt)?);
            }
            // Prepare identities
            let _crate = parse_str::<Ident>(CRATE).unwrap();
            let this = &self.ident;
            let _var = &self.rules[rule].ident;
            let generics = &self.generics.params;
            let comma = generics.to_token_stream().into_iter().last().map(|x| x.to_string() == ",").unwrap_or(false);
            let generics = 
                if !comma && !generics.is_empty() { quote! { #generics, } }
                else                              { quote! { #generics  } };
            // Build Rule<N, ERROR>
            output.extend(quote! {
                impl<#generics const ERROR: bool> #_crate::RuleImpl<#rule, ERROR> for #this<#generics> 
                    where Self: #_crate::Space,
                {
                    #[inline]
                    fn rule_impl(
                        input: &str, end: usize, 
                        trace: &mut Vec<(usize, usize)>,
                        stack: &mut Vec<#_crate::Tag>,
                    ) -> Result<usize, ()> {
                        // println!("REST\t{}", &input[end..]);
                        // println!("RULE\t{}", stringify!(#_var));
                        let size = stack.len();
                        let rule = <Self as #_crate::Num>::num(#rule);
                        let begin = end;
                        if stack.last().map(|top| 
                            top.span.start == end && 
                            top.rule == rule
                        ).unwrap_or(false) {
                            Ok(stack.last().unwrap().span.end)
                        } else {
                            #body
                            stack.push(#_crate::Tag { rule, span: begin..end });
                            // println!("TAKE\t{}", &input[begin..end]);
                            Ok(end)
                        }
                    }
                }
            })
        }
        // The final output
        Ok(output)
    }
}