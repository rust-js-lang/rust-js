use dep::{Empty, Refused, collected, nothing};

pub fn main() {
    let Refused = collected(vec![4, 5].into_iter());
    println!("refused none");
    let refused = std::panic::catch_unwind(|| {
        let _: Refused = collected(vec![1, 2].into_iter());
    });
    println!("{}", refused.is_err());
    let empty: Empty = collected(0..3u32);
    println!("{empty:?}");
    let (bytes, unit, empty) = nothing(7u8);
    println!("{} {unit:?} {empty:?}", bytes.len());
}
