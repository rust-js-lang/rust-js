// A `Fuse` of a `FusedIterator` keeps its source when a `try_fold` runs it
// to its end, as std's specialization does, so what that source holds is
// dropped with the `Fuse`, after the statement's value is printed (#244,
// fuzz seed 51474).

struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn id<T>(x: T) -> T {
    x
}

fn louds(from: u32, n: u32) -> Vec<Loud> {
    (from..from + n).map(Loud).collect()
}

fn main() {
    println!("{}", louds(10, 3).into_iter().take(id(1)).fuse().any(|_| false));
    println!("{:?}", louds(20, 3).into_iter().take(id(1)).fuse().position(|_| false));
    let first = louds(30, 1)
        .into_iter()
        .rev()
        .take(id(0))
        .fuse()
        .find(|loud| loud.0 >= 30)
        .map(|loud| loud.0);
    println!("{first:?}");
    println!(
        "{:?}",
        louds(40, 1)
            .into_iter()
            .rev()
            .take(id(0))
            .fuse()
            .find(|loud| loud.0 >= 40)
            .map(|loud| loud.0)
    );
}
