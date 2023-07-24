use std::alloc;
use std::{alloc::Layout, ptr::NonNull};

use crate::int_trait::Int;

// if this is used the size will go up to 24 unless the compiler gets smarter
pub(crate) struct RawVec<T, Limit: Int> {
    ptr: NonNull<T>,
    cap: Limit,
}

unsafe impl<T: Send, Limit: Int> Send for RawVec<T, Limit> {}
unsafe impl<T: Sync, Limit: Int> Sync for RawVec<T, Limit> {}

impl<T, Limit: Int> RawVec<T, Limit> {
    pub(crate) const fn new() -> Self {
        assert!(std::mem::size_of::<T>() != 0, "TODO: implement ZST support");
        RawVec {
            ptr: NonNull::dangling(),
            cap: Limit::ZERO,
        }
    }

    fn grow(&mut self) {
        if self.cap == Limit::MAX {
            crate::capacity_overflow()
        }

        // This can't overflow because we ensure self.cap <= isize::MAX.
        let new_cap = if self.cap == Limit::ZERO {
            Limit::ONE
        } else {
            self.cap.saturating_mul(Limit::ONE.add(Limit::ONE))
        };

        // Layout::array checks that the number of bytes is <= usize::MAX,
        // but this is redundant since old_layout.size() <= isize::MAX,
        // so the `unwrap` should never fail.
        let new_layout = Layout::array::<T>(new_cap.as_usize()).unwrap();

        // Ensure that the new allocation doesn't exceed `isize::MAX` bytes.
        assert!(
            new_layout.size() <= isize::MAX as usize,
            "Allocation too large"
        );

        let new_ptr = if self.cap == Limit::ZERO {
            unsafe { alloc::alloc(new_layout) }
        } else {
            let old_layout = Layout::array::<T>(self.cap.as_usize()).unwrap();
            let old_ptr = self.ptr.as_ptr() as *mut u8;
            unsafe { alloc::realloc(old_ptr, old_layout, new_layout.size()) }
        };

        // If allocation fails, `new_ptr` will be null, in which case we abort.
        self.ptr = match NonNull::new(new_ptr as *mut T) {
            Some(p) => p,
            None => alloc::handle_alloc_error(new_layout),
        };
        self.cap = new_cap;
    }
}

impl<T, Limit: Int> Drop for RawVec<T, Limit> {
    fn drop(&mut self) {
        if self.cap != Limit::ZERO {
            let layout = Layout::array::<T>(self.cap.as_usize()).unwrap();
            unsafe {
                alloc::dealloc(self.ptr.as_ptr() as *mut u8, layout);
            }
        }
    }
}
