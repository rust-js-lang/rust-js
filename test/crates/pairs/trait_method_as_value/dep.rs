//! A trait and its impls, a macro's, as num-traits' `Zero` is.
pub trait Zero {
    fn is_zero(&self) -> bool;
}

macro_rules! zero_impl {
    ($t:ty) => {
        impl Zero for $t {
            fn is_zero(&self) -> bool {
                *self == 0
            }
        }
    };
}

zero_impl!(u8);
zero_impl!(u32);
