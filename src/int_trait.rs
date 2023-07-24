pub trait Int: Sized + Copy + PartialEq {
    const ZERO: Self;
    const ONE: Self;
    const MAX: Self;
    /// implementer must always be smaller than a usize
    fn as_usize(self) -> usize;
    fn sub(self, rhs: Self) -> Self;
    fn add(self, rhs: Self) -> Self;
    fn half(self) -> Self;
    fn saturating_add(self, rhs: Self) -> Self;
    fn saturating_mul(self, rhs: Self) -> Self;
    fn from_usize(val: usize) -> Self;
}

macro_rules! impl_int {
    ($t:ty) => {
        impl crate::Int for $t {
            const ZERO: Self = 0;
            const ONE: Self = 1;
            const MAX: Self = Self::MAX;
            #[inline]
            fn as_usize(self) -> usize {
                self as usize
            }
            #[inline]
            fn sub(self, rhs: Self) -> Self {
                self - rhs
            }
            #[inline]
            fn add(self, rhs: Self) -> Self {
                self + rhs
            }
            #[inline]
            fn saturating_mul(self, rhs: Self) -> Self {
                self.saturating_mul(rhs)
            }
            #[inline]
            fn half(self) -> Self {
                self >> 1
            }
            #[inline]
            fn saturating_add(self, rhs: Self) -> Self {
                self.saturating_add(rhs)
            }
            #[inline]
            fn from_usize(val: usize) -> Self {
                val as Self
            }
        }
    };
}

#[cfg(target_pointer_width = "128")]
compile_error!("currently unsupported");

#[cfg(target_pointer_width = "64")]
mod b64impl {
    impl_int!(u32);
    impl_int!(u16);
    impl_int!(u8);
}

#[cfg(target_pointer_width = "32")]
mod b32impl {
    impl_int!(u16);
    impl_int!(u8);
}

#[cfg(target_pointer_width = "16")]
compile_error!("currently unsupported");
