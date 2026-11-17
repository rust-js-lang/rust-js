// `len()` of a `rev()` of a `copied()` or `cloned()` of a slice's `map`,
// made in place: the `rev()` makes the map one that knows its back,
// `$mapEnds(..)`, and its length is counted as a map's, its source's,
// without calling its function (#1978). And a `map(..).rev()` over the
// `copied()`, collected, and the reversed copies stepped and summed. And
// what's counted through such a map, made so by a `rev()` over an
// `enumerate()`, a `step_by(s)` or a `skip(k)`, a slice's `chunks(k)`
// mapped, or a `take(k)` of a reversed copy, of each length to 6 and a step
// known only at run time.
use std::hint::black_box;

fn enumerated(v: &[u32], s: usize) -> usize {
    v.iter().map(|x| x + 1).enumerate().step_by(s).rev().len()
}

fn twice_reversed(v: &[u32], s: usize) -> usize {
    v.iter().map(|x| x + 1).rev().step_by(s).rev().len()
}

fn skipped(v: &[u32], s: usize, k: usize) -> usize {
    v.iter().map(|x| x + 1).skip(k).step_by(s).rev().len()
}

fn chunked(v: &[u32], k: usize) -> usize {
    v.chunks(k).map(|c| c.len()).rev().len()
}

fn cloned_skipped(v: &[u32], k: usize) -> usize {
    v.iter().map(|x| x.min(&9)).cloned().skip(k).rev().len()
}

fn copied_taken(v: &[u32], k: usize) -> usize {
    v.iter().map(|x| x.min(&9)).copied().rev().take(k).len()
}

fn main() {
    let items = [1, 2, 3, 4, 5];
    println!("{}", items.iter().map(|x| x.min(&9)).copied().rev().len());
    println!("{}", items.iter().map(|x| x.min(&3)).cloned().rev().len());
    println!("{}", items.iter().map(|x| x.max(&2)).copied().rev().skip(1).len());
    let n = items.iter().map(|x| x.min(&9)).copied().map(|x| x * 10).rev();
    println!("{:?}", n.collect::<Vec<_>>());
    let total: i32 = items.iter().map(|x| x.min(&4)).copied().rev().sum();
    println!("{total}");
    let mut calls = 0;
    let counted = items
        .iter()
        .map(|x| {
            calls += 1;
            x.min(&9)
        })
        .copied()
        .rev()
        .len();
    println!("{counted} {calls}");
    for n in 0..7u32 {
        let v: Vec<u32> = (0..n).collect();
        for s in 1..4 {
            let s = black_box(s);
            println!(
                "{n} {s} {} {} {} {} {} {}",
                enumerated(&v, s),
                twice_reversed(&v, s),
                skipped(&v, s, black_box(2)),
                chunked(&v, s),
                cloned_skipped(&v, s),
                copied_taken(&v, s)
            );
        }
    }
}
