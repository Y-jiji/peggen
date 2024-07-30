use crate::*;

pub trait AstImplBuild {
    fn ast_impl_build(&self) -> Result<TokenStream>;
}

impl AstImplBuild for Builder {
    fn ast_impl_build(&self) -> Result<TokenStream> {
        let this = &self.ident;
        let generics = &self.generics;
        let count = &self.rules.len();
        let _crate = parse_str::<Ident>(CRATE).unwrap();
        let mut arms = TokenStream::new();
        let mut constraints = TokenStream::new();
        // For each rule, construct a branch
        for (num, rule) in self.rules.iter().enumerate() {
            let variant = &rule.ident;
            let mut argb = TokenStream::new();
            let mut argv = Vec::new();
            // Generate code for converting part of the tags into ast and remove them from stack. 
            // At the same time, collect typ constraints s.t. AstImpl<Extra> is implemented for each usage. 
            for expr in rule.exprs.iter().rev() {
                fn normalize(arg: &str) -> Result<Ident> {
                    let arg = if arg.chars().all(|arg| arg.is_digit(10)) {
                        format!("_{arg}")
                    } else {
                        arg.to_string()
                    };
                    parse_str::<Ident>(&arg)
                }
                match expr {
                    Fmt::Symbol { arg, typ, .. } => {
                        let arg = normalize(arg)?;
                        argb.extend(quote! {
                            let (stack, #arg) = <#typ as #_crate::AstImpl<Extra>>::ast(input, stack, extra);
                        });
                        argv.push(quote! { #arg, });
                        constraints.extend(quote! { #typ: AstImpl<Extra>, });
                    }
                    Fmt::Regex { arg, typ, .. } => {
                        let arg = normalize(arg)?;
                        argb.extend(quote! {
                            let (stack, #arg) = {
                                let tag = &stack[stack.len()-1];
                                (
                                    &stack[..stack.len()-1],
                                    <#typ as core::str::FromStr>::from_str(&input[tag.span.clone()])
                                        .unwrap()
                                )
                            };
                        });
                        argv.push(quote! { #arg, });
                        constraints.extend(quote! { #typ: AstImpl<Extra>, });
                    }
                    _ => {}
                }
            }
            argv.reverse();
            let argv = {
                let mut stream = TokenStream::new();
                for arg in argv { stream.extend(arg); }
                stream
            };
            // Construct the result ast
            let argv = 
                if rule.named { quote! { {#argv} } } 
                else          { quote! { (#argv) } };
            // Return ast and the rest part of the stack
            arms.extend(if self.is_enum {
                quote! {
                    #num => {#argb; (stack, Self::#variant #argv)}
                }   
            } else {
                quote! {
                    #num => {#argb; (stack, Self #argv)}
                }
            });
        }
        Ok(quote!{
            impl<#generics Extra> #_crate::AstImpl<Extra> for #this<#generics> {
                fn ast<'a>(input: &'a str, stack: &'a [#_crate::Tag], extra: &'a Extra) -> (&'a [#_crate::Tag], Self) {
                    if stack.len() == 0 { panic!("empty stack"); }
                    let tag   = &stack[stack.len()-1];
                    let stack = &stack[..stack.len()-1];
                    if tag.rule < <Self as #_crate::Num>::num(0)       { panic!("rule number not belong to this type"); }
                    if tag.rule >= <Self as #_crate::Num>::num(#count) { panic!("rule number not belong to this type"); }
                    match tag.rule - <Self as #_crate::Num>::num(0) {
                        #arms
                        _ => unreachable!()
                    }
                }
            }
        })
    }
}