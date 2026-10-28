// A zip whose sides are `TrustedLen` but not both `TrustedRandomAccess`,
// folded, `collect()`'s: std's `SpecFold` steps the shorter side's count of
// pairs and no more, so the longer side's closure runs once fewer than in a
// `for`, which steps by `next()`: `collect()`, `sum()`, `count()`,
// `for_each`, `last()` and a held zip's `fold` (#380).
fn main() {
    let v = [1, 2, 3];
    let w = [10, 20];
    let mut calls = 0;
    let folded: Vec<(i32, i32)> = v
        .iter()
        .map(|&x| {
            calls += 1;
            x
        })
        .zip(vec![7, 8])
        .collect();
    println!("{folded:?} {calls}");
    calls = 0;
    for p in v
        .iter()
        .map(|&x| {
            calls += 1;
            x
        })
        .zip(vec![7, 8])
    {
        let _ = p;
    }
    println!("{calls}");
    calls = 0;
    let back: Vec<(i32, &i32)> = v
        .iter()
        .rev()
        .map(|&x| {
            calls += 1;
            x
        })
        .zip(w.iter())
        .collect();
    println!("{back:?} {calls}");

    calls = 0;
    let s: i32 = v
        .iter()
        .map(|&x| {
            calls += 1;
            x
        })
        .zip(vec![7, 8])
        .map(|(a, b)| a + b)
        .sum();
    println!("{s} {calls}");
    calls = 0;
    let n = v
        .iter()
        .map(|&x| {
            calls += 1;
            x
        })
        .zip(vec![7, 8])
        .count();
    println!("{n} {calls}");
    calls = 0;
    v.iter()
        .map(|&x| {
            calls += 1;
            x
        })
        .zip(vec![7, 8])
        .for_each(|_| ());
    println!("{calls}");
    calls = 0;
    let l = v
        .iter()
        .map(|&x| {
            calls += 1;
            x
        })
        .zip(vec![7, 8])
        .last();
    println!("{l:?} {calls}");

    calls = 0;
    let z = v
        .iter()
        .rev()
        .map(|&x| {
            calls += 1;
            x
        })
        .zip(w.iter());
    println!("{:?}", z.size_hint());
    let n = z.fold(0, |a, _| a + 1);
    println!("{n} {calls}");
}
