// A function written in a trait's default method, as base64's `Engine`
// writes `encode`'s `inner`, of an impl in another module: the trait
// module's own, which the default each impl's dictionary copies imports,
// and which imports what it calls (ADRs 0185, 0308).
mod encode {
    pub const fn encoded_len(n: usize) -> usize {
        n.div_ceil(3) * 4
    }
}

mod engine {
    use super::encode::encoded_len;

    pub trait Engine {
        fn pad(&self) -> usize;

        fn size(&self, n: usize) -> usize {
            fn inner<E: Engine + ?Sized>(engine: &E, n: usize) -> usize {
                encoded_len(n) + engine.pad()
            }
            inner(self, n)
        }
    }

    pub mod general {
        pub struct Standard;

        impl super::Engine for Standard {
            fn pad(&self) -> usize {
                2
            }
        }
    }
}

fn main() {
    use engine::Engine;
    println!("{} {}", engine::general::Standard.size(10), engine::general::Standard.size(3));
}
