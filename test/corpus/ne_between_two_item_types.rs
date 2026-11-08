// `!=` and slice `==` between two item types, `A: PartialEq<B>`, through
// generic `T: PartialEq<U>` too, run `A`'s own `ne`, as std's do: a slice's
// `==` asks each two `!=` after their lengths (#970, ADR 0057 amended).
struct A(i32);
struct B(i32);

impl PartialEq<B> for A {
    fn eq(&self, other: &B) -> bool {
        println!("eq {} {}", self.0, other.0);
        self.0 == other.0
    }

    fn ne(&self, other: &B) -> bool {
        println!("ne {} {}", self.0, other.0);
        self.0 != other.0
    }
}

fn differ<T: PartialEq<U>, U>(a: &T, b: &U) -> bool {
    a != b
}

fn same<T: PartialEq<U>, U>(a: &[T], b: &[U]) -> bool {
    a == b
}

fn main() {
    println!("{}", A(1) != B(2));
    println!("{}", differ(&A(3), &B(3)));
    let a = [A(1), A(2)];
    let b = [B(1), B(5)];
    println!("{}", a == b);
    println!("{}", a[..] == b[..]);
    println!("{}", same(&a, &b));
    println!("{}", vec![A(4)] == vec![B(4)]);
    println!("{}", same(&[A(7)], &[B(7), B(8)]));
}
