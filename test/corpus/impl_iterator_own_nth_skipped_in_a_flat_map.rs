// An iterator of the crate's that writes its own `nth`, returned behind an
// `impl Iterator`, skipped in a `flat_map` and stepped by a short-circuiting
// consumer: std's `Skip::try_fold` asks its source `nth(n - 1)`, as of the
// iterator named by its type. The chain is std's compiled structs either
// way, where behind an `impl Iterator` it was JS's helpers, whose `drop(2)`,
// stepped by `next`, asked `nth(2)` (#2067's single-crate twin).

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
}

fn c() -> impl Iterator<Item = i32> {
    Count { at: 0, end: 20 }
}

fn main() {
    let past = c()
        .take(2)
        .flat_map(|_| c().skip(2))
        .skip_while(|&x| x < 4)
        .any(|x| x > 25);
    println!("{past}");
    println!("{:?}", c().skip(3).find(|&x| x > 5));
}
