// A B-tree keyed by value, a tuple, or a struct or an enum whose `Ord` is
// derived, is a `$KeyMap` or `$KeySet` in its keys' derived order: two it
// orders `Equal` are equal field by field, as `$key` tells them (ADR 0404).
use std::collections::{BTreeMap, BTreeSet};

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy)]
struct Point {
    x: i32,
    y: i32,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone)]
enum Shape {
    Rect(u8, u8),
    Circle(u8),
    Dot,
}

fn main() {
    let mut grid: BTreeMap<(i32, i32), char> = BTreeMap::new();
    grid.insert((2, 1), 'a');
    grid.insert((1, 5), 'b');
    grid.insert((1, -3), 'c');
    println!("{:?}", grid.insert((2, 1), 'd'));
    println!("{:?} {:?} {}", grid, grid.get(&(1, 5)), grid.contains_key(&(9, 9)));
    println!("{:?}", grid.range((1, 0)..(2, 5)).collect::<Vec<_>>());
    println!("{:?} {:?}", grid.first_key_value(), grid.last_key_value());
    *grid.entry((0, 0)).or_insert('z') = 'y';
    println!("{:?} {:?}", grid.remove(&(1, -3)), grid.pop_first());
    grid.retain(|&(x, _), _| x > 1);
    println!("{:?}", grid);

    let points: BTreeSet<Point> = [Point { x: 3, y: 0 }, Point { x: 1, y: 9 }, Point { x: 1, y: 2 }, Point { x: 3, y: 0 }]
        .into_iter()
        .collect();
    println!("{:?} {:?} {}", points, points.last(), points.len());

    let mut shapes: BTreeMap<Shape, &str> = BTreeMap::new();
    shapes.insert(Shape::Dot, "dot");
    shapes.insert(Shape::Circle(2), "small");
    shapes.insert(Shape::Rect(1, 9), "tall");
    shapes.insert(Shape::Circle(1), "tiny");
    println!("{:?}", shapes.values().collect::<Vec<_>>());

    let mut paths: BTreeSet<(Option<u8>, Vec<u8>)> = BTreeSet::new();
    paths.insert((Some(1), vec![2, 3]));
    paths.insert((None, vec![9]));
    paths.insert((Some(1), vec![2]));
    paths.insert((Some(1), vec![2]));
    println!("{:?}", paths);
}
