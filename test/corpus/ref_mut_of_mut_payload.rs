// `ref mut x` of an `Option<&mut T>`'s payload, `x: &mut &mut T`: that
// payload where it's used, as a `let outer = &mut list` is `list`.
struct Foo {
    x: usize,
}

struct Unit;

impl Unit {
    fn name(&self) -> &'static str {
        "unit"
    }
}

fn opt(o: &mut Option<&mut Foo>) -> usize {
    match o {
        &mut Some(ref mut f) => {
            f.x += 10;
            f.x
        }
        &mut None => 0,
    }
}

fn unit<'a>(o: &'a mut Option<&'a mut Unit>) -> &'a mut Unit {
    match o {
        &mut Some(ref mut u) => *u,
        &mut None => panic!(),
    }
}

/// Used twice: the payload where each is, `$someValue(o.value)`.
fn unit_twice<'a>(o: &'a mut Option<&'a mut Unit>) -> String {
    match o {
        &mut Some(ref mut u) => format!("{} {}", u.name(), u.name()),
        &mut None => String::new(),
    }
}

fn main() {
    let mut foo = Foo { x: 1 };
    let mut o = Some(&mut foo);
    let a = opt(&mut o);
    if let Some(ref mut f) = o {
        f.x += 100;
    }
    let mut u = Unit;
    let mut p = Some(&mut u);
    println!("{} {} {}", a, foo.x, unit(&mut p).name());
    let mut v = Unit;
    println!("{}", unit_twice(&mut Some(&mut v)));
}
