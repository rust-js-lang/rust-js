// The crate's own iterator with a `Drop`, collected into an `Option` or a
// `Result` of a `Vec`, std's `GenericShunt`: dropped once, as the collect
// ends, whether its items drop anything or not, and where a `None` or an
// `Err` stops it. Of items that drop nothing, it was closed by the collect
// and dropped again after it (#1647).
struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop loud {}", self.0);
    }
}

struct Source(u32);

impl Iterator for Source {
    type Item = Option<u32>;

    fn next(&mut self) -> Option<Option<u32>> {
        if self.0 == 0 {
            None
        } else {
            self.0 -= 1;
            Some((self.0 != 5).then_some(self.0))
        }
    }
}

impl Drop for Source {
    fn drop(&mut self) {
        println!("drop Source at {}", self.0);
    }
}

struct Louds(u32);

impl Iterator for Louds {
    type Item = Option<Loud>;

    fn next(&mut self) -> Option<Option<Loud>> {
        if self.0 == 0 {
            None
        } else {
            self.0 -= 1;
            Some((self.0 != 1).then(|| Loud(self.0)))
        }
    }
}

impl Drop for Louds {
    fn drop(&mut self) {
        println!("drop Louds at {}", self.0);
    }
}

fn main() {
    let empty: Option<Vec<u32>> = Source(0).collect();
    println!("{empty:?}");
    let two: Option<Vec<u32>> = Source(2).collect();
    println!("{two:?}");
    let stopped: Option<Vec<u32>> = Source(8).collect();
    println!("{stopped:?}");
    let results: Result<Vec<u32>, ()> = Source(3).map(|x| x.ok_or(())).collect();
    println!("{results:?}");
    let louds: Option<Vec<Loud>> = Louds(1).collect();
    println!("{}", louds.map_or(0, |v| v.len()));
    let stopped_louds: Option<Vec<Loud>> = Louds(4).collect();
    println!("{}", stopped_louds.is_none());
}
