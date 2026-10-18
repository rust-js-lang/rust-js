// An `Option` whose payload looks like `None` is boxed, `Some(())` an object
// of its own (ADR 0051): `==` compares what's in it, not which object.

fn unit(o: Option<()>) -> bool {
    o == Some(())
}

fn nested(o: Option<Option<u32>>) -> bool {
    o == Some(None)
}

fn main() {
    println!("{} {}", unit(Some(())), unit(None));
    println!("{} {} {}", nested(Some(None)), nested(Some(Some(1))), nested(None));
    let a: Option<Option<u32>> = Some(Some(2));
    println!("{} {}", a == Some(Some(2)), a != Some(Some(3)));
}
