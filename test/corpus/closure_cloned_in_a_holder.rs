// A closure that changes what it captured, cloned or copied inside what
// holds it, an `Option`, a `Result`, a tuple, an array, a `Vec`, a `Box` or
// a struct of the crate's that derives `Clone`: each clone counts on its
// own, as each copy of a `Copy` one does, not the closure it was made from
// (#1211).
#[derive(Clone, Copy)]
struct Wrap<F>(F, u8);

#[derive(Clone)]
struct Named<F> {
    tick: F,
    name: &'static str,
}

fn counter() -> impl FnMut() -> i32 + Copy {
    let mut count = 0;
    move || {
        count += 1;
        count
    }
}

fn texts() -> impl FnMut() -> String + Clone {
    let mut said = String::from("a");
    move || {
        said.push('b');
        said.clone()
    }
}

fn main() {
    let mut count = 0;
    let plain = move || {
        count += 1;
        count
    };
    let mut held = Some(plain);
    let held2 = held.clone();
    println!("{} {}", held.as_mut().unwrap()(), held2.map(|mut f| f()).unwrap());

    let maybe = Some(counter());
    let copied = maybe;
    let cloned = maybe.clone();
    println!(
        "{} {} {}",
        maybe.map(|mut f| f()).unwrap(),
        copied.map(|mut f| f()).unwrap(),
        cloned.map(|mut f| f()).unwrap()
    );

    let pair = (counter(), 1);
    let (mut other, _) = pair;
    let mut again = pair.clone();
    println!("{} {} {}", other(), (again.0)(), { pair }.0());

    let ok: Result<_, ()> = Ok(texts());
    let ok2 = ok.clone();
    println!("{} {}", ok.map(|mut f| f()).unwrap(), ok2.map(|mut f| f()).unwrap());

    let mut arr = [texts(), texts()];
    let mut arr2 = arr.clone();
    println!("{} {} {}", arr[0](), arr[0](), arr2[0]());

    let mut many = vec![texts()];
    let mut many2 = many.clone();
    println!("{} {} {}", many[0](), many[0](), many2[0]());

    let mut boxed = Box::new(texts());
    let mut boxed2 = boxed.clone();
    println!("{} {}", boxed(), boxed2());

    let w = Wrap(counter(), 0);
    let mut w2 = w;
    let mut w3 = w.clone();
    println!("{} {} {}", (w2.0)(), (w2.0)(), (w3.0)());

    let mut named = Named { tick: texts(), name: "n" };
    let mut named2 = named.clone();
    println!("{} {} {}", (named.tick)(), (named2.tick)(), named2.name);
}
