// A `Copy` struct changed in place somewhere, given to functions that only
// read it: uncopied where nothing can change the caller's value during
// the call; copied where the callee changes it, or where a closure, a
// `&mut` or a raw pointer can change the caller's while it runs (#1801).
#[derive(Clone, Copy, Debug)]
struct Controls {
    low: f64,
    high: f64,
}

struct Holder {
    c: Controls,
}

fn normalize(mut c: Controls) -> Controls {
    if c.low > c.high {
        std::mem::swap(&mut c.low, &mut c.high);
    }
    c.low = c.low.max(0.0);
    c
}

// Changes what it's given in place, and gives back none of it.
fn clamped_width(mut c: Controls) -> f64 {
    c.low = c.low.max(0.0);
    c.high - c.low
}

fn incidence_at(c: Controls, y: f64) -> f64 {
    (y - c.low) / (c.high - c.low)
}

fn lamp_at(c: Controls, y: f64) -> f64 {
    let t = incidence_at(c, y);
    t * 2.0
}

// Reads its parameter before and after calling what it's given.
fn spread_around(c: Controls, mut bump: impl FnMut()) -> f64 {
    let before = c.high;
    bump();
    c.high - before
}

fn spread_through(c: Controls, other: &mut Controls) -> f64 {
    let before = c.low;
    other.low += 10.0;
    c.low - before
}

fn spread_raw(c: Controls, other: *mut Controls) -> f64 {
    let before = c.low;
    unsafe { (*other).low += 5.0 };
    c.low - before
}

fn main() {
    let c = normalize(Controls { low: 4.0, high: 1.0 });
    println!("{}", lamp_at(c, 2.5));
    // Callees that change what they're given: copied.
    let a = Controls { low: 3.0, high: 2.0 };
    let n = normalize(a);
    println!("{:?} {:?}", a, n);
    let b = Controls { low: -1.0, high: 2.0 };
    let width = clamped_width(b);
    println!("{width} {:?}", b);
    // Copied into what's made beside the call, then changed in place.
    let mut held = Holder { c };
    let at = incidence_at(c, 2.0);
    held.c.low = 9.0;
    println!("{at} {:?} {:?}", held.c, c);
    // Values something can change while the call runs: copied.
    let mut d = c;
    let moved = spread_around(d, || d.high += 1.0);
    println!("{moved} {:?}", d);
    let mut e = c;
    let r = &mut e;
    let through = spread_through(*r, r);
    println!("{through} {:?}", e);
    let mut f = c;
    let p = &raw mut f;
    let raw = spread_raw(f, p);
    println!("{raw} {:?} {:?}", f, c);
    let mut g = c;
    let q = &raw mut g;
    let behind = spread_around(unsafe { *q }, || unsafe { (*q).high += 2.0 });
    println!("{behind} {:?}", g);
}
