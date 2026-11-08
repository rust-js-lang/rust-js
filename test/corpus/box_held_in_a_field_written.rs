// A box held in a field, `*w.0`, and a field of a box, `(*b).x`, written in
// place after a read of it: the read keeps the value it read, as a box's
// own `*n` does (#980), where it was moved past the write, MIR's place of
// each reached through the box's deref, which isn't a reference's: a box
// is the value it holds, its local's own (#1009).
struct Wrapped(Box<i32>);

struct Point {
    x: i32,
    y: i32,
}

struct Named {
    count: Box<u32>,
    label: &'static str,
}

fn pair(a: i32, b: i32) -> (i32, i32) {
    (a, b)
}

fn main() {
    let mut w = Wrapped(Box::new(5));
    println!(
        "{:?}",
        pair(*w.0, {
            *w.0 += 1;
            *w.0
        })
    );
    let mut p = Box::new(Point { x: 1, y: 2 });
    println!(
        "{:?}",
        pair(p.x, {
            p.x += 10;
            p.x
        })
    );
    let before = p.y;
    p.y = 20;
    println!("{before} {} {}", p.x, p.y);
    let mut named = Named {
        count: Box::new(0),
        label: "n",
    };
    for i in 1..=3 {
        let was = *named.count;
        *named.count += i;
        println!("{} {was} -> {}", named.label, named.count);
    }
    let mut nested = Box::new(Wrapped(Box::new(7)));
    let seen = *nested.0;
    *nested.0 = 70;
    println!("{seen} {}", nested.0);
}
