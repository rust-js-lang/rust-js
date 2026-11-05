// A `step_by(n)` stepped from its back, by `rev()`, `rfold`, `nth_back`,
// `next_back` and `rfind`, and counted, by an `enumerate()` folded from its
// back: `$stepByEnds` of its source made knowing its back and its length,
// as std's `StepBy` is double-ended and exact-sized where its source is
// exact-sized. Of a `Vec`'s items with a destructor, each it passes over
// is dropped by its source's own `nth_back`, as one slice, front to back,
// as std's is (#718).
struct L(u32);

impl Drop for L {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn ls(n: u32) -> Vec<L> {
    (1..=n).map(L).collect()
}

fn main() {
    let v: Vec<L> = ls(3).into_iter().step_by(2).rev().collect();
    println!("{}", v.len());
    drop(v);
    println!("{}", ls(5).into_iter().step_by(2).rfold(0, |n, l| n * 10 + l.0));
    println!("{}", ls(9).into_iter().step_by(3).rev().map(|l| l.0).sum::<u32>());
    println!("{:?}", ls(8).into_iter().step_by(3).nth_back(1).map(|l| l.0));
    println!("{:?}", ls(8).into_iter().step_by(3).next_back().map(|l| l.0));
    println!("{:?}", ls(10).into_iter().step_by(4).rfind(|l| l.0 < 5).map(|l| l.0));
    let folded = ls(9)
        .into_iter()
        .step_by(3)
        .enumerate()
        .rfold(0, |n, (i, l)| n * 10 + i as u32 + l.0);
    println!("{folded}");
    let r: Vec<u32> = ls(7).into_iter().step_by(2).rev().take(2).map(|l| l.0).collect();
    println!("{r:?}");
    // Held in a variable, std's struct, beside them.
    let mut it = ls(7).into_iter().step_by(3);
    println!("{:?}", it.next_back().map(|l| l.0));
    println!("{:?}", it.next().map(|l| l.0));
    // Of numbers, counted.
    let w = vec![1u32, 2, 3, 4, 5, 6, 7, 8];
    println!(
        "{}",
        w.iter()
            .step_by(3)
            .enumerate()
            .rfold(0, |n, (i, x)| n * 10 + i as u32 + x)
    );
    println!("{:?}", w.iter().step_by(3).enumerate().rev().collect::<Vec<_>>());
    println!("end");
}
