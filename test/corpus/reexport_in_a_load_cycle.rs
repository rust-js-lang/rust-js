// A module's load reading another's `const`, where that module re-exports
// one of its functions, `pub use super::z::one`: the re-export is an
// import, so the two are a load cycle, and `z`'s `S` reads `K`'s value,
// not `K` by its name before `a` has made it (#295).
pub mod a {
    pub use super::z::one;

    pub const K: u32 = 7;

    pub fn k() -> u32 {
        K
    }
}

pub mod z {
    pub static S: [u32; 1] = [super::a::K];

    pub fn one() -> u32 {
        S[0]
    }
}

fn main() {
    println!("{}", a::k());
    println!("{}", a::one());
}
