use core::{marker::PhantomData, ops::Index, mem::*};
use super::*;

#[derive(Clone)]
pub struct AVec<'a, T> {
    start: usize,
    count: usize,
    arena: &'a Arena,
    phant: PhantomData<T>,
}

impl<'a, T> AVec<'a, T> {
    /// Create an arena vector builder
    pub fn new(arena: &'a Arena) -> Self {
        AVec {
            start: arena.size(),
            count: 0,
            arena: &arena, 
            phant: PhantomData
        }
    }
    /// Push value into vector & arena
    pub fn push(&mut self, value: T) {
        debug_assert!(self.start + self.count * core::mem::size_of::<T>() == self.arena.size());
        self.arena.alloc(value);
    }
}

impl<'a, T: 'a> Index<usize> for AVec<'a, T> {
    type Output = T;
    fn index(&self, index: usize) -> &'a Self::Output {
        if index >= self.count {
            panic!("{index} >= {} index out of range", self.count);
        }
        let mut a = (self.arena.size() + align_of::<T>() - 1) / align_of::<T>() * align_of::<T>();
        let mut b = a + size_of::<T>();
        if (a + 1) / N < b / N {
            b += N - a % N;
            a += N - a % N;
        }
        unsafe {
            let slice = &(*self.arena.buffer.get())[a / N][a % N .. b % N];
            &*(slice.as_ptr() as *const T)
        }
    }
}