// A trait's method generic over a const, giving its associated type of it,
// `Self::Assoc<N>`, rustc's own `const-generics-gat-in-trait-return-type-2`:
// the impl's dictionary asks what the method gives in the trait method's
// terms, whose generics have `N`, and rust-js crashed asking it in the
// dictionary's, which don't (#1304).
trait Foo {
    type Assoc<const N: usize>;
    fn foo<const N: usize>(&self) -> Self::Assoc<N>;
}

impl Foo for () {
    type Assoc<const N: usize> = [(); N];
    fn foo<const N: usize>(&self) -> Self::Assoc<N> {
        [(); N]
    }
}

trait Filled {
    type Row<const N: usize>;
    fn row<const N: usize>(&self, with: u8) -> Self::Row<N>;
}

struct Bytes;

impl Filled for Bytes {
    type Row<const N: usize> = [u8; N];
    fn row<const N: usize>(&self, with: u8) -> Self::Row<N> {
        [with; N]
    }
}

fn main() {
    assert_eq!(().foo::<10>(), [(); 10]);
    println!("{}", ().foo::<3>().len());
    println!("{:?}", Bytes.row::<4>(7));
}
