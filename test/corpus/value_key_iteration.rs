// A map and a set keyed by a struct, a tuple or an enum find their keys
// by value (ADR 0121), and give back the keys themselves as they're
// stepped: `keys()`, `values()`, `iter()`, `into_iter()`, a `for` of a
// reference and `retain`. Each is sorted before it's shown, as a
// `HashMap` iterates in insertion order in JS and in another natively.
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct Point {
    x: i32,
    y: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum Cell {
    Wall,
    Door(u8),
}

fn main() {
    let mut places: HashMap<Point, &str> = HashMap::new();
    places.insert(Point { x: 1, y: 2 }, "a");
    places.insert(Point { x: 3, y: 4 }, "b");
    places.insert(Point { x: 1, y: 2 }, "c");
    let mut keys: Vec<&Point> = places.keys().collect();
    keys.sort();
    println!("{keys:?} {}", keys[0].x + keys[1].y);
    let mut values: Vec<&&str> = places.values().collect();
    values.sort();
    println!("{values:?}");
    let mut pairs: Vec<(&Point, &&str)> = places.iter().collect();
    pairs.sort();
    println!("{pairs:?}");
    let mut seen = Vec::new();
    for (point, name) in &places {
        seen.push(format!("{}:{} {name}", point.x, point.y));
    }
    seen.sort();
    println!("{seen:?}");
    places.retain(|point, _| point.x > 1);
    let mut owned: Vec<(Point, &str)> = places.into_iter().collect();
    owned.sort();
    println!("{owned:?}");

    let cells: HashSet<(Point, Cell)> = [(Point { x: 0, y: 0 }, Cell::Wall), (Point { x: 0, y: 1 }, Cell::Door(7)), (Point { x: 0, y: 0 }, Cell::Wall)].into_iter().collect();
    let mut doors: Vec<u8> = cells
        .iter()
        .filter_map(|(_, cell)| match cell {
            Cell::Door(n) => Some(*n),
            Cell::Wall => None,
        })
        .collect();
    doors.sort();
    let mut all: Vec<&(Point, Cell)> = cells.iter().collect();
    all.sort();
    println!("{} {doors:?} {all:?}", cells.len());
}
