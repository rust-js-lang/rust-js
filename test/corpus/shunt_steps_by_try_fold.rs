// A collect into an `Option` or a `Result` of a `Vec` steps its source as
// std's `GenericShunt` does, by its `try_fold`, broken at each item: a
// `skip(1)` of the crate's iterator asks its `nth(0)` and then steps it,
// where `Skip::next` would ask `nth(1)`; a `step_by(2)`'s first item is its
// `next`, where `StepBy::next` would ask `nth(0)` (#1004).
struct Counting {
    at: u32,
    end: u32,
}

impl Iterator for Counting {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        println!("next");
        if self.at == self.end {
            return None;
        }
        self.at += 1;
        Some(self.at)
    }

    fn nth(&mut self, n: usize) -> Option<u32> {
        println!("nth {n}");
        for _ in 0..n {
            self.next()?;
        }
        self.next()
    }
}

impl Drop for Counting {
    fn drop(&mut self) {
        println!("drop at {}", self.at);
    }
}

fn main() {
    let skipped: Option<Vec<u32>> = Counting { at: 0, end: 4 }.skip(1).map(Some).collect();
    println!("{skipped:?}");
    let stepped: Result<Vec<u32>, ()> = Counting { at: 0, end: 5 }.step_by(2).map(Ok).collect();
    println!("{stepped:?}");
    let taken: Option<Vec<u32>> = Counting { at: 0, end: 5 }.take_while(|n| *n < 3).map(Some).collect();
    println!("{taken:?}");
    let plain: Vec<u32> = Counting { at: 0, end: 3 }.skip(1).collect();
    println!("{plain:?}");
}
