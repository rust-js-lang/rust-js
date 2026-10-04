// A std function taken as a value is the arrow its call would be: `.map(str::len)`
// is `.map((s) => $byteLen(s))`, as `.map(|s| s.len())` is, of any number of
// arguments, zero for `unwrap_or_else(Vec::new)` and two for `fold(0, i32::max)`.

use std::collections::HashMap;

fn main() {
    // Of one argument: lengths in bytes, as `len()` is, and emptiness.
    let words = vec!["héllo", "ab", ""];
    let bytes: Vec<usize> = words.iter().copied().map(str::len).collect();
    let owned: Vec<String> = words.iter().map(ToString::to_string).collect();
    let sizes: Vec<usize> = owned.iter().map(String::len).collect();
    let rows = vec![vec![1, 2], vec![]];
    let counts: Vec<usize> = rows.iter().map(Vec::len).collect();
    let empty: Vec<bool> = rows.iter().map(Vec::is_empty).collect();
    println!("{:?} {:?} {:?} {:?} {:?}", bytes, owned, sizes, counts, empty);

    // Conversions, and what an `Option` or a `Result` gives.
    let names: Vec<String> = vec!["a", "b"].into_iter().map(str::to_string).collect();
    let into: Vec<String> = vec!["c"].into_iter().map(Into::into).collect();
    let present: Vec<i32> = vec![Some(1), Some(2)].into_iter().map(Option::unwrap).collect();
    let parsed: Vec<Result<i32, String>> = vec![Ok(3), Err("no".to_string())];
    let good: Vec<i32> = parsed.into_iter().filter_map(Result::ok).collect();
    println!("{:?} {:?} {:?} {:?}", names, into, present, good);
    let numbers: Result<Vec<i32>, _> = "4 5 6".split(' ').map(str::parse::<i32>).collect();
    let bad: Result<Vec<u8>, _> = "7 300".split(' ').map(str::parse::<u8>).collect();
    println!("{:?} {:?}", numbers, bad);

    // Of none: what makes an empty value.
    let missing: Option<Vec<i32>> = None;
    let text: Option<String> = None;
    let table: Option<HashMap<String, i32>> = None;
    println!(
        "{:?} {:?} {:?}",
        missing.unwrap_or_else(Vec::new),
        text.unwrap_or_else(String::new),
        table.unwrap_or_else(HashMap::new).len()
    );
    let mut groups: HashMap<bool, Vec<i32>> = HashMap::new();
    for n in 1..6 {
        groups.entry(n % 2 == 0).or_insert_with(Vec::new).push(n);
    }
    println!("{:?} {:?}", groups[&true], groups[&false]);

    // Of two: a trait's method on numbers, as a fold's or a sort's.
    let highest = vec![3, 9, 4].into_iter().fold(i32::MIN, i32::max);
    let lowest = vec![5i64, -2, 8].into_iter().fold(i64::MAX, i64::min);
    let widest = vec![1.5, 0.25].into_iter().fold(0.0, f64::max);
    let distances: Vec<i64> = vec![-7i64, 4].into_iter().map(i64::abs).collect();
    let mut sorted = vec![3, 1, 2];
    sorted.sort_by(i32::cmp);
    println!("{} {} {} {:?} {:?}", highest, lowest, widest, sorted, distances);

    // In a variable, called later.
    let measure = str::len;
    let fresh = Vec::<u8>::new;
    println!("{} {:?}", measure("日本"), fresh());
}
