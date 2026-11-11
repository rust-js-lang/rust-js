// ppv-lite86's storages (#1402): `x.into()` of a type parameter's value
// where the bound is `Storage: From<W>`, a `From` of another type, made of
// each half.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Storage([u32; 4]);

#[derive(Clone, Copy, Debug)]
pub struct Lanes([u32; 4]);

impl From<Lanes> for Storage {
    fn from(x: Lanes) -> Self {
        Storage([x.0[3], x.0[2], x.0[1], x.0[0]])
    }
}

impl From<u32> for Storage {
    fn from(x: u32) -> Self {
        Storage([x; 4])
    }
}

#[derive(Debug)]
pub struct Wide([Storage; 2]);

pub struct X2<W>([W; 2]);

impl<W> From<X2<W>> for Wide
where
    W: Copy,
    Storage: From<W>,
{
    fn from(x: X2<W>) -> Self {
        Wide([x.0[0].into(), x.0[1].into()])
    }
}

fn main() {
    let wide: Wide = X2([Lanes([1, 2, 3, 4]), Lanes([5, 6, 7, 8])]).into();
    println!("{wide:?}");
    println!("{:?}", Wide::from(X2([9u32, 10])));
}
