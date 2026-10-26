// A generic function given as a value, `map(keep)`, is the function itself
// where its body drops nothing it's given, and an arrow that gives it its
// drop where its body drops, `map((arg0) => first(arg0, loudDrop_drop))`
// (ADR 0300).

struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn keep<T>(value: T) -> T {
    value
}

fn first<T>(items: Vec<T>) -> Option<T> {
    items.into_iter().next()
}

fn main() {
    let kept: Vec<Loud> = vec![Loud(1), Loud(2)].into_iter().map(keep).collect();
    println!("{}", kept.len());
    let firsts: Vec<Option<Loud>> = vec![vec![Loud(3), Loud(4)]].into_iter().map(first).collect();
    println!("{}", firsts.len());
}
