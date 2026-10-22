// A closure that moves what it took out, an `FnOnce`, is cloned apart: its
// clone holds a clone of its own, which what it's given to may change.
fn run_twice<F: FnOnce() -> Vec<i32> + Clone>(f: F) -> (Vec<i32>, Vec<i32>) {
    let g = f.clone();
    let mut a = f();
    a.push(9);
    (a, g())
}

fn consume(mut v: Vec<i32>) -> Vec<i32> {
    v.push(1);
    v
}

fn main() {
    let v = vec![1];
    let h = move || {
        let mut v = v;
        v[0] *= 5;
        v
    };
    let h2 = h.clone();
    println!("{:?} {:?}", h(), h2());

    let w = vec![1];
    println!("{:?}", run_twice(move || w));

    let d = vec![0];
    let d = move || consume(d);
    let all = [d.clone(), d.clone(), d];
    println!("{:?}", all.map(|f| f()));
}
