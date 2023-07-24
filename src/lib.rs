mod int_trait;
mod raw_vec;

// provides a small size optimized vec

use int_trait::Int;
use std::alloc::{self, Layout};
use std::mem::ManuallyDrop;
use std::ptr;
use std::ptr::NonNull;

// could make for smaller ourter structs by allowing users to mesh data into a T
// stored in the vec

// pub struct Wrapper {
//     inner: SmallerVec<usize, u32>,
// }
// impl Wrapper {
//     pub fn push(&mut self, value: usize) {
//         self.inner.push(value);
//     }
//     pub fn pop(&mut self) -> Option<usize> {
//         self.inner.pop()
//     }
//     pub fn capacity(&self) -> usize {
//         self.inner.capacity()
//     }
//     pub fn len(&self) -> usize {
//         self.inner.len()
//     }
//     pub fn is_empty(&self) -> bool {
//         self.inner.is_empty()
//     }
//     pub fn insert(&mut self, index: usize, value: usize) {
//         self.inner.insert(index, value);
//     }
//     pub fn remove(&mut self, index: usize) -> usize {
//         self.inner.remove(index)
//     }
// }



#[derive(Debug, Clone)]
pub struct SmallerVec<T, Limit: Int> {
    ptr: NonNull<T>,
    len: Limit,
    cap: Limit,
}

impl<T, Limit: Int> SmallerVec<T, Limit> {

    const FIRST_ALLOC_SIZE: usize = match core::mem::size_of::<T>()  {
        i if i == 1 => 8,
        i if i <= 1024 => 4,
        _ => 1,
    };

    pub const fn new() -> Self {
        Self::new_unallocated()
    }
    #[cfg(not(no_global_oom_handling))]
    #[track_caller]
    pub fn push(&mut self, value: T) {
        if self.len == self.cap {
            self.grow();
        }

        unsafe {
            core::ptr::write(self.ptr.as_ptr().add(self.len.as_usize()), value);
        }

        self.len = self.len.add(Limit::ONE);
    }

    #[cfg(not(no_global_oom_handling))]
    pub fn pop(&mut self) -> Option<T> {
        if self.len == Limit::ZERO {
            None
        } else {
            self.len = self.len.sub(Limit::ONE);
            unsafe { Some(core::ptr::read(self.ptr.as_ptr().add(self.len.as_usize()))) }
        }
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.cap.as_usize()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.len.as_usize()
    }
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    const fn new_unallocated() -> Self {
        Self {
            ptr: NonNull::dangling(),
            len: Int::ZERO,
            cap: Int::ZERO,
        }
    }

