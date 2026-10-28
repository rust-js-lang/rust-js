// `len()` of an iterator made where it's asked, as std counts it: each
// adapter asks its source's, so a `map`'s function isn't called, a
// `take(n)` gives at most `n` and a `skip(n)` `n` fewer (ADR 0986).
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
}
