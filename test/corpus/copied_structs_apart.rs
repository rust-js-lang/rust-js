// A `Copy` struct that std copies out of a slice or a map is a value of
// its own: changing the copy leaves the original, by `copied()` and its
// adapters, `extend` of references, `copy_from_slice`, `repeat`, `concat`
// and an array made of a slice.
use std::collections::{BTreeMap, VecDeque};

#[derive(Clone, Copy, Debug)]
struct Point {
    x: i32,
}

fn main() {
    let points = vec![Point { x: 0 }, Point { x: 5 }];
    let mut copies: Vec<Point> = points.iter().copied().collect();
    copies[0].x = 1;
    let mut first = points.iter().copied().next().unwrap();
    first.x = 2;
    let mut found = points.iter().copied().find(|p| p.x == 5).unwrap();
    found.x = 3;
    let mut reversed: Vec<Point> = points.iter().copied().rev().take(1).collect();
    reversed[0].x = 4;
    let (mut small, _): (Vec<Point>, Vec<Point>) = points.iter().partition(|p| p.x < 3);
    small[0].x = 6;
    let mut queue: VecDeque<Point> = points.iter().copied().collect();
    queue[0].x = 7;
    println!("{points:?}");

    let mut extended: Vec<Point> = Vec::new();
    extended.extend(&points);
    extended[0].x = 8;
    let mut overwritten = [Point { x: 9 }, Point { x: 9 }];
    overwritten.copy_from_slice(&points);
    overwritten[0].x = 10;
    let mut repeated = points.repeat(2);
    repeated[0].x = 11;
    let mut joined = [&points[..], &points[..1]].concat();
    joined[0].x = 12;
    let mut array: [Point; 2] = points[..].try_into().unwrap();
    array[0].x = 13;
    println!("{points:?}");

    let by_name = BTreeMap::from([("a", Point { x: 0 })]);
    let mut values: Vec<Point> = by_name.values().copied().collect();
    values[0].x = 14;
    println!("{by_name:?} {values:?}");
}
