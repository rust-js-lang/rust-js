// A map of a `Vec`'s iterator held in a variable and stepped from its back,
// `rfind`, made again there: the held one, which nothing reads, is no
// statement, its closure run only as `rfind` steps it, nothing early; and
// a `collect()` of a map nothing reads after, an array's eager `map`, runs
// its closure on each item still, as Rust's does (#805).
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
    println!("| held map rfind {found:?}");
    let v = vec![1u32, 2, 3];
    let h = D(8);
    let found = v
        .iter()
        .map(|d| {
            print!("see {d} ");
            d + h.0
        })
        .rfind(|d| d % 2 == 1);
    println!("| borrowed map rfind {found:?}");
    let _unread: Vec<u32> = v
        .iter()
        .map(|x| {
            print!("collect {x} ");
            x * 2
        })
        .collect();
    println!("| collected, unread");
}
