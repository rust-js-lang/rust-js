//@ library-refused: a std type holding a value with a destructor
// An `Rc` of a type parameter's value, `Rc<RefCell<TeeBuffer<I::Item, I>>>`
// as itertools' `tee` holds, in a crate where a type may have a destructor,
// is counted, as every `Rc` is then: its last clone drops what it holds,
// its items and its iterator. And `VecDeque::pop_front` of a type
// parameter's item boxes its `Some`, `$shift`, as `pop` does (ADR 0494).
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

#[derive(Clone, Debug)]
struct Loud(u8);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct TeeBuffer<A, I> {
    backlog: VecDeque<A>,
    iter: I,
    owner: bool,
}

struct Tee<I: Iterator> {
    rcbuffer: Rc<RefCell<TeeBuffer<I::Item, I>>>,
    id: bool,
}

fn tee<I: Iterator>(iter: I) -> (Tee<I>, Tee<I>) {
    let buffer = TeeBuffer {
        backlog: VecDeque::new(),
        iter,
        owner: false,
    };
    let t1 = Tee {
        rcbuffer: Rc::new(RefCell::new(buffer)),
        id: true,
    };
    let t2 = Tee {
        rcbuffer: t1.rcbuffer.clone(),
        id: false,
    };
    (t1, t2)
}

impl<I: Iterator> Iterator for Tee<I>
where
    I::Item: Clone,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        let mut buffer = self.rcbuffer.borrow_mut();
        if buffer.owner == self.id {
            if let Some(elt) = buffer.backlog.pop_front() {
                return Some(elt);
            }
        }
        match buffer.iter.next() {
            None => None,
            Some(elt) => {
                buffer.backlog.push_back(elt.clone());
                buffer.owner = !self.id;
                Some(elt)
            }
        }
    }
}

fn main() {
    let (mut a, mut b) = tee(vec![1, 2, 3].into_iter());
    println!("{:?} {:?} {:?}", a.next(), a.next(), b.next());
    let (mut c, d) = tee(vec![Loud(1), Loud(2)].into_iter());
    println!("{:?}", c.next().map(|l| l.0));
    drop(d);
    println!("d dropped");
    println!("{:?}", c.next().map(|l| l.0));
    drop(c);
    let (mut e, mut f) = tee(vec![None, Some(1)].into_iter());
    println!("{:?} {:?} {:?}", e.next(), f.next(), f.next());
    println!("end");
}
