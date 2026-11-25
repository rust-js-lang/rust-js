pub mod root_alone {
    pub use dep as d;
}

pub mod rng_alone {
    pub use dep::rng;
}

pub mod filed;

pub use filed as f;
pub use rng_alone as r;
pub use root_alone as i;
pub use dep::Point;

pub fn total() -> u32 {
    i::d::seed()
}
