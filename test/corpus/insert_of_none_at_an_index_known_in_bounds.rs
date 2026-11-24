// `v.insert(at, None)` where the body's ranges show `at` at most the
// length is `v.splice(at, 0, undefined)`: its `undefined` is the item
// inserted, never left out as a trailing argument, or `splice` inserts
// nothing (ADR 0330, amended; `inserted-none-left-out`).
#[derive(Debug)]
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop D{}", self.0);
    }
}

fn main() {
    let mut v: Vec<Option<u32>> = vec![Some(1), Some(2)];
    let n = v.len();
    v.insert(n, None);
    println!("{v:?} {}", v.len());
    for i in 0..2 {
        if i < v.len() {
            v.insert(i, None);
        }
    }
    println!("{v:?} {}", v.len());
    let mut w: Vec<Option<D>> = vec![Some(D(1))];
    let len = w.len();
    w.insert(len, None);
    w.insert(len, Some(D(2)));
    println!("{w:?} {}", w.len());
}
