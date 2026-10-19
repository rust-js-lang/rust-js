// A closure that holds a value with a destructor, given by value where it's
// called by `&` or `&mut`: a generic function's `F: Fn` drops it as its
// caller's drop says, when it's done; std's `all`, `fold` and `sort_by` drop
// it as they return (ADR 0470).
struct Loud(u32);

impl Loud {
    fn get(&self) -> u32 {
        self.0
    }
}

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn twice<F: Fn(u32) -> u32>(f: F) -> u32 {
    f(1) + f(2)
}

fn every<F: FnMut(&u32) -> bool>(f: F) -> bool {
    [1, 2].iter().all(f)
}

fn main() {
    let a = Loud(1);
    println!("twice {}", twice(move |x| x + a.get()));
    let a = Loud(2);
    let all = [1, 2, 3].iter().all(move |&x| x < a.get());
    println!("all {all}");
    let a = Loud(3);
    println!("every {}", every(move |x: &u32| *x < a.get()));
    let a = Loud(4);
    let total = [1, 2, 3].into_iter().fold(0, move |sum, x| sum + x + a.get());
    println!("fold {total}");
    let a = Loud(5);
    let mut v = vec![3, 1, 2];
    v.sort_by(move |x, y| (x % a.get()).cmp(&(y % a.get())));
    println!("sorted {v:?}");
    let a = Loud(6);
    let read = move || a.get();
    println!("read {}", read());
    println!("twice {}", twice(move |x| x + read()));
    println!("end");
}