    // This can't overflow an isize since self.cap.bits < isize::BITS.
    #[cfg(not(no_global_oom_handling))]
    #[inline(always)]
    fn growth_factor(&self) -> Limit {
        // self.cap.saturating_add(self.cap.sub(self.cap.half()))
        self.cap.saturating_add(self.cap)
    }
    #[cfg(not(no_global_oom_handling))]
    fn grow(&mut self) {
        // failout if allocation is already at the max
        if self.cap == Limit::MAX {
            capacity_overflow()
        }
        let (new_cap, new_layout) = if self.cap == Limit::ZERO {
            (
                Limit::from_usize(Self::FIRST_ALLOC_SIZE),
                Layout::array::<T>(Self::FIRST_ALLOC_SIZE).unwrap(),
            )
        } else {
            let new_cap = self.growth_factor();
            // `Layout::array` checks that the number of bytes is <= usize::MAX,
            // but this is redundant since old_layout.size() <= isize::MAX,
            // so the `unwrap` should never fail.
            let new_layout = Layout::array::<T>(new_cap.as_usize()).unwrap();
            (new_cap, new_layout)
        };

        // since the limit must be < usize it is also < isize

        let new_ptr = if self.cap == Limit::ZERO {
            unsafe { alloc::alloc(new_layout) }
        } else {
            let old_layout = Layout::array::<T>(self.cap.as_usize()).unwrap() ;
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

    #[cfg(not(no_global_oom_handling))]
    pub fn insert(&mut self, index: usize, element: T) {
        // space for the new element
        if self.cap == self.len {
            self.grow();
        }

        let len = self.len();
        if len < index {
            assert_failed(index, len);
        }
        unsafe {
            let insert_on = self.as_mut_ptr().add(index);

            ptr::copy(insert_on, insert_on.add(1), len - index);
            // Write it in, overwriting the first copy of the `index`th
            // element.
            ptr::write(insert_on, element);

            self.len = self.len.add(Limit::ONE);
        }
    }

    #[cfg(not(no_global_oom_handling))]
    pub fn remove(&mut self, index: usize) -> T {
        // Note: `<` because it's *not* valid to remove after everything
        let len = self.len();
        if len < index {
            assert_failed(index, len);
        }
        unsafe {
            self.len = self.len.sub(Limit::ONE);
            let p = self.as_mut_ptr().add(index);
            let result = ptr::read(p);
            ptr::copy(p.add(index + 1), p, len - index);
            result
        }
    }
}
#[cold]
#[inline(never)]
fn assert_failed(index: usize, len: usize) -> ! {
    panic!("insertion index: {index} should be <= len: {len}");
}

impl<T: Clone, Limit: Int> SmallerVec<T, Limit> {
    pub fn extend_from_slice(&mut self, other: &[T]) {
        for elem in other {
            self.push(elem.clone())
        }
    }
}

impl<T, Limit: Int> Default for SmallerVec<T, Limit> {
    fn default() -> Self {
        Self::new()
    }
}


#[cfg(not(no_global_oom_handling))]
#[cold]
fn capacity_overflow() -> ! {
    panic!("capacity overflow");
}

impl<T, Limit: Int> Drop for SmallerVec<T, Limit> {
    fn drop(&mut self) {
        if self.cap != Limit::ZERO {
            while self.pop().is_some() {}
            let layout = Layout::array::<T>(self.cap.as_usize()).unwrap();
            unsafe {
                alloc::dealloc(self.ptr.as_ptr() as *mut u8, layout);
            }
        }
    }
}

impl<T, Limit: Int> std::ops::Deref for SmallerVec<T, Limit> {
    type Target = [T];
    #[inline]
    fn deref(&self) -> &[T] {
        unsafe { std::slice::from_raw_parts(self.ptr.as_ptr(), self.len.as_usize()) }
    }
}

impl<T, Limit: Int> std::ops::DerefMut for SmallerVec<T, Limit> {
    #[inline]
    fn deref_mut(&mut self) -> &mut [T] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.len.as_usize()) }
    }
}

pub struct IntoIter<T> {
    buf: NonNull<T>,
    cap: usize,
    start: *const T,
    end: *const T,
}

impl<T, Limit: Int> IntoIterator for SmallerVec<T, Limit> {
    type Item = T;
    type IntoIter = IntoIter<T>;
    fn into_iter(self) -> IntoIter<T> {
        // Make sure not to drop Vec since that would free the buffer
        let vec = ManuallyDrop::new(self);

        // Can't destructure Vec since it's Drop
        let ptr = vec.ptr;
        let cap = vec.cap;
        let len = vec.len;

        unsafe {
            IntoIter {
                buf: ptr,
                cap: cap.as_usize(),
                start: ptr.as_ptr(),
                end: if cap == Limit::ZERO {
                    // can't offset off this pointer, it's not allocated!
                    ptr.as_ptr()
                } else {
                    ptr.as_ptr().add(len.as_usize())
                },
            }
        }
    }
}

impl<T> Iterator for IntoIter<T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        if self.start == self.end {
            None
        } else {
            unsafe {
                let result = ptr::read(self.start);
                self.start = self.start.offset(1);
                Some(result)
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = (self.end as usize - self.start as usize) / core::mem::size_of::<T>();
        (len, Some(len))
    }
}
impl<T> DoubleEndedIterator for IntoIter<T> {
    fn next_back(&mut self) -> Option<T> {
        if self.start == self.end {
            None
        } else {
            unsafe {
                self.end = self.end.offset(-1);
                Some(ptr::read(self.end))
            }
        }
    }
}
impl<T> Drop for IntoIter<T> {
    fn drop(&mut self) {
        if self.cap != 0 {
            // drop any remaining elements
            for _ in &mut *self {}
            let layout = Layout::array::<T>(self.cap).unwrap();
            unsafe {
                alloc::dealloc(self.buf.as_ptr() as *mut u8, layout);
            }
        }
    }
}
