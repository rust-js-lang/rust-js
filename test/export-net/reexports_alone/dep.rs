pub mod rng {
    pub fn next(n: u32) -> u32 {
        n * 3 + 1
    }
}

pub mod outer {
    pub mod inner {
        pub fn twice(n: u32) -> u32 {
            n * 2
        }
    }
}

pub const LIMIT: u8 = 9;

pub struct Point {
    pub x: u32,
}

pub fn seed() -> u32 {
    5
}

pub fn name_of(n: Option<&str>, xs: &[u64]) -> String {
    String::new()
}
