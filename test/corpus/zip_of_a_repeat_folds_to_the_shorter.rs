// A zip whose first side's step clones what may be the crate's own, a
// `repeat()`'s value, of two `TrustedLen` sides, std folds to the shorter
// side's count of pairs, `SpecFold`, where its `next()` steps the first side
// once more: a `collect()`, a `count()` and a `for_each()` clone the value
// as many times as there are pairs, `next()` once more at the end (#1162;
// ADR 0997).
#[derive(Debug)]
struct C(u32);

impl Clone for C {
    fn clone(&self) -> C {
        println!("clone {}", self.0);
        C(self.0 + 100)
    }
}

fn main() {
    let v: Vec<(C, C)> = std::iter::repeat(C(7)).zip(vec![C(1), C(2), C(3)]).collect();
    println!("{v:?}");
    let n = std::iter::repeat(C(8)).zip(vec![C(1), C(2)]).count();
    println!("{n}");
    let mut z = std::iter::repeat(C(9)).zip(vec![C(1)]);
    println!("{:?} {:?}", z.next(), z.next());
    std::iter::repeat(C(5)).zip(vec![C(1), C(2)]).for_each(|p| println!("{p:?}"));
    let w: Vec<(C, C)> = std::iter::repeat_n(C(6), 3).zip([C(1), C(2)]).collect();
    println!("{w:?}");
}
