// A slice's or a `VecDeque`'s `iter_mut()` kept in a variable and stepped:
// each `&mut` it gives writes its item.
use std::collections::VecDeque;

#[derive(Debug)]
struct Point {
    x: i32,
}

fn main() {
    let mut v: Vec<i32> = (1..=6).collect();
    let mut it = v.iter_mut();
    if let Some(first) = it.next() {
        *first -= 1;
    }
    if let Some(last) = it.next_back() {
        *last *= 10;
    }
    println!("{}", it.len());
    for x in it.by_ref().take(2) {
        *x += 100;
    }
    println!("{}", it.count());
    println!("{v:?}");

    let mut part = v[2..].iter_mut();
    while let Some(x) = part.next() {
        *x = -*x;
    }
    println!("{v:?}");

    let mut names: Vec<String> = vec!["a".into(), "b".into(), "c".into()];
    let mut each = names.iter_mut();
    each.next().unwrap().push('!');
    let rest: Vec<&mut String> = each.collect();
    for name in rest {
        name.insert(0, '>');
    }
    println!("{names:?}");

    let mut points = [Point { x: 1 }, Point { x: 2 }];
    let mut walk = points.iter_mut();
    walk.next_back().unwrap().x = 9;
    println!("{points:?}");

    let mut queue: VecDeque<u8> = [1, 2, 3].into();
    queue.push_front(0);
    let mut q = queue.iter_mut();
    *q.next().unwrap() += 10;
    for x in q {
        *x *= 2;
    }
    println!("{queue:?}");
}
