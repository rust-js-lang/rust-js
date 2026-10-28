// A zip owns its sides, as std's `Zip` does: one that ends first keeps what
// it has left until the zip is dropped, which drops `a`'s, then `b`'s. A
// side that ends at a `None` of its own, a `take_while`'s, a `scan`'s, a
// `map_while`'s, a `take`'s or a `skip`'s past them, isn't dropped there,
// before the zip's other item is; and a zip a variable holds, stepped to
// its end, drops both only where the variable is dropped.
struct Loud(u8);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn sides() -> (Vec<Loud>, Vec<Loud>) {
    (
        vec![Loud(10), Loud(11), Loud(12), Loud(13)],
        vec![Loud(1), Loud(3), Loud(2), Loud(5)],
    )
}

fn odd(l: &Loud) -> bool {
    l.0 % 2 == 1
}

fn ended_by_a_side() {
    let (a, b) = sides();
    let got: Vec<(u8, u8)> = a
        .into_iter()
        .map(|l| l.0)
        .zip(b.into_iter().take_while(odd).map(|l| l.0))
        .collect();
    println!("take_while {got:?}");

    let (a, b) = sides();
    let got: Vec<(u8, u8)> = a
        .into_iter()
        .map(|l| l.0)
        .zip(b.into_iter().scan((), |_, l| odd(&l).then_some(l.0)))
        .collect();
    println!("scan {got:?}");

    let (a, b) = sides();
    let got: Vec<(u8, u8)> = a
        .into_iter()
        .map(|l| l.0)
        .zip(b.into_iter().map_while(|l| odd(&l).then_some(l.0)))
        .collect();
    println!("map_while {got:?}");

    let (a, b) = sides();
    let got: Vec<(u8, u8)> = a
        .into_iter()
        .map(|l| l.0)
        .zip(b.into_iter().filter(odd).take(1).map(|l| l.0))
        .collect();
    println!("take {got:?}");

    let (a, b) = sides();
    let got: Vec<(u8, u8)> = a
        .into_iter()
        .map(|l| l.0)
        .zip(b.into_iter().take_while(odd).skip(3).map(|l| l.0))
        .collect();
    println!("skip {got:?}");
}

fn held_to_its_end() {
    let (a, b) = sides();
    let mut zipped = a.into_iter().filter(odd).zip(b.into_iter().filter(odd));
    while let Some((x, y)) = zipped.next() {
        println!("pair {} {}", x.0, y.0);
    }
    println!("zip done");
    drop(zipped);
    println!("zip dropped");
}

fn main() {
    ended_by_a_side();
    held_to_its_end();
}
