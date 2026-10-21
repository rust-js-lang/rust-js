// A range, or a type parameter's iterator, mapped by a function it's given,
// not a closure made there, folded from its back and reversed: its items
// from its end, each the function's, as Rust's `Map` steps them (ADR 0535).
fn padded<F: FnMut(usize) -> String>(from: usize, to: usize, filler: F) -> Vec<String> {
    (from..to).map(filler).rfold(Vec::new(), |mut all, s| {
        all.push(s);
        all
    })
}

fn reversed<F: FnMut(u32) -> u32>(range: std::ops::Range<u32>, f: F) -> (Option<u32>, Vec<u32>) {
    let mut back = range.map(f).rev();
    let last = back.next();
    (last, back.collect())
}

fn tripled_back<I: DoubleEndedIterator<Item = u32>, F: FnMut(u32) -> u32>(items: I, f: F) -> Vec<u32> {
    items.map(f).rfold(Vec::new(), |mut all, x| {
        all.push(x * 3);
        all
    })
}

fn main() {
    println!("{:?}", padded(1, 4, |i| format!("<{i}>")));
    println!("{:?}", padded(2, 2, |i| i.to_string()));
    println!("{:?}", reversed(1..5, |x| x * 10));
    println!("{:?}", reversed(3..3, |x| x + 1));
    println!("{:?}", tripled_back(vec![1, 2, 3].into_iter(), |x| x + 1));
    println!("{:?}", tripled_back(5..7, |x| x * 2));
}
