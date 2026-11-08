// `&a != &b` of an `A: PartialEq<B>` that writes its own `ne` is std's
// `impl PartialEq<&B> for &A`, whose `ne` calls `A`'s: never `!(a ==
// b)`, which once ran `eq` (#1023).
struct A(u32);
struct B(u32);

impl PartialEq<B> for A {
    fn eq(&self, other: &B) -> bool {
        println!("eq {} {}", self.0, other.0);
        self.0 == other.0
    }

    #[allow(clippy::partialeq_ne_impl)]
    fn ne(&self, other: &B) -> bool {
        println!("ne {} {}", self.0, other.0);
        self.0 != other.0
    }
}

fn main() {
    let (a, b) = (A(1), B(2));
    println!("{}", &a != &b);
    let (ra, rb) = (&&a, &&b);
    println!("{}", ra != rb);
    println!("{}", A(3) != B(3));
}
