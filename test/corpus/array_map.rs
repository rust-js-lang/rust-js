// `[T; N]::map` is a new array of what `f` makes of each item, in order: an
// array's own `map` in JS. Each item is given by value, so changing it in
// the closure leaves the array it came from as it was.
#[derive(Clone, Copy, Debug)]
struct P {
    x: i32,
}

fn double(n: i32) -> i32 {
    n * 2
}

fn main() {
    let numbers = [1, 2, 3];
    println!("{:?} {:?}", numbers.map(|n| n + 1), numbers.map(double));
    println!("{:?}", numbers.map(|n| format!("#{n}")));
    let nested = [Some(Some(1)), Some(None), None];
    println!("{:?}", nested.map(|o| o.flatten()));
    let mut seen = Vec::new();
    let squares = numbers.map(|n| {
        seen.push(n);
        n * n
    });
    println!("{squares:?} {seen:?}");
    let points = [P { x: 1 }, P { x: 2 }];
    let moved = points.map(|mut p| {
        p.x += 10;
        p
    });
    println!("{points:?} {moved:?}");
    // So is an array iterated by value: its items are copies.
    let iterated: Vec<P> = points.into_iter().map(|mut p| {
        p.x += 20;
        p
    }).collect();
    println!("{points:?} {iterated:?}");
}
