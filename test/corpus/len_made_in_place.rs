// `len()` of an iterator made where it's asked, as std counts it: each
// adapter asks its source's, so a `map`'s function isn't called, a
// `take(n)` gives at most `n` and a `skip(n)` `n` fewer (ADR 0986). Of a
// generic one's, its own `len()`, where a zip of it with a slice's items or
// a `take(n)` or `skip(n)` of its copies counts it in place.
fn count<I: ExactSizeIterator>(it: I) -> usize {
    it.len()
}

fn reversed_len<I: DoubleEndedIterator + ExactSizeIterator>(it: I) -> usize {
    it.rev().len()
}

fn mapped_len<I: ExactSizeIterator<Item = u32>>(it: I) -> usize {
    it.map(|x| x + 1).len()
}

fn taken_len<I: ExactSizeIterator>(it: I) -> usize {
    it.take(2).len()
}

fn zipped_len<I: ExactSizeIterator<Item = u32>>(it: I, v: &[u32]) -> usize {
    it.zip(v.iter()).len()
}

fn zipped_after<I: ExactSizeIterator<Item = u32>>(it: I, v: &[u32]) -> usize {
    v.iter().zip(it).len()
}

fn copies_taken<'a, I: ExactSizeIterator<Item = &'a u32>>(it: I) -> usize {
    it.copied().take(2).len()
}

fn copies_skipped<'a, I: ExactSizeIterator<Item = &'a u32>>(it: I) -> usize {
    it.copied().skip(1).len()
}

fn main() {
    let v = vec![1u32, 2, 3, 4, 5];
    let mut calls = 0;
    let n = v
        .iter()
        .map(|x| {
            calls += 1;
            x * 2
        })
        .len();
    println!("{n} {calls}");
    println!("{}", v.iter().map(|x| x * 2).rev().len());
    println!("{} {}", (1..4u32).rev().len(), (2..=9u8).rev().len());
    println!("{} {}", v.iter().enumerate().len(), v.iter().enumerate().rev().len());
    println!("{}", v.iter().copied().map(|x| x + 1).enumerate().len());
    println!("{} {}", v.iter().skip(1).rev().len(), v.iter().skip(9).len());
    println!("{} {}", v.iter().take(2).map(|x| x + 1).len(), v.iter().take(9).len());
    println!("{}", v.clone().into_iter().map(|x| x + 1).len());
    println!("{}", (0..10u32).skip(3).take(4).rev().len());
    println!("{} {}", count(v.iter().map(|x| x * 2).rev()), count(v.iter().rev()));
    let k = 2;
    println!("{}", v.iter().skip(k).map(|x| x - 1).len());
    println!("{}", v.iter().skip(k + 7).map(|x| x - 1).len());
    println!("{} {}", count((1..4u32).rev()), count(v.iter().enumerate().rev()));
    println!(
        "{} {} {}",
        reversed_len(v.iter()),
        mapped_len(v.iter().copied()),
        taken_len(v.iter())
    );
    println!(
        "{} {} {}",
        reversed_len(1..9u32),
        mapped_len(4..9u32),
        taken_len(0..1u32)
    );
    println!(
        "{} {} {} {}",
        zipped_len(v.clone().into_iter(), &v[1..]),
        zipped_after(3..9u32, &v),
        copies_taken(v.iter()),
        copies_skipped(v[4..].iter())
    );
    println!(
        "{} {} {} {}",
        zipped_len(std::iter::empty(), &v),
        zipped_after([7u32].into_iter(), &[]),
        copies_taken([].iter()),
        copies_skipped([].iter())
    );
}
