// What's left of a generic iterator stepped through is taken as it's
// stepped: an iterator of the crate's makes each item when it's asked for,
// not all of them when the loop starts.
struct Counter(u32);

impl Iterator for Counter {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.0 == 4 {
            return None;
        }
        self.0 += 1;
        println!("made {}", self.0);
        Some(self.0)
    }
}

fn after_first<I: Iterator<Item = u32>>(mut it: I) {
    it.next();
    for x in it {
        println!("took {x}");
        if x >= 2 {
            break;
        }
    }
}

fn main() {
    after_first(Counter(0));
}
