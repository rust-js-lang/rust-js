// An iterator a box holds, replaced through it, `*it = ..`, which writes
// `it` again, as `it = ..` does: what's stepped after is the new one,
// and each step's form is the one it was made with (#980's rule, the
// write counted through the box).
fn double(x: i32) -> i32 {
    x * 2
}

fn main() {
    let mut it = Box::new([1, 2, 3].into_iter().map(double as fn(i32) -> i32));
    let first = (*it).next();
    *it = [7, 8, 9].into_iter().map(double as fn(i32) -> i32);
    let back = (*it).next_back();
    let rest: Vec<i32> = (*it).by_ref().collect();
    println!("{first:?} {back:?} {rest:?}");
}
