use crate::*;

pub trait AstImplBuild {
    fn ast_impl_build(&self) -> Result<TokenStream>;
}

impl AstImplBuild for Builder {
    fn ast_impl_build(&self) -> Result<TokenStream> {
        let _crate = parse_str::<Ident>(CRATE).unwrap();
        let mut arms = TokenStream::new();
        let mut constraints = TokenStream::new();
        // For each rule, construct a branch
        for (num, rule) in self.rules.iter().enumerate() {
            // Add trace if trace presents
            let this = &self.ident;
            let variant = &rule.ident;
            let trace_prolog = if rule.trace { quote! {
                println!("phase 2 tag {}::{}", stringify!(#this), stringify!(#variant));
            } } else { quote! {} };
            let mut argb = TokenStream::new();
            let mut argv = Vec::new();
            // Generate code for converting part of the tags into ast and remove them from stack. 
            // At the same time, collect typ constraints s.t. AstImpl<Extra> is implemented for each usage. 
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
                            #trace_prolog
                            let (stack, #arg) = <#typ as #_crate::AstImpl<Extra>>::ast(input, stack, extra);
                        });
                        argv.push(quote! { #arg, });
                        constraints.extend(quote! { #typ: AstImpl<Extra>, });
                    }
                    // For regex, just grab the argument
                    Fmt::RegExp { arg, typ, .. } => {
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
            // Merge arguments into an expression
            let argv = {
                argv.reverse();
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
                quote! { #num => {#argb; (stack, {Self::#variant #argv})} }
            } else {
                quote! { #num => {#argb; (stack, {Self #argv})} }
            });
        }
        // Prepare several tokens to be used later
        let this = &self.ident;
        let generics = &self.generics;
        Ok(quote!{impl<#generics Extra> #_crate::AstImpl<Extra> for #this<#generics> {
            fn ast<'a>(input: &'a str, stack: &'a [#_crate::Tag], extra: &'a Extra) -> (&'a [#_crate::Tag], Self) {
                // Get the tag number
                let tag = stack[stack.len()-1].rule - <Self as #_crate::Num>::num(0);
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
        }})
    }
}