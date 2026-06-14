use crate::*;

pub trait NumBuild {
    fn num_build(&self) -> Result<TokenStream>;
}

impl NumBuild for Builder {
    fn num_build(&self) -> Result<TokenStream> {
        let this = &self.ident;
        let count = self.rules.len().max(self.max_group() + 1);
        let generics = &self.generics;
        Ok(quote! {
            impl<#generics> #CRATE::Num for #this<#generics> {
                fn num(rule: usize) -> usize {
                    use core::sync::atomic::Ordering::Relaxed;
                    use #CRATE::LazyLock;
                    static DELTA: LazyLock<usize> =
                        LazyLock::new(|| #CRATE::PEGGEN_COUNT.fetch_add(#count, Relaxed));
                    *DELTA + rule
                }
            }
        })
    }
}
