//@ native: wasm32
// An unstable sort picks its small sort and its partition by its items'
// size (ADR 0342), which for a `&str`, a `usize`, a `String` and an array
// of `usize`s is a 32-bit target's: rust-js's (ADR 0090). Native Rust here
// is `wasm32-wasip1`'s, whose sizes are those.

fn main() {
    let text = "a bb cc d eee ff g hh ii j kkk ll m nn o pp qqq rr s tt uu v ww x yy zz aa bbb c dd ee f gg";
    let mut words: Vec<&str> = text.split(' ').collect();
    words.sort_unstable_by_key(|w| w.len());
    println!("{}", words.join(","));

    let mut pairs: Vec<(usize, u32)> = (0..40u32).map(|i| ((i as usize * 7) % 4, i)).collect();
    pairs.sort_unstable_by_key(|p| p.0);
    println!("{:?}", pairs.iter().map(|p| p.1).collect::<Vec<_>>());

    let mut owned: Vec<String> = text.split(' ').map(String::from).collect();
    owned.sort_unstable_by(|a, b| b.len().cmp(&a.len()));
    println!("{}", owned.join(","));

    let mut wide: Vec<[usize; 20]> = (0..50).map(|i| [i % 3, i, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]).collect();
    wide.sort_unstable_by_key(|w| w[0]);
    println!("{:?}", wide.iter().map(|w| w[1]).collect::<Vec<_>>());

    let mut picked: Vec<&str> = text.split(' ').collect();
    let middle = picked.len() / 2;
    picked.select_nth_unstable_by_key(middle, |w| w.len());
    println!("{}", picked.join(","));
}
