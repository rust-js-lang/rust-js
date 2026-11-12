//@ library-differs
// Room for an impl's associated type's items: of an impl's method, its
// item's, `I::Item`, which its dictionary can't give the size of (#934),
// checked by its count alone, the guide's difference 23; of a built type of
// it, `Option<I::Item>`, and of a trait's default an impl copies,
// `Option<Self::Item>`, by the size the impl's dictionary is made with,
// `wrapperRoom(16)` (ADR 0774, amended, #1432).
struct Wrapper<I>(I);

impl<I: Iterator> Iterator for Wrapper<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        self.0.next()
    }
}

trait Room: Iterator {
    fn items(&self, n: usize) -> usize;

    fn options(&self, n: usize) -> usize;

    fn reserved(&self, n: usize) -> usize {
        let mut v: Vec<Option<Self::Item>> = Vec::new();
        v.reserve(n);
        v.len()
    }
}

impl<I: Iterator> Room for Wrapper<I> {
    fn items(&self, n: usize) -> usize {
        Vec::<I::Item>::with_capacity(n).len()
    }

    fn options(&self, n: usize) -> usize {
        let v: Vec<Option<I::Item>> = std::iter::repeat_with(|| None).take(n).collect();
        let mut w: Vec<Option<I::Item>> = Vec::new();
        w.resize_with(n, || None);
        v.len() + w.len()
    }
}

fn through<R: Room>(r: &R, n: usize) -> (usize, usize, usize) {
    (r.items(n), r.options(n), r.reserved(n))
}

fn main() {
    let w = Wrapper([1u64, 2].into_iter());
    println!("{} {} {}", w.items(3), w.options(3), w.reserved(3));
    println!("{:?}", through(&w, 2));
}
