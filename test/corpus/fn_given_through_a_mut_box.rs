// A function given for a generic `F: FnOnce()`, `FnMut()` or `Fn()` through
// references and boxes is the function, as std's `impl FnOnce for &mut F`,
// `&F` and `Box<F>` call what they refer to (#1909): a `&mut` to a boxed
// `dyn Fn` that a `let mut` holds, which is its box elsewhere, is given as
// the function, and one held in a `&mut` that's itself boxed is read
// through, `lent.value`.

fn invoke(run: impl FnOnce()) {
    run();
}

fn twice<F: FnMut()>(mut f: F) {
    f();
    f();
}

fn called<F: Fn() -> u32>(f: F) -> u32 {
    f()
}

fn main() {
    let mut callback: Box<dyn Fn()> = Box::new(|| println!("called"));
    invoke(&mut callback);
    let boxed: Box<dyn Fn()> = Box::new(|| println!("boxed by ref"));
    invoke(&boxed);
    let plain = || println!("through &mut &dyn");
    let by_ref: &dyn Fn() = &plain;
    let mut outer = by_ref;
    invoke(&mut outer);
    let mut count = 0;
    let mut add = || count += 1;
    let lent: &mut dyn FnMut() = &mut add;
    let held: Box<&mut dyn FnMut()> = Box::new(lent);
    twice(held);
    println!("count {count}");
    let mut n = 0;
    let mut step = || {
        n += 2;
        println!("step {n}");
    };
    twice(&mut step);
    twice(&mut step);
    let seven = || 7;
    println!("{}", called(&&seven));

    let mut inner: Box<dyn Fn()> = Box::new(|| println!("deep"));
    let mut lent = &mut inner;
    invoke(&mut lent);
    let held = &mut inner;
    invoke(held);
    inner = Box::new(|| println!("after"));
    invoke(&mut inner);
}
