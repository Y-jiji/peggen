use core::{marker::PhantomData, ops::Index, mem::*};
use super::*;

#[derive(Clone)]
pub struct AVec<'a, T> {
    // The head block
    begin: &'a [T],
    // The head block no
    block: usize,
    // Count of total item
    count: usize,
    // The arena that this vector grows in
    arena: &'a Arena,
}

impl<'a, T> AVec<'a, T> {
    /// Create an arena vector builder
    pub fn new(arena: &'a Arena) -> Self {
        AVec {
            begin: &[],
            block: unsafe { (*arena.buffer.get()).len() },
            count: 0, arena,
        }
    }
    /// Push value into vector & arena
    /// This is only allowed when AVec is new
    pub unsafe fn push(&mut self, value: T) {
        let v_ref = self.arena.alloc_val(value);
        if self.count == 0 {
            self.begin = from_raw_parts(v_ref as *const _, 1);
        }
        self.count += 1;
        if v_ref as *const _ == self.begin.as_ptr_range().end {
            self.begin = from_raw_parts(self.begin.as_ptr() as *const _, self.count);
        }
        debug_assert_eq!(v_ref as *const _, &self[self.count-1] as *const _, "index {}", self.count-1);
    }
}

impl<'a, T: 'a> Index<usize> for AVec<'a, T> {
    type Output = T;
    fn index(&self, index: usize) -> &Self::Output {
        if index < self.begin.len() {
            &self.begin[index]
        }
        else if index < self.count { unsafe {
            let blk = (index - self.begin.len()) / (N / size_of::<T>());
            let off = (index - self.begin.len()) % (N / size_of::<T>()) * size_of::<T>();
            #[cfg(test)]
            std::println!("block={} offset={}", self.block + blk, off);
            &*((*self.arena.buffer.get())[self.block+blk][off .. off+size_of::<T>()].as_ptr() as *const T)
        } }
        else {
            panic!("{index} >= {} index out of range. ", self.count)
        }
    }
}

#[cfg(test)]
mod test {
    use std::println;

    use super::*;
    use rand::*;
    use rand_xoshiro::*;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct A(u8, [u64; 2]);

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct B(u8);

    #[test]
    fn avec() {
        let mut rng = Xoroshiro64Star::from_seed(15u64.to_be_bytes());
        println!("{}", size_of::<A>());
        for i in 4064..=4064 {
            let arena = Arena::new();
            for _ in 0..=i {
                arena.alloc_val(B(rng.gen()));
            }
            let mut vec = Vec::new();
            let mut avec = AVec::new(&arena);
            for j in 0..N {
                let v = A(rng.gen(), [rng.gen(); 2]);
                vec.push(v);
                unsafe { avec.push(v) };
                assert!(vec[j] == avec[j], "{i} {j}: vec[{j}] = {:?}; avec[{j}] = {:?} ({:?})", vec[j], avec[j], &avec[j] as *const _);
            }
            for j in 0..N {
                assert!(vec[j] == avec[j], "{i} {j}: vec[{j}] = {:?}; avec[{j}] = {:?}", vec[j], avec[j]);
            }
        }
    }
}