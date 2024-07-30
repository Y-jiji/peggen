use crate::*;

pub trait NumBuild {
    fn num_build(&self) -> Result<TokenStream>;
}

impl NumBuild for Builder {
    /// Build rule number trait for a type
    fn num_build(&self) -> Result<TokenStream> {
        let _crate = parse_str::<Ident>(CRATE).unwrap();
        let this = &self.ident;
        let generics = &self.generics;
        let count = self.rules.len().max(self.group+1);
        Ok(quote! {
            impl<#generics> #_crate::Num for #this<#generics> {
                fn num(rule: usize) -> usize {
                    static DELTA: #_crate::Lazy<usize> = #_crate::Lazy::new(|| #_crate::COUNT.fetch_add(#count, core::sync::atomic::Ordering::SeqCst));
                    *DELTA + rule
                }
            }
        })
    }
}