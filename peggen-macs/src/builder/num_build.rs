use crate::*;

pub trait NumBuild {
    /// Build rule number trait for a type
    fn num_build(&self) -> Result<TokenStream>;
}

impl NumBuild for Builder {
    fn num_build(&self) -> Result<TokenStream> {
        // The structure name
        let this = &self.ident;
        // The total count or rules/groups (in case that groups are more than rules)
        let count = self.rules.len().max(self.group+1);
        // The generic parameters
        let generics = &self.generics;
        Ok(quote! {
            impl<#generics> #CRATE::Num for #this<#generics> {
                // Implement rule counting using static trick. 
                fn num(rule: usize) -> usize {
                    // Use a global counter
                    use core::sync::atomic::Ordering::SeqCst;
                    // Here we still use once_cell::sync::Lazy
                    use #CRATE::LazyLock;
                    // A global counter will step, so the rule numbers will be kept unique during a run
                    static DELTA: LazyLock<usize> = 
                        LazyLock::new(|| #CRATE::PIGEON_COUNT.fetch_add(#count, SeqCst));
                    *DELTA + rule
                }
            }
        })
    }
}