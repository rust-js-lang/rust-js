// A boxed function boxed again, `Box<dyn Fn>` given to generic code that
// boxes what it's given, and a `Box<Box<dyn FnOnce>>` made a `dyn FnOnce`:
// one allocation inside another, each dropped once, the inner's drop as
// the outer's runs, never twice and never a drop that calls itself
// (#1221). A closure given to generic code that boxes it as a `dyn
// FnOnce`, of each kind: called, it drops what it holds as its body ends,
// once; dropped uncalled, as the box is.
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn boxed<F: Fn() -> usize + 'static>(f: F) -> Box<dyn Fn() -> usize> {
    Box::new(f)
}

fn holding<K: 'static>(value: K) -> Box<dyn Fn() -> usize> {
    let shared = vec![value];
    let inner: Box<dyn Fn() -> usize> = Box::new(move || shared.len());
    boxed(inner)
}

fn once<F: FnOnce() + 'static>(f: F) -> Box<dyn FnOnce()> {
    Box::new(f)
}

fn run(task: Box<dyn FnOnce()>) {
    task();
    println!("called");
}

fn main() {
    let count = holding(D(1));
    println!("{}", count());
    let twice = boxed(boxed(holding(D(2))));
    println!("{}", twice());
    drop(count);
    drop(twice);

    let a = D(3);
    let inner: Box<dyn FnOnce()> = Box::new(move || {
        let _a = a;
        println!("moved out");
    });
    run(Box::new(inner));
    let b = D(4);
    let inner: Box<dyn FnOnce()> = Box::new(move || {
        let _b = &b;
        println!("held");
    });
    run(Box::new(inner));
    let c = D(5);
    let inner: Box<dyn FnOnce()> = Box::new(move || {
        let _c = &c;
        println!("never");
    });
    let outer: Box<dyn FnOnce()> = Box::new(inner);
    drop(outer);

    let d = D(6);
    let moving = once(move || {
        let _d = d;
        println!("moving called");
    });
    moving();
    let e = D(7);
    let kept = once(move || {
        let _e = &e;
        println!("kept called");
    });
    kept();
    let f = D(8);
    let inner: Box<dyn FnOnce()> = Box::new(move || {
        let _f = f;
        println!("inner called");
    });
    let outer = once(inner);
    outer();
    let g = D(9);
    let uncalled = once(move || {
        let _g = &g;
        println!("uncalled");
    });
    drop(uncalled);
    println!("end");
}
