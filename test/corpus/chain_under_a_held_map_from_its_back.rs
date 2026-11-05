// A chain of the crate's iterators under a map stepped from its back, which
// MIR may hold in a variable before the map is made: a map whose closure
// owns what has a destructor, held to drop it, then reversed; a map's
// `rfold` given a call's value; a held map's `next_back` and `nth_back`;
// and a filter's map reversed. The chain is made knowing its back where
// it's made, as the map that steps it is (#739, `with_stepped_sources`).
struct Own(u32, &'static str);

impl Drop for Own {
    fn drop(&mut self) {
        println!("drop {}", self.1);
    }
}

impl Iterator for Own {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.0 == 0 {
            return None;
        }
        self.0 -= 1;
        Some(self.0)
    }
}

impl DoubleEndedIterator for Own {
    fn next_back(&mut self) -> Option<u32> {
        self.next()
    }
}

struct Tag(&'static str);

impl Drop for Tag {
    fn drop(&mut self) {
        println!("drop tag {}", self.0);
    }
}

fn mk() -> String {
    String::from(">")
}

fn main() {
    let s = Own(3, "a")
        .chain(Own(2, "b"))
        .map(|x| x + 1)
        .rfold(mk(), |a, x| format!("{a}{x}"));
    println!("{s}");
    let t = Tag("t");
    let m = Own(2, "c").chain(Own(2, "d")).map(move |x| {
        let _ = &t;
        x * 2
    });
    println!("{:?}", m.rev().collect::<Vec<_>>());
    let mut it = Own(2, "e").chain(Own(1, "f")).map(|x| x + 7);
    println!("{:?} {:?}", it.next_back(), it.nth_back(0));
    let v = vec![Tag("v1"), Tag("v2")];
    let w = Own(2, "g")
        .chain(Own(1, "h"))
        .filter(|x| *x > 0)
        .map(|x| x + v.len() as u32);
    println!("{:?}", w.rev().collect::<Vec<_>>());
}
