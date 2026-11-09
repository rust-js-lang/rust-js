// A chain of a type the crate clones is cloned apart, made by a function of
// its sides that its clone calls again (ADR 0525); another chain of that
// type stepped from its back beneath a map, a filter or as it is is made
// knowing its back by that function, its sides made knowing theirs, as
// one not cloned apart is (#1002).
#[derive(Clone)]
struct Countdown(u32);

impl Iterator for Countdown {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        if self.0 == 0 {
            return None;
        }
        self.0 -= 1;
        Some(self.0)
    }
}

impl DoubleEndedIterator for Countdown {
    fn next_back(&mut self) -> Option<u32> {
        self.next().map(|n| n + 100)
    }
}

fn main() {
    let v = vec![1, 2, 3];
    let w = vec![10, 20];
    // Cloned, each of the three types.
    let slices = v.iter().chain(w.iter());
    let ahead = slices.clone().nth(1);
    println!("{ahead:?} {}", slices.count());
    let ranges = (0..3).chain(7..9);
    let copied: Vec<i32> = ranges.clone().collect();
    println!("{copied:?} {}", ranges.sum::<i32>());
    let own = (0..2).chain(5..6);
    println!("{:?}", own.clone().rev().collect::<Vec<i32>>());

    // Another of each, stepped from its back.
    let mapped = v.iter().chain(w.iter()).map(|x| x * 2).next_back();
    let filtered = v.iter().chain(w.iter()).filter(|x| **x < 15).next_back();
    let last = v.iter().chain(w.iter()).next_back();
    println!("{mapped:?} {filtered:?} {last:?}");
    let ranged = (0..3).chain(7..9).map(|x| x + 1).rfold(0, |a, x| a * 10 + x);
    let found = (0..3).chain(7..9).rfind(|x| x % 2 == 0);
    println!("{ranged} {found:?}");
    let counted = Countdown(3).chain(Countdown(2)).map(|n| n * 3).next_back();
    let both = Countdown(2).chain(Countdown(1));
    let again: Vec<u32> = both.clone().rev().collect();
    println!("{counted:?} {again:?} {}", both.count());
}
