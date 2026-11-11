// A write through a raw pointer into a part of a value taken by value, a
// field of a struct, an item of an array, a field of a tuple, changes the
// copy alone, as a `&mut` of that part does (ADR 0052, ADR 0407, #1386):
// the caller's value, and the one a `let` copied it from, keep theirs.
#[derive(Clone, Copy, Debug)]
struct Point {
    x: i32,
    y: i32,
}

#[derive(Clone, Copy, Debug)]
struct Segment {
    from: Point,
    to: Point,
}

fn moved(mut point: Point, by: i32) -> Point {
    let p = &raw mut point.y;
    unsafe { *p += by };
    point
}

fn stretched(mut segment: Segment, by: i32) -> Segment {
    let p = &raw mut segment.to.x;
    unsafe { *p += by };
    segment
}

fn ended(mut segment: Segment) -> Segment {
    let to = &raw mut segment.to;
    let from = &raw mut segment.from;
    unsafe {
        *to = Point { x: 0, y: 0 };
        (*from).y = -1;
    }
    segment
}

fn bumped(mut items: [u8; 3]) -> [u8; 3] {
    let first = &raw mut items[0];
    unsafe {
        *first += 1;
        *first.add(2) += 5;
    }
    items
}

fn swapped(mut pair: (i32, &'static str)) -> (i32, &'static str) {
    let n = &raw mut pair.0;
    let s = &raw mut pair.1;
    unsafe {
        *n = -*n;
        *s = "changed";
    }
    pair
}

fn rows(mut grid: [[i32; 2]; 2]) -> [[i32; 2]; 2] {
    let corner = &raw mut grid[1][1];
    unsafe { *corner = 99 };
    grid
}

fn main() {
    let start = Point { x: 1, y: 2 };
    println!("{:?} {start:?}", moved(start, 10));

    let segment = Segment {
        from: start,
        to: Point { x: 3, y: 4 },
    };
    println!("{:?} {segment:?}", stretched(segment, 7));
    println!("{:?} {segment:?}", ended(segment));

    let items = [1, 2, 3];
    println!("{:?} {items:?}", bumped(items));

    let pair = (5, "kept");
    println!("{:?} {pair:?}", swapped(pair));

    let grid = [[1, 2], [3, 4]];
    println!("{:?} {grid:?}", rows(grid));

    // A copy a `let` makes, written through a pointer to its field.
    let mut copy = start;
    let x = &raw mut copy.x;
    unsafe { *x = 40 };
    println!("{copy:?} {start:?}");

    let mut more = items;
    let last = &raw mut more[2];
    unsafe { *last = 30 };
    println!("{more:?} {items:?}");
}
