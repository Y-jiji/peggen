mod pigeon {
    pub(crate) use pigeon_core::*;
    pub(crate) use pigeon_macs::*;    
}

use core::fmt::Debug;
use pigeon::*;
use bumpalo::Bump;

#[derive(PrependAstImpl)]
pub struct BRVec<'b, T>(bumpalo::collections::Vec<'b, T>);

impl<'b, T> Prepend<&'b Bump> for BRVec<'b, T> {
    type Item = T;
    fn empty(with: &'b Bump) -> Self {
        Self(bumpalo::collections::Vec::new_in(with))
    }
    fn prepend(&mut self, value: Self::Item, _: &'b Bump) {
        let Self(inner) = self;
        inner.push(value);
    }
}

impl<'b, T: Debug> Debug for BRVec<'b, T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_list().entries(self.0.iter().rev()).finish()
    }
}

impl<'b, T> IntoIterator for BRVec<'b, T> {
    type Item = T;
    type IntoIter = core::iter::Rev<bumpalo::collections::vec::IntoIter<'b, T>>;
    fn into_iter(self) -> Self::IntoIter {
        let Self(inner) = self;
        inner.into_iter().rev()
    }
}