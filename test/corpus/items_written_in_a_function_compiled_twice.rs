// A function compiled twice, its JS for JS's callers and a copy its
// crate's Rust calls (ADR 0051, amended), `evens` and `evensRevealed` of
// an `impl Iterator` it gives back (#1999): each copy declares what its
// body writes, a nested fn and a `const`, its own (#2018).
#[derive(Clone, Copy, Debug)]
pub struct P(pub u32, pub u32);

pub fn evens(ps: &[P]) -> impl Iterator<Item = P> + '_ {
    const STEP: u32 = 2;
    fn keep(p: &&P) -> bool {
        p.0 % STEP == 0
    }
    ps.iter().filter(keep).copied()
}

pub fn odds(ps: &[P]) -> impl Iterator<Item = P> + '_ {
    fn keep(p: &&P) -> bool {
        p.0 % 2 == 1
    }
    ps.iter().filter(|p| keep(p)).cloned()
}

fn main() {
    let ps = [P(1, 10), P(2, 20), P(3, 30), P(4, 40)];
    let mut e: Vec<P> = evens(&ps).collect();
    e[0].1 = 7;
    let o: Vec<P> = odds(&ps).collect();
    println!("{e:?} {o:?} {ps:?}");
}
