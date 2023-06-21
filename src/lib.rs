#![feature(allocator_api)]
#![feature(alloc_layout_extra)]
use std::{
    alloc::{AllocError, Allocator, Layout},
    cell::Cell,
    ptr::NonNull,
};

#[derive(Debug)]
pub struct Arena {
    offset: Cell<usize>,
    allocation: Box<[u8]>,
}

impl Arena {
    pub fn with_capacity(capacity: usize) -> Result<Self, AllocError> {
        let layout = std::alloc::Layout::array::<u8>(capacity).map_err(|_| AllocError)?;
        let allocation: Box<[u8]> = unsafe {
            Box::from_raw(std::slice::from_raw_parts_mut(
                std::alloc::alloc(layout),
                capacity,
            ))
        };

        Ok(Self {
            offset: Cell::new(0),
            allocation,
        })
    }

    pub fn can_fit<T>(&self) -> bool {
        self.padding(Layout::new::<T>()).is_some()
    }
    pub fn can_fit_slice<T>(&self, n: usize) -> bool {
        Layout::new::<T>()
            .repeat(n)
            .ok()
            .and_then(|(l, _)| self.padding(l))
            .is_some()
    }

    fn padding(&self, layout: Layout) -> Option<usize> {
        let req_size = layout.size();
        let ptr = self.allocation.as_ptr() as usize + self.offset.get();
        let padding = (layout.align() - (ptr % layout.align())) % layout.align();
        let rem_size = (self.allocation.len() - self.offset.get()).checked_sub(padding)?;
        if rem_size < req_size {
            return None;
        }
        Some(padding)
    }
}

unsafe impl Allocator for &Arena {
    fn allocate(&self, layout: std::alloc::Layout) -> Result<NonNull<[u8]>, AllocError> {
        let padding = self.padding(layout).ok_or(AllocError)?;
        let padded_ptr = unsafe {
            (self.allocation.as_ptr())
                .add(self.offset.get())
                .add(padding)
        };

        let fat_ptr = unsafe {
            NonNull::new_unchecked(std::ptr::slice_from_raw_parts_mut(
                padded_ptr as *mut u8,
                layout.size(),
            ))
        };
        self.offset.set(self.offset.get() + padding + layout.size());
        Ok(fat_ptr)
    }

    unsafe fn deallocate(&self, _ptr: NonNull<u8>, _layout: std::alloc::Layout) {}
}

#[cfg(test)]
mod tests {
    use crate::Arena;

    #[test]
    fn it_works() {
        let arena = Arena::with_capacity(1024).unwrap();
        let a = Box::new_in(5.0, &arena);
        assert_eq!(a.as_ref(), &5.0);
        let b = Box::new_in(12, &arena);
        assert_eq!(b.as_ref(), &12)
    }

    #[test]
    fn it_works_vec() {
        let arena = Arena::with_capacity(24).unwrap();
        let mut a: Vec<u8, _> = Vec::new_in(&arena);
        a.extend(0..10);
        assert_eq!(&a, &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
    }

    #[test]
    fn test_allignement() {
        let arena = Arena::with_capacity(24).unwrap();
        let a = Box::new_in(5u8, &arena);
        assert_eq!(a.as_ref(), &5);
        let b = Box::new_in(u128::MAX, &arena);
        assert_eq!(b.as_ref(), &u128::MAX);
        assert_eq!(
            (b.as_ref() as *const u128) as usize % std::mem::align_of_val(b.as_ref()),
            0
        );
    }
    #[test]
    fn test_fit() {
        let arena = Arena::with_capacity(24).unwrap();
        assert!(arena.can_fit::<u8>());
        assert!(!arena.can_fit_slice::<u8>(200));
        assert!(arena.can_fit_slice::<u8>(24));
        let mut a: Vec<u8, _> = Vec::new_in(&arena);
        a.extend(0..24);
        assert!(!arena.can_fit::<u8>());
    }
}
