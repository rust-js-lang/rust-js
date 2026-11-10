// `take(n)` and `filter(f)` of a type parameter's iterator, made in generic
// code where their hint is asked, JS's own adapters, which know none: std's
// `Take`'s and `Filter`'s of their source's, `$takeHint(i, 2)` and
// `$upperOnly(i)`, of each instance a caller gives (#1236).
struct Hinted(u32);
impl Iterator for Hinted {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.0 == 0 {
            None
        } else {
            self.0 -= 1;
            Some(self.0)
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.0 as usize, Some(self.0 as usize + 5))
    }
}

fn taken<I: Iterator<Item = u32>>(i: I) -> (usize, Option<usize>) {
    i.take(2).size_hint()
}

fn filtered<I: Iterator<Item = u32>>(i: I) -> (usize, Option<usize>) {
    i.filter(|x| *x > 0).size_hint()
}

fn none_taken<I: Iterator<Item = u32>>(i: I) -> (usize, Option<usize>) {
    i.take(0).size_hint()
}

fn main() {
    println!(
        "{:?} {:?} {:?}",
        taken(vec![1u32, 2, 3].into_iter()),
        taken([4u32, 5].iter().copied()),
        taken(0..10u32)
    );
    println!(
        "{:?} {:?} {:?}",
        taken(Hinted(1)),
        taken(Hinted(9)),
        taken(std::iter::repeat(1u32))
    );
    println!(
        "{:?} {:?} {:?}",
        filtered(vec![1u32, 0, 2].into_iter()),
        filtered(0..4u32),
        filtered(Hinted(3))
    );
    println!("{:?} {:?}", none_taken(0..4u32), none_taken(std::iter::repeat(2u32)));
}
