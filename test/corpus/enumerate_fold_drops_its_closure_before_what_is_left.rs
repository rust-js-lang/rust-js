// An `enumerate()` folded by value, `for_each` or `fold`, of a source that
// ends before its items do, a `take()` of a `Vec`'s, drops the consumer's
// closure before what's left of the source, as std's folds, each by value,
// drop their `f` first: the enumeration gives its source's fold what drops
// with it (#734, #1979).
struct D(&'static str);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn main() {
    let g = D("f");
    vec![D("a"), D("b"), D("c")].into_iter().take(1).enumerate().for_each(move |(i, d)| {
        let _g = &g;
        println!("item {i} {}", d.0);
    });
    println!("--");
    let g = D("f2");
    let n = vec![D("x"), D("y"), D("z")].into_iter().skip(1).take(1).enumerate().fold(0, move |a, (i, d)| {
        let _g = &g;
        a + i + d.0.len()
    });
    println!("folded {n}");
    println!("--");
    let g = D("f3");
    vec![D("p"), D("q")].into_iter().rev().take(1).enumerate().for_each(move |(i, d)| {
        let _g = &g;
        println!("rev item {i} {}", d.0);
    });
}
