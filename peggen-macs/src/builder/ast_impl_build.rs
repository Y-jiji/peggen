use crate::*;

pub trait AstImplBuild {
    fn ast_impl_build(&self) -> Result<TokenStream>;
}

impl AstImplBuild for Builder {
    fn ast_impl_build(&self) -> Result<TokenStream> {
        // Prepare several tokens to be used later
        let this = &self.ident;
        // Build generics and where condition
        let generics = &self.generics;
        let comma = generics.to_token_stream().into_iter().last().map(|x: TokenTree| x.to_string() == ",").unwrap_or(false);
        let generics = 
            if !comma && !generics.is_empty() { quote! { #generics, } }
            else                              { quote! { #generics  } };
        let (front, with) = if let Some(with) = self.with.clone() {
            (generics.clone(), with)
        } else {
            (quote! { #generics Extra: Copy }, quote! { Extra })
        };
        let mut arms = TokenStream::new();
        // For each rule, construct a branch
        for (num, rule) in self.rules.iter().enumerate() {
            let variant = &rule.variant;
            let mut argb = TokenStream::new();
            let mut argv = Vec::new();
            // Generate code for converting part of the tags into ast and remove them from stack. 
            // At the same time, collect typ constraints s.t. AstImpl<#with> is implemented for each usage. 
            // Ast is suffix encoded, so the 2nd-parser have to parse from tail to head. 
            for expr in rule.exprs.iter().rev() {
                // The argument can be a number, so normalize the number to rust ident with _
                fn normalize(arg: &str) -> Result<Ident> {
                    let dig = arg.chars().all(|arg| arg.is_digit(10));
                    let arg = if dig { format!("_{arg}") } else { arg.to_string() };
                    parse_str::<Ident>(&arg)
                }
                // Just an expression. 
                match expr {
                    // For symbols and lists, call the related 2nd-parser
                    Fmt::Symbol { arg, typ, .. } | Fmt::SeqExp { arg, typ, .. } => {
                        let arg = normalize(arg)?;
                        argb.extend(quote! {
                            let (stack, #arg) = <#typ as #CRATE::AstImpl<#with>>::ast(input, stack, with);
                        });
                        argv.push(quote! { #arg, });
                    }
                    // For regex, just grab the argument
                    Fmt::RegExp { arg, typ, .. } => {
                        let arg = normalize(arg)?;
                        argb.extend(quote! {
                            let (stack, #arg) = {
                                let tag = &stack[stack.len()-1];
                                (
                                    &stack[..stack.len()-1],
                                    <#typ as #CRATE::FromStr<#with>>::from_str_with(&input[tag.span.clone()], with)
                                )
                            };
                        });
                        argv.push(quote! { #arg, });
                    }
                    _ => {}
                }
            }
            // Construct the result ast args
            let argv = {
                argv.reverse();
                if rule.named { quote! { {#(#argv)*} } }
                else          { quote! { (#(#argv)*) } }
            };
            let trace = rule.trace;
            // Return ast and the rest part of the stack
            arms.extend(if self.is_enum {
                quote! { #num => {
                    if #trace { println!("AST\t{}::{}\t{stack:?}", stringify!(#this), stringify!(#variant)); }
                    #argb;
                    (stack, {Self::#variant #argv})
                } }
            } else {
                quote! { #num => {
                    if #trace { println!("AST\t{}\t{stack:?}", stringify!(#this)); }
                    #argb;
                    (stack, {Self #argv})
                } }
            });
        }
        Ok(quote!{
            impl<#front> #CRATE::AstImpl<#with> for #this<#generics> {
                fn ast<'lifetime>(
                    input: &'lifetime str, 
                    stack: &'lifetime [#CRATE::Tag], 
                    with: #with
                ) -> (&'lifetime [#CRATE::Tag], Self) {
                    // Get the tag number
                    let tag = stack[stack.len()-1].rule - <Self as #CRATE::Num>::num(0);
                    // Remove the last element from stack, this will be processed by arms
                    let stack = &stack[..stack.len()-1];
                    // Get the tag, it should range from 0 to <RULES>
                    match tag {
                        // All the match arms
                        #arms
                        // The rest of the arms is unreachable
                        _ => unreachable!()
                    }
                }
            }
        })
    }
}