//! libm's shape: `pub use self::math::*;`, whose `math` holds `pub mod
//! support`, which the glob re-exports as its namespace (#1102).
pub mod math {
    pub mod support {
        pub fn helper(n: u32) -> u32 {
            n * 10
        }
    }

    pub fn floor_of(n: u32) -> u32 {
        support::helper(n) + 1
    }
}

pub use self::math::*;
