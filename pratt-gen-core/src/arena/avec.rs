use core::{ops::Index, mem::*};
use super::*;

/// ### Brief
/// A vector-like structure in arena. (somewhat too fat)
#[derive(Clone, Copy)]
pub struct AVec<'a, T> {
    /// The first block (blocks all starts with T, but not the first block)
    begin: &'a [T],
    /// The first block number
    block: usize,
    /// Count of total items
    count: usize,
    /// The arena that this vector grows in
    arena: &'a Arena,
}

impl<'a, T> AVec<'a, T> {
    /// ### Brief
    /// Create an arena vector builder. 
    pub fn new(arena: &'a Arena) -> Self {
        AVec {
            begin: &[],
            block: 0,
            count: 0, arena,
        }
    }
    /// ### Brief
    /// Push value into vector & arena
    /// 
    /// ### Parameters
    /// - `value`: the value to append
    /// 
    /// ### Safety
    /// The last element of this AVec must be the last element of self.arena. 
    pub unsafe fn push(&mut self, value: T) {
        // Allocate the value in arena
        let v_ref = self.arena.alloc_val(value) as *const T;
        // Count increments anyway
        self.count += 1;
        if self.count == 1 {
            // When self.begin is not allocated
            self.begin = from_raw_parts(v_ref, 1);
        } else if self.count == self.begin.len() + 1 && v_ref == self.begin.as_ptr_range().end {
            // When self.begin is in the same block as the newly allocated element, extend self.begin. 
            self.begin = from_raw_parts(self.begin.as_ptr() as *const _, self.count);
        } else if self.count == self.begin.len() + 1 {
            // When the first new block is allocated, store the block id. 
            self.block = self.arena.size() / N;
            #[cfg(feature="trace")]
            std::println!("another block {v_ref:?}");
        }
        // This will help us find error, but not exhaustively. 
        debug_assert_eq!(v_ref, &self[self.count-1] as *const _, "index {}", self.count-1);
    }
}

impl<'a, T: 'a> Index<usize> for AVec<'a, T> {
    type Output = T;
    fn index(&self, index: usize) -> &Self::Output {
        if index < self.begin.len() {
            &self.begin[index]
        }
        else if index < self.count { unsafe {
            let blk = (index - self.begin.len()) / (N / size_of::<T>()) + self.block;
            let off = (index - self.begin.len()) % (N / size_of::<T>()) * size_of::<T>();
            #[cfg(feature="trace")]
            std::println!("access block {blk} offset {off}");
            &*((*self.arena.buffer.get())[blk][off .. off+size_of::<T>()].as_ptr() as *const T)
        } }
        else {
            panic!("{index} >= {} index out of range. ", self.count)
        }
    }
}

impl<'a> AVec<'a, u8> {
    pub unsafe fn push_char(&mut self, char: char) {
        let len = char.len_utf8();
        let mut byt = [0u8; 2];
        char.encode_utf8(&mut byt);
        for i in 0..len { self.push(byt[i]); }
    }
    pub unsafe fn push_str(&mut self, str: &str) {
        // TODO: make this more efficient
        for &byte in str.as_bytes() { self.push(byte); }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use rand::*;
    use rand_xoshiro::*;

    macro_rules! test_for_type {($h: ty $(, $t: ty)*) => {{
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub struct A($h $(, $t)*);
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub struct B(u8);
        let mut rng = Xoroshiro64Star::from_seed(15u64.to_be_bytes());
        for i in 0..N {
            let arena = Arena::new();
            for _ in 0..=i {
                arena.alloc_val(B(rng.gen()));
            }
            let mut vec = Vec::new();
            let mut avec = AVec::new(&arena);
            for j in 0..2*N {
                let v = A(rng.gen::<$h>() $(, rng.gen::<$t>())*);
                vec.push(v);
                unsafe { avec.push(v) };
                assert!(vec[j] == avec[j], "{i} {j}: vec[{j}] = {:?}; avec[{j}] = {:?} ({:?})", vec[j], avec[j], &avec[j] as *const _);
            }
            for j in 0..2*N {
                assert!(vec[j] == avec[j], "{i} {j}: vec[{j}] = {:?}; avec[{j}] = {:?}", vec[j], avec[j]);
            }
        }
    }};}

    #[test]
    fn avec() {
        test_for_type!(u8, u8);
        test_for_type!(u8, u64);
        test_for_type!(u16, u8);
    }
}