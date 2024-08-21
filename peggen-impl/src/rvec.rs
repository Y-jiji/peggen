mod peggen {
    pub(crate) use peggen_core::*;
    pub(crate) use peggen_macs::*;    
}

use core::fmt::Debug;
use alloc::vec::Vec;
use peggen::*;

/// Reversed vector
#[derive(PrependAstImpl, Clone, PartialEq, Eq)]
pub struct RVec<T>(Vec<T>);

impl<T, Extra: Copy> Prepend<Extra> for RVec<T> {
    type Item = T;
    fn empty(_: Extra) -> Self {
        Self(Vec::new())
    }
    fn prepend(&mut self, value: Self::Item, _: Extra) {
        let Self(inner) = self;
        inner.push(value);
    }
}

impl<T: Debug> Debug for RVec<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_list().entries(self.0.iter().rev()).finish()
    }
}

impl<T> IntoIterator for RVec<T> {
    type Item = T;
    type IntoIter = core::iter::Rev<alloc::vec::IntoIter<T>>;
    fn into_iter(self) -> Self::IntoIter {
        let Self(inner) = self;
        inner.into_iter().rev()
    }
}

impl<T> core::ops::Index<usize> for RVec<T> {
    type Output = T;
    fn index(&self, index: usize) -> &Self::Output {
        &self.0[self.0.len()-1-index]
    }
}