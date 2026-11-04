// What `rfold` steps from its back is closed as it ends and as a panic in
// its closure unwinds, as `rfold(self)` drops what it took: a `Vec`'s rest,
// and the crate's double-ended iterator with a `Drop`; and a generic
// `rev()` of the crate's iterator is closed, dropping it, as `Rev` drops
// what it holds (#650).
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Own(u32);

impl Drop for Own {
    fn drop(&mut self) {
        println!("drop own");
    }
}

impl Iterator for Own {
    type Item = D;
    fn next(&mut self) -> Option<D> {
        if self.0 == 0 {
            return None;
        }
        self.0 -= 1;
        Some(D(self.0))
    }
}

impl DoubleEndedIterator for Own {
    fn next_back(&mut self) -> Option<D> {
        self.next()
    }
}

fn any<I: DoubleEndedIterator<Item = D>>(it: I) -> bool {
    it.rev().chain(vec![D(6)]).any(|d| d.0 == 1)
}

fn main() {
    let r = std::panic::catch_unwind(|| {
        vec![D(1), D(2), D(3)]
            .into_iter()
            .rfold(0, |a, d| if d.0 == 2 { panic!("f") } else { a + d.0 })
    });
    println!("{}", r.is_err());
    println!("{}", vec![D(4), D(5), D(6)].into_iter().rfold(0, |a, d| a + d.0));
    let r = std::panic::catch_unwind(|| Own(3).rfold(0, |a, d| if d.0 == 1 { panic!("f") } else { a + d.0 }));
    println!("{}", r.is_err());
    println!("{}", Own(2).rfold(0, |a, d| a + d.0));
    println!("{}", any(Own(3)));
}
