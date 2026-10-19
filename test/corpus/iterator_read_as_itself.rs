// A generic iterator another bound reads as its own type, `I: Iterator +
// Clone + Debug`, given as it is, the crate's own its value, so its `Clone`
// and `Debug` read it, and stepped by its `Iterator` dictionary (ADR 0474).
#[derive(Clone, Debug)]
struct Count(u32);

impl Iterator for Count {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        if self.0 < 3 {
            self.0 += 1;
            Some(self.0)
        } else {
            None
        }
    }
}

fn dup<I: Iterator + Clone + std::fmt::Debug>(mut it: I) -> (Vec<I::Item>, Vec<I::Item>) {
    it.next();
    println!("{it:?}");
    (it.clone().collect(), it.collect())
}

fn main() {
    println!("{:?}", dup(Count(0)));
    println!("{:?}", dup(vec![1, 2, 3].into_iter()));
    println!("{:?}", dup([4, 5, 6].iter()));
}
