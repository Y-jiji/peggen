// Using alloc for no_std allocation. 
extern crate alloc;
use core::mem::*;
use core::cell::*;
use core::alloc::*;
use core::slice::*;
use alloc::*;
use alloc::vec::*;
use alloc::alloc::*;
use alloc::boxed::*;

// Export vector and string in arena. 
mod avec;
pub use avec::*;

#[cfg(test)]
const N: usize = 512;   // Use smaller N, so tests can run faster. 

#[cfg(not(test))]
const N: usize = 4096;  // Bigger page size to mitigate fragmented allocation. 

/// A stack-like arena that allow shrink in unsafe mode
pub struct Arena {
    /// A pool for all chunks
    buffer: UnsafeCell<Vec<Box<[u8; N]>>>,
    /// The size of in-use space
    size: UnsafeCell<usize>,
}

impl Arena {
    /// Create a new arena
    pub fn new() -> Self {
        let block = unsafe {
            Box::from_raw(alloc_zeroed(Layout::from_size_align_unchecked(N, 8)) as *mut _)
        };
        Arena {
            buffer: UnsafeCell::new(vec![block]), 
            size: UnsafeCell::new(0),
        }
    }
    /// Make sure a slice with given size, at least from a
    unsafe fn ensure(&self, mut start: usize, size: usize) -> &mut [u8] {
        let mut end = start + size;
        // Value cannot be allocated across blocks, like: | ... start | end ... |
        // We must avoid that, so we move start to the next block, like: | ... | start ... end ... |
        // We presume that size_of::<V>() <= N
        if start / N < (end - 1) / N {
            end   += N - start % N;
            start += N - start % N;
        }
        // Current capacity
        let capacity = (*self.buffer.get()).len() * N;
        // If capacity is not enough, allocate a new block. 
        // This is different from because size can change without affecting capacity. 
        if end > capacity {
            let slice = alloc_zeroed(Layout::from_size_align_unchecked(N, 8)) as *mut _;
            (*self.buffer.get()).push(Box::from_raw(slice));
        }
        *self.size.get() = end;
        #[cfg(feature="trace")]
        std::println!("allocate block {} offset {}", start / N, start % N);
        // Allocate a new slice. 
        return &mut (*self.buffer.get())[start / N][start % N .. (end - 1) % N + 1];
    }
    /// Allocate a value
    pub fn alloc_val<V>(&self, value: V) -> &V { unsafe {
        // Aligned start
        let start = (self.size() + align_of::<V>() - 1) / align_of::<V>() * align_of::<V>();
        // Allocate a slice to copy data into. 
        let slice = self.ensure(start, size_of::<V>());
        // Copy data into allocate slice. 
        slice.copy_from_slice(from_raw_parts(&value as *const _ as *const u8, size_of::<V>()));
        // Return slice as &V
        &*(slice.as_ptr() as *const V)
    } }
    /// Allocate a string
    pub fn alloc_str(&self, value: &str) -> &str { unsafe {
        // If string is zero-length, just return a static str. 
        if value.len() == 0 { return "" }
        // Allocate a slice to copy string into. 
        let slice = self.ensure(self.size(), value.len());
        // Copy data into allocate slice. 
        slice.copy_from_slice(value.as_bytes());
        // Return slice as &str
        core::str::from_utf8(slice).unwrap()
    } }
    /// The size of arena
    pub fn size(&self) -> usize { unsafe {
        *self.size.get()
    } }
    /// Shrink arena to smaller size
    pub unsafe fn shrink_to(&self, len: usize) {
        *self.size.get() = len
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use rand::*;
    use rand_xoshiro::*;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct A(u8, u64, u8);

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct B(u8);

    #[test]
    fn arena_mix() {
        let mut rng = Xoroshiro64Star::from_seed(15u64.to_be_bytes());
        for _ in 0..100 {
            let arena = Arena::new();
            let mut a = (Vec::new(), Vec::new());
            let mut b = (Vec::new(), Vec::new());
            let mut choices = vec![];
            for _ in 0..N*4 {
                let choice = rng.gen_bool(0.5);
                choices.push(choice);
                if choice {
                    let v = A(rng.gen(), rng.gen(), rng.gen());
                    a.1.push(v);
                    let v_ref = arena.alloc_val(v);
                    a.0.push(v_ref);
                    assert!(v_ref == &v);
                } else {
                    let v = B(rng.gen());
                    b.1.push(v);
                    let v_ref = arena.alloc_val(v);
                    b.0.push(v_ref);
                    assert!(v_ref == &v);
                }
            }
            for i in 0..a.0.len() {
                assert!(a.0[i] == &a.1[i], "a[{i}] {:?} {:?} ({:?})", a.0[i], a.1[i], a.0[i] as *const _);
            }
            for i in 0..b.0.len() {
                assert!(b.0[i] == &b.1[i], "b[{i}] {:?} {:?} ({:?})", b.0[i], b.1[i], b.0[i] as *const _);
            }
        }
    }

    #[test]
    fn arena_a() {
        let mut rng = Xoroshiro64Star::from_seed(15u64.to_be_bytes());
        let arena = Arena::new();
        let mut a = (Vec::new(), Vec::new());
        for _ in 0..N*4 {
            let v = A(rng.gen(), rng.gen(), rng.gen());
            a.1.push(v);
            let v_ref = arena.alloc_val(v);
            a.0.push(v_ref);
            assert!(v_ref == &v);
        }
        for i in 0..N*4 {
            assert!(a.0[i] == &a.1[i], "a[{i}] {:?} {:?} ({:?})", a.0[i], a.1[i], a.0[i] as *const _);
        }
    }

    #[test]
    fn arena_off() {
        let mut rng = Xoroshiro64Star::from_seed(15u64.to_be_bytes());
        for i in 0..N {
            let arena = Arena::new();
            let mut a = (Vec::new(), Vec::new());
            for _ in 0..=i {
                arena.alloc_val(B(rng.gen()));
            }
            for _ in 0..N*4 {
                let v = A(rng.gen(), rng.gen(), rng.gen());
                a.1.push(v);
                let v_ref = arena.alloc_val(v);
                a.0.push(v_ref);
                assert!(v_ref == &v);
            }
            for i in 0..N*4 {
                assert!(a.0[i] == &a.1[i], "a[{i}] {:?} {:?} ({:?})", a.0[i], a.1[i], a.0[i] as *const _);
            }
        }
    }

    #[test]
    fn arena_u8() {
        let mut rng = Xoroshiro64Star::from_seed(15u64.to_be_bytes());
        let arena = Arena::new();
        let mut b = (Vec::new(), Vec::new());
        for _ in 0..N*4 {
            let v = B(rng.gen());
            b.1.push(v);
            let v_ref = arena.alloc_val(v);
            b.0.push(v_ref);
            assert!(v_ref == &v);
        }
        for i in 0..N*4 {
            assert!(b.0[i] == &b.1[i], "b[{i}] {:?} {:?} ({:?})", b.0[i], b.1[i], b.0[i] as *const _);
        }
    }
}