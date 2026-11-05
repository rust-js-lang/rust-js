// An iterator's `is_sorted_by(f)`, `f` of each two in a row up to the
// first out of order, and `is_sorted_by_key(f)`, std's
// `self.map(f).is_sorted()`, each item's key made once; a function that
// holds what has a destructor dropped where std drops it, `is_sorted_by`'s
// before the iterator, as its `all` drops it, `is_sorted_by_key`'s after,
// as the `Map` it folds drops its fields (#763).
struct D(&'static str);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct It {
    left: u32,
    _held: D,
}

impl Iterator for It {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        (self.left > 0).then(|| {
            self.left -= 1;
            self.left
        })
    }
}

fn it(left: u32) -> It {
    It { left, _held: D("it") }
}

fn zero(_g: &D) -> u32 {
    0
}

fn main() {
    println!("{}", [1, 2, 2, 5].iter().is_sorted_by(|a, b| a <= b));
    println!("{}", vec![3, 1].into_iter().is_sorted_by(|a, b| a <= b));
    println!("{}", (0..5).map(|x| x % 3).is_sorted_by_key(|x| x));
    println!("{}", ["a", "bb", "ccc"].iter().is_sorted_by_key(|s| s.len()));
    println!("{}", [3u32, 1, 2].into_iter().is_sorted_by_key(|x| x % 3));
    println!("{}", [1.0, f64::NAN].iter().is_sorted_by(|a, b| a <= b));
    let g = D("f");
    println!("{}", it(3).is_sorted_by(move |a, b| a + zero(&g) >= *b));
    let g = D("f");
    println!("{}", it(3).is_sorted_by_key(move |a| a + zero(&g)));
    let g = D("f");
    println!("{}", (0..3u32).is_sorted_by(move |a, b| a + zero(&g) <= *b));
    let g = D("f");
    println!("{}", (0..3u32).is_sorted_by_key(move |a| a + zero(&g)));
    let mut calls = 0;
    println!(
        "{}",
        (0..6u32).map(|x| if x == 2 { 9 } else { x }).is_sorted_by(|a, b| {
            calls += 1;
            a <= b
        })
    );
    println!("calls {calls}");
}
