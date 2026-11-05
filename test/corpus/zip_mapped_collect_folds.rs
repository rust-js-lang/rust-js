// A `map()` or an `enumerate()` of a zip std folds as a whole, of
// `TrustedLen` sides not read by index whose first side's step gives an
// item with a destructor, collected into a `Vec`: std's `extend_trusted`
// folds it, its fold the zip's, which takes the shorter side's count of
// pairs, so the longer side's rest is dropped as one slice with it. Stepped
// by `next`, the zip would take one item more of its first side and drop
// it first (#654). An `inspect()` isn't `TrustedLen`: std steps it by
// `next` as it's collected.
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn ds() -> Vec<D> {
    vec![D(1), D(2), D(3), D(4), D(5)]
}

fn main() {
    let v: Vec<u32> = ds().into_iter().rev().zip(10..13u32).map(|(d, n)| d.0 * 100 + n).collect();
    println!("{v:?}");
    let v: Vec<u32> = ds().into_iter().zip(10..13u32).map(|(d, n)| d.0 * 100 + n).collect();
    println!("{v:?}");
    let v: Vec<(usize, u32)> = ds()
        .into_iter()
        .rev()
        .zip(10..12u32)
        .enumerate()
        .map(|(i, (d, n))| (i, d.0 + n))
        .collect();
    println!("{v:?}");
    let mut seen = 0;
    let v: Vec<(D, u32)> = ds()
        .into_iter()
        .rev()
        .zip(10..12u32)
        .inspect(|_| seen += 1)
        .collect();
    println!("{} {seen}", v.len());
    // Folded, an inspect's is its zip's fold too.
    let mut seen = 0;
    let n = ds()
        .into_iter()
        .rev()
        .zip(10..12u32)
        .inspect(|_| seen += 1)
        .fold(0, |n, (d, m)| n + d.0 + m);
    println!("{n} {seen}");
}
