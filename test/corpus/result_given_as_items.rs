// A `Result` given where an `IntoIterator` is taken, to `extend`, `chain`,
// `zip` or a generic function, is its `Ok` value or nothing, as its
// `into_iter()` is (ADR 0923); a `&mut` to one is a handle on its value.

struct Loud(i32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn total<I: IntoIterator<Item = i32>>(items: I) -> i32 {
    items.into_iter().sum()
}

fn main() {
    let ok: Result<i32, String> = Ok(6);
    let err: Result<i32, String> = Err("no".to_string());

    let mut extended = vec![1];
    extended.extend(ok.clone());
    extended.extend(err.clone());
    let chained: Vec<i32> = [1].into_iter().chain(ok.clone()).chain(err.clone()).collect();
    let zipped: Vec<(i32, i32)> = [1, 2].into_iter().zip(ok.clone()).collect();
    println!("{extended:?} {chained:?} {zipped:?}");
    println!("{} {}", total(ok.clone()), total(err.clone()));

    let mut held: Result<i32, String> = Ok(1);
    if let Ok(x) = &mut held {
        *x += 10;
    }
    let more: Vec<i32> = [&0].into_iter().chain(&held).copied().collect();
    let mut zero = 0;
    for x in std::iter::once(&mut zero).chain(&mut held) {
        *x += 1;
    }
    println!("{held:?} {more:?} {zero}");

    let louds: Vec<Loud> = [Loud(1)].into_iter().chain(Ok::<Loud, ()>(Loud(2))).collect();
    println!("{}", louds.len());
}
