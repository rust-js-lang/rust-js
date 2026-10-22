// `scan`'s state, given its closure by `&mut`, is changed in place where
// it's an object, a struct, a tuple or a `Vec`, and through its box where
// it's a number or text, as a `&mut` of each is.
#[derive(Clone, Copy, Debug)]
struct Total {
    sum: u32,
}

fn main() {
    let running: Vec<u32> = [1u32, 2, 3, 4]
        .iter()
        .scan(Total { sum: 0 }, |total, &n| {
            total.sum += n;
            Some(total.sum)
        })
        .collect();
    let fibonacci: Vec<u64> = (0..8)
        .scan((0u64, 1u64), |pair, _| {
            let out = pair.0;
            *pair = (pair.1, pair.0 + pair.1);
            Some(out)
        })
        .collect();
    let seen: Vec<usize> = [3, 1, 4]
        .iter()
        .scan(Vec::new(), |seen, &x| {
            seen.push(x);
            Some(seen.len())
        })
        .collect();
    let words: Vec<String> = ["a", "b", "c"]
        .iter()
        .scan(String::new(), |text, word| {
            text.push_str(word);
            Some(text.clone())
        })
        .collect();
    let counts: Vec<i32> = [5, 6]
        .iter()
        .scan(0, |n, &x| {
            *n += x;
            Some(*n)
        })
        .collect();
    println!("{running:?} {fibonacci:?} {seen:?} {words:?} {counts:?}");
}
