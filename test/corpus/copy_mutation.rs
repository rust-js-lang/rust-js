// A copy is its own value: changing it, or the original, leaves the other
// as it was, however deep the value is.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Point {
    x: i32,
    y: i32,
}

#[derive(Debug, Clone)]
struct Shape {
    name: String,
    points: Vec<Point>,
}

fn moved(mut p: Point) -> Point {
    p.x += 100;
    p
}

// A generic function's clone of a type changed in place elsewhere, as
// `Tagged<Meters>` and `Holder<Numbers>` are below: its fields are known,
// but which of its types it's given isn't.
#[derive(Clone)]
struct Tagged<T> {
    n: i32,
    unit: std::marker::PhantomData<T>,
}

#[derive(Clone)]
struct Meters;

fn duplicate<T: Clone>(t: &Tagged<T>) -> Tagged<T> {
    t.clone()
}

#[derive(Clone)]
struct Holder<T: Iterator> {
    item: T::Item,
}

#[derive(Clone)]
struct Numbers;

impl Iterator for Numbers {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        None
    }
}

fn held<T: Iterator<Item = u32> + Clone>(h: &Holder<T>) -> Holder<T> {
    h.clone()
}

fn main() {
    let a = Point { x: 1, y: 2 };
    let mut b = a;
    b.x = 10;
    println!("{a:?} {b:?}");

    let mut grid = [[0, 0, 0], [0, 0, 0]];
    let row = grid[0];
    grid[0][1] = 5;
    println!("{grid:?} {row:?}");

    let shape = Shape { name: "tri".to_string(), points: vec![a, b] };
    let mut copy = shape.clone();
    copy.name.push_str("-copy");
    copy.points[0].y = -1;
    copy.points.push(Point { x: 0, y: 0 });
    println!("{shape:?}\n{copy:?}");

    let c = moved(a);
    println!("{a:?} {c:?} {}", a == Point { x: 1, y: 2 });

    let mut points = vec![a; 3];
    for p in points.iter_mut() {
        p.y *= 3;
    }
    let first = points[0];
    points[0].x = 42;
    println!("{points:?} {first:?}");

    let mut nested = vec![vec![1, 2], vec![3]];
    let snapshot = nested.clone();
    nested[0].push(9);
    nested[1][0] = 0;
    println!("{nested:?} {snapshot:?}");

    let tagged = Tagged::<Meters> { n: 1, unit: std::marker::PhantomData };
    let mut copy = duplicate(&tagged);
    copy.n += 1;
    println!("{} {}", tagged.n, copy.n);

    let holder = Holder::<Numbers> { item: 1 };
    let mut copy = held(&holder);
    copy.item += 1;
    println!("{} {}", holder.item, copy.item);

    // An array of what's changed in place is copied as it is: what a
    // closure given its items by value changes, and a copy's item, leave
    // the array they came from as it was.
    let points = [Point { x: 1, y: 2 }, Point { x: 3, y: 4 }];
    let moved: Vec<Point> = points.into_iter().map(|mut p| {
        p.x += 10;
        p
    }).collect();
    let mut copied = points;
    copied[0].y = 0;
    println!("{points:?} {moved:?} {copied:?}");
}
