extern crate alloc;
use core::mem::*;
use core::cell::*;
use core::alloc::*;
use core::slice::*;
use alloc::vec::*;
use alloc::alloc::*;
use alloc::boxed::*;
mod avec;
mod astr;

const N: usize = 4096;

/// A stack-like arena that allow shrink in unsafe mode
pub struct Arena {
    /// A pool for all chunks
    buffer: UnsafeCell<Vec<Box<[u8; N]>>>,
    /// The size of in-use space
    size: UnsafeCell<usize>,
}

impl Arena {
    pub fn new() -> Self {
        Arena {
            buffer: UnsafeCell::new(Vec::new()), 
            size: UnsafeCell::new(0),
        }
    }
    pub fn alloc<V>(&self, value: V) -> &V { unsafe {
        // aligned start
        let mut a = (self.size() + align_of::<V>() - 1) / align_of::<V>() * align_of::<V>();
        let mut b = a + size_of::<V>();
        if (a + 1) / N < b / N {
            b += N - a % N;
            a += N - a % N;
        }
        // current capacity
        let capacity = (*self.buffer.get()).len() * N;
        if b > capacity {
            let slice = alloc_zeroed(Layout::from_size_align_unchecked(N, 8)) as *mut _;
            (*self.buffer.get()).push(Box::from_raw(slice));
        }
        *self.size.get() = b;
        // allocate new slice
        let slice = &mut (*self.buffer.get())[a / N][a % N .. b % N];
        let v_ref = from_raw_parts(&value as *const _ as *const u8, size_of::<V>());
        slice.copy_from_slice(v_ref);
        &*(slice.as_ptr() as *const V)
    } }
    pub fn alloc_str(&self, value: &str) -> &str { unsafe {
        if value.len() == 0 { return "" }
        // aligned start
        let mut a = self.size();
        let mut b = a + value.len();
        if (a + 1) / N < b / N {
            b += N - a % N;
            a += N - a % N;
        }
        // current capacity
        let capacity = (*self.buffer.get()).len() * N;
        if b > capacity {
            let slice = alloc_zeroed(Layout::from_size_align_unchecked(N, 8)) as *mut _;
            (*self.buffer.get()).push(Box::from_raw(slice));
        }
        *self.size.get() = b;
        // allocate new slice
        let slice = &mut (*self.buffer.get())[a / N][a % N .. b % N];
        slice.copy_from_slice(value.as_bytes());
        core::str::from_utf8(slice).unwrap()
    } }
    pub fn size(&self) -> usize { unsafe {
        *self.size.get()
    } }
    pub unsafe fn shrink_to(&self, len: usize) {
        *self.size.get() = len
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use rand::Rng;
    use rand_xoshiro::*;
    use rand_core::*;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct A(u8, u64);

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct B(u8);

    #[test]
    fn alloc_test() {
        let mut rng = Xoroshiro64Star::from_seed(15u64.to_be_bytes());
        let arena = Arena::new();
        let mut a = (Vec::new(), Vec::new());
        let mut b = (Vec::new(), Vec::new());
        for _ in 0..4096 {
            let v = A(rng.gen(), rng.gen());
            a.1.push(v);
            let v_ref = arena.alloc(v);
            a.0.push(v_ref);
            assert!(v_ref == &v);
            let v = B(rng.gen());
            b.1.push(v);
            let v_ref = arena.alloc(v);
            b.0.push(v_ref);
            assert!(v_ref == &v);
        }
        for i in 0..4096 {
            assert!(a.0[i] == &a.1[i], "a[{i}] {:?} {:?} ({:?})", a.0[i], a.1[i], a.0[i] as *const _);
            assert!(b.0[i] == &b.1[i], "b[{i}] {:?} {:?} ({:?})", b.0[i], b.1[i], b.0[i] as *const _);
        }
    }
}