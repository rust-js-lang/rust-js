// std's `From` conversions, and the `Into` each gives, read as one list of
// what's converted into what: `Some(x)` of an `Option`, a `Vec` of an
// array, an `Rc` or an `Arc` of a value, of text as its `str`, of a `Vec`'s
// items as its `[T]`, as `Rc::new` makes one (ADR 0320).
use std::collections::{BinaryHeap, HashMap, VecDeque};
use std::rc::Rc;
use std::sync::Arc;

fn main() {
    let some: Option<u32> = 5u32.into();
    let from = Option::from("text");
    let nested: Option<Option<u32>> = Option::from(None::<u32>);
    println!("{some:?} {from:?} {nested:?}");

    let v = Vec::from([8u32, 9]);
    let w: Vec<u32> = [1u32, 2].into();
    println!("{v:?} {w:?}");

    let deque = VecDeque::from(vec![3u32, 4]);
    let heap: BinaryHeap<u32> = vec![2u32, 7, 1].into();
    let map = HashMap::from([("k", 1u32)]);
    println!("{deque:?} {:?} {map:?}", heap.peek());

    let rc: Rc<u32> = Rc::from(7u32);
    let text: Rc<str> = Rc::from("hi");
    let owned: Rc<str> = String::from("yo").into();
    let items: Arc<[u32]> = vec![1u32, 2].into();
    let boxed: Arc<Vec<u32>> = vec![5u32].into();
    println!("{rc} {text} {owned} {items:?} {boxed:?}");
}
