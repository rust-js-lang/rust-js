// An `Option` of what could look like `None` changed in place: what it's
// given held boxed, `$some(value)`, and the `&mut` it gives back a handle
// that reads and writes through the box, `$someMut`; `take_if`'s taken one
// given as it's held, box and all (ADR 0486).
struct Product<I: Iterator> {
    a: I,
    a_cur: Option<Option<I::Item>>,
}

impl<I: Iterator> Product<I>
where
    I::Item: Clone,
{
    fn current(&mut self) -> Option<I::Item> {
        let a = &mut self.a;
        self.a_cur.get_or_insert_with(|| a.next()).as_ref().cloned()
    }

    fn advance(&mut self) {
        self.a_cur = None;
    }
}

fn main() {
    let mut p = Product { a: vec![1, 2].into_iter(), a_cur: None };
    println!("{:?} {:?}", p.current(), p.current());
    p.advance();
    println!("{:?}", p.current());
    p.advance();
    println!("{:?} {:?}", p.current(), p.a_cur);

    let mut o: Option<Option<i32>> = None;
    *o.insert(None) = Some(4);
    println!("{o:?}");
    let r = o.get_or_insert(Some(9));
    *r = None;
    println!("{o:?}");

    let mut d: Option<Option<i32>> = None;
    *d.get_or_insert_default() = Some(2);
    println!("{d:?}");

    let mut e: Option<Option<String>> = None;
    e.get_or_insert(None);
    println!("{e:?}");

    let mut u: Option<Option<i32>> = Some(Some(1));
    let taken = u.take_if(|v| v.is_some());
    println!("{taken:?} {u:?}");
    let mut n: Option<Option<i32>> = Some(None);
    let kept = n.take_if(|v| v.is_some());
    println!("{kept:?} {n:?}");
}
