// A parameter assigned itself, `foo = foo;`, is still what its caller gave:
// asking what an assignment of it stores followed it forever, and the
// compiler overflowed its stack (rustc's nll/issue-50461-used-mut-from-moves).
struct Foo {
    value: i32,
}

fn use_foo_mut(mut foo: Foo) {
    foo = foo;
    println!("{}", foo.value);
}

fn step(mut x: Option<i32>, again: bool) -> Option<i32> {
    x = x;
    if again {
        x = x.map(|x| x + 1);
    }
    x
}

fn main() {
    use_foo_mut(Foo { value: 413 });
    println!("{:?} {:?} {:?}", step(Some(1), true), step(None, true), step(Some(2), false));
}
