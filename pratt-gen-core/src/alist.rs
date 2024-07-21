use core::fmt::Debug;

use crate::*;

#[derive(Clone, Copy)]
pub enum AList<'a, T> {
    None,
    Some(T, &'a AList<'a, T>)
}

impl<'a, T> AList<'a, T> {
    /// An arena list from iterator.
    /// This function have to be implemented using the stack trick.  
    pub fn try_from<E>(arena: &'a Arena, mut iter: impl Iterator<Item = Result<T, E>>) -> Result<Self, E> {
        stacker::maybe_grow(32*1024, 1024*104, || {
            let Some(val) = iter.next() else {return Ok(Self::None)};
            let val = val?;
            let suf = AList::try_from(arena, iter)?;
            Ok(AList::Some(val, arena.alloc_val(suf)))
        })
    }
    pub fn from(arena: &'a Arena, mut iter: impl Iterator<Item = T>) -> Self {
        stacker::maybe_grow(32*1024, 1024*104, || {
            let Some(val) = iter.next() else {return Self::None};
            let suf = AList::from(arena, iter);
            AList::Some(val, arena.alloc_val(suf))
        })
    }
}

impl<'a, T> Iterator for AList<'a, T> where
    T: Clone
{
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::None => None,
            Self::Some(val, next) => {
                let val = val.clone();
                *self = next.clone();
                Some(val)
            }
        }
    }
}

impl<'a, T> Debug for AList<'a, T> where 
    T: Clone + Debug
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}

#[cfg(test)]
mod test {
    use alloc::format;
    use alloc::vec::Vec;
    use rand::Rng;
    use rand_xoshiro::*;
    use rand_core::*;
    use crate::{AList, Arena};

    #[test]
    fn alist() {
        let mut rng = Xoroshiro128Plus::from_seed(15u128.to_ne_bytes());
        let arena = Arena::new();
        for _ in 0..20 {
            let seed = rng.r#gen();
            let mut rng = Xoroshiro128Plus::from_seed(seed);
            let alist = AList::from(&arena, (0..100).map(|_| arena.alloc_val(rng.next_u64())));
            let mut rng = Xoroshiro128Plus::from_seed(seed);
            let mock = (0..100).map(|_| arena.alloc_val(rng.next_u64())).collect::<Vec<_>>();
            assert!(format!("{alist:?}") == format!("{mock:?}"))
        }
    }
}