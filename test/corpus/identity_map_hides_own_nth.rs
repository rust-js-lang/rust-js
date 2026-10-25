// An identity `map` over an iterator of the crate's that writes its own
// `nth` and `last` is kept (#235): std's `Map` doesn't pass them on, so a
// `skip`'s first step and `last()` of it don't reach them, as they reach
// them without the map.
struct Count {
    at: i32,
    end: i32,
}

impl Iterator for Count {
    type Item = i32;

    fn next(&mut self) -> Option<i32> {
        if self.at < self.end {
            self.at += 1;
            Some(self.at)
        } else {
            None
        }
    }

    fn nth(&mut self, n: usize) -> Option<i32> {
        println!("nth {n}");
        self.at = self.at.saturating_add(n as i32);
        self.next()
    }

    fn last(self) -> Option<i32> {
        println!("last");
        (self.at < self.end).then_some(self.end)
    }
}

fn count() -> Count {
    Count { at: 0, end: 5 }
}

fn main() {
    let mapped = count().map(|x| x).skip(2).next();
    let direct = count().skip(2).next();
    println!("{mapped:?} {direct:?}");
    let mapped_last = count().map(|x| x).last();
    let direct_last = count().last();
    println!("{mapped_last:?} {direct_last:?}");
}
