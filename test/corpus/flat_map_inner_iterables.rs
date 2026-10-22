// What `flat_map` and `flatten` step of each inner value is its items: a
// range's numbers, the crate's iterator's own `next`, a `Result`'s `Ok`
// value, as `chain` steps them.
struct Count {
    at: u32,
    end: u32,
}

impl Iterator for Count {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        (self.at < self.end).then(|| {
            self.at += 1;
            self.at - 1
        })
    }
}

fn main() {
    let counted: Vec<u32> = (1..4).flat_map(|n| Count { at: 0, end: n }).collect();
    let ranged: Vec<u32> = (1..4).flat_map(|x| 0..x).collect();
    let spans = vec![0..2, 5..7];
    let joined: Vec<i32> = spans.into_iter().flatten().collect();
    println!("{counted:?} {ranged:?} {joined:?}");
    let results = vec![Ok(1), Err("x"), Ok(3)];
    let oks: Vec<i32> = results.into_iter().flatten().collect();
    let total: u32 = Some(Count { at: 0, end: 4 }).into_iter().flatten().sum();
    println!("{oks:?} {total}");
}
