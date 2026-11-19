// `rposition` asks its iterator's length, then folds it from its back by
// its own `try_rfold`, as std's does: an `enumerate()` of the crate's
// iterator asks its source's length once, not at each step (#2142).

struct Src(Vec<u32>);

impl Iterator for Src {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        println!("next");
        if self.0.is_empty() {
            None
        } else {
            Some(self.0.remove(0))
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        println!("hint");
        (self.0.len(), Some(self.0.len()))
    }
}

impl DoubleEndedIterator for Src {
    fn next_back(&mut self) -> Option<u32> {
        println!("back");
        self.0.pop()
    }
}

impl ExactSizeIterator for Src {}

fn generic<I: DoubleEndedIterator<Item = u32> + ExactSizeIterator>(src: I, at: usize) -> Option<usize> {
    src.enumerate().rposition(|(i, _)| i == at)
}

// Of a `&mut` of one, which std's `try_rfold` passes on to it, leaving
// what it didn't take.
fn lent<I: DoubleEndedIterator<Item = u32> + ExactSizeIterator>(src: &mut I, x: u32) -> Option<usize> {
    src.by_ref().enumerate().rposition(|(_, item)| item == x)
}

fn main() {
    println!("{:?}", generic(Src(vec![1, 2, 3]), 7));
    println!("--");
    println!("{:?}", generic(Src(vec![1, 2, 3]), 1));
    println!("--");
    println!("{:?}", Src(vec![1, 2, 3]).rposition(|x| x == 7));
    println!("--");
    let mut src = Src(vec![4, 5, 6]);
    println!("{:?}", lent(&mut src, 5));
    println!("{:?}", src.0);
}
