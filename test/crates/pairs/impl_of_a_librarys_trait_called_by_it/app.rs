//! The consumer's impl of its library's trait, which its own code calls
//! with a `u32` alone (#1233).
use dep::Visit;

struct Mine;

impl Visit for Mine {
    fn twin<T: Clone>(&self, t: &T) -> (T, T) {
        (t.clone(), t.clone())
    }
}

pub fn main() {
    println!("{:?}", Mine.twin(&5u32));
    println!("{}", dep::run(&Mine));
}
