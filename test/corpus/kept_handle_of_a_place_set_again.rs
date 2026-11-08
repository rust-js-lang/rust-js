// A `&mut` to a number kept past where it's made is a handle on its place
// (ADR 0099), which reads its place whenever it's read: the place as it is
// where the borrow is made, an index a variable set again holds, `&mut
// c[k]` before `k = 2`, and an object a reference a field holds is,
// `Box::new(&mut h.node.count)` before `h.node = &mut other`, as borrowck lets each
// be set again while the borrow lasts (#988).
fn keep(r: &mut u32) -> Option<&mut u32> {
    Some(r)
}

struct Node {
    count: u32,
}

struct Holder<'a> {
    node: &'a mut Node,
}

fn main() {
    let mut c = [1u32, 2, 3];
    let mut k = 0;
    let t = keep(&mut c[k]).unwrap();
    k = 2;
    *t += 10;
    println!("{c:?} {k}");

    let mut a = [1u32, 2, 3];
    let mut i = 0;
    let r = &mut a[i];
    i = 2;
    *r += 20;
    println!("{a:?} {i}");

    let mut na = Node { count: 1 };
    let mut nb = Node { count: 5 };
    let mut h = Holder { node: &mut na };
    let kept = Box::new(&mut h.node.count);
    h.node = &mut nb;
    **kept += 100;
    println!("{}", h.node.count);
    drop(h);
    println!("{}", na.count);
}
