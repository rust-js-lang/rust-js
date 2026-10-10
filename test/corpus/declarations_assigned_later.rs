// A variable assigned after it's made: what it's made with runs, though
// it's never read; what reads it between sees the first value; one a
// closure assigns, or a `&mut` to it does, changes.
fn made(n: i32) -> i32 {
    println!("made {n}");
    n
}

fn bump(n: &mut i32) {
    *n += 1;
}

#[allow(unused_assignments)]
fn main() {
    let mut a = made(1);
    a = 2;
    println!("{a}");
    let mut b = 3;
    println!("{b}");
    b = 4;
    println!("{b}");
    let mut c = 0;
    let mut add = |k: i32| c += k;
    add(5);
    add(6);
    println!("{c}");
    let mut d = 7;
    bump(&mut d);
    let kept = &mut d;
    *kept += 1;
    println!("{d}");
}
