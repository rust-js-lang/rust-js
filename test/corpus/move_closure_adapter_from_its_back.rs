// A `map()` or an `inspect()` of a `Vec`'s numbers, of a `move` closure that
// copies a field of a value with a destructor, `g.0`, or owns the value
// itself, stepped from its back: `rfind`, `next_back` then `next` of one
// held in a variable, and a `rev()` of one; each item seen as std's adapter
// gives it, and the value the closure owns dropped as the adapter is (#780).
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        print!("drop {} ", self.0);
    }
}

fn main() {
    let g = D(9);
    let found = vec![1u32, 2, 3, 4]
        .into_iter()
        .map(move |d| {
            print!("see {d} ");
            d + g.0
        })
        .rfind(|d| d % 2 == 0);
    println!("| copied field rfind {found:?}");
    let g = D(8);
    let found = vec![1u32, 2, 3, 4]
        .into_iter()
        .inspect(move |d| print!("see {d} {} ", g.0))
        .rfind(|d| d % 2 == 1);
    println!("| copied field inspect rfind {found:?}");
    let h = D(7);
    let found = vec![1u32, 2, 3, 4]
        .into_iter()
        .inspect(move |d| {
            let _ = &h;
            print!("see {d} ");
        })
        .rfind(|d| d % 2 == 1);
    println!("| owning inspect rfind {found:?}");
    let g = D(6);
    let mut it = vec![1u32, 2, 3].into_iter().map(move |d| d + g.0);
    let back = it.next_back();
    let front = it.next();
    println!("| held {back:?} {front:?}");
    drop(it);
    println!("| dropped");
    let k = D(5);
    let mut it = vec![1u32, 2, 3].into_iter().inspect(move |d| {
        let _ = &k;
        print!("see {d} ");
    });
    let back = it.next_back();
    println!("| owning held {back:?}");
    drop(it);
    println!("| owning dropped");
    let g = D(4);
    let r: Vec<u32> = vec![1u32, 2, 3].into_iter().map(move |d| d * g.0).rev().collect();
    println!("| rev {r:?}");
}
