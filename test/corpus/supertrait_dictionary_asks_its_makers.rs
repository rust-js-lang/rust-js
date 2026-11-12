//@ library-differs
// The size of an associated type a supertrait's impl gives, made where
// another impl's dictionary makes that supertrait's, ppv-lite86's `BitOps0:
// AndNot<Output = Self>` of its `x2<W, G>`, or an associated type's bound's
// (#1430).
use std::marker::PhantomData;
use std::mem::size_of;

trait AndNot {
    type Output;
    fn andnot(self, rhs: Self) -> Self::Output;
}

trait BitOps0: AndNot<Output = Self> + Copy {}

#[derive(Clone, Copy)]
struct X2<W, G>([W; 2], PhantomData<G>);

impl<W: Copy, G: Copy> AndNot for X2<W, G> {
    type Output = Self;
    fn andnot(self, _rhs: Self) -> Self {
        self
    }
}

impl<W: Copy, G: Copy> BitOps0 for X2<W, G> {}

fn output_size<T: AndNot>() -> usize {
    size_of::<T::Output>()
}

fn through_sub<T: BitOps0>(t: T) -> usize {
    let _ = t.andnot(t);
    output_size::<T>()
}

fn wrapped<W: Copy>(w: W) -> usize {
    through_sub(X2::<W, u8>([w, w], PhantomData))
}

fn wrapped_alone<W: Copy>() -> usize {
    output_size::<X2<W, u8>>()
}

trait Halves {
    type Half: AndNot;
    fn half(&self) -> Self::Half;
}

impl<W: Copy> Halves for X2<W, u8> {
    type Half = X2<W, ()>;
    fn half(&self) -> X2<W, ()> {
        X2(self.0, PhantomData)
    }
}

fn half_size<T: Halves>(t: &T) -> usize {
    let _ = t.half();
    output_size::<T::Half>()
}

fn halved<W: Copy>(w: W) -> usize {
    half_size(&X2::<W, u8>([w, w], PhantomData))
}

fn main() {
    println!("{}", through_sub(X2::<u32, u8>([1, 2], PhantomData)));
    println!("{}", output_size::<X2<u64, ()>>());
    println!("{}", wrapped(7u16));
    println!("{}", wrapped([1u64; 3]));
    println!("{}", wrapped_alone::<u16>());
    println!("{}", halved(3u8));
    println!("{}", halved((1u32, 2u64)));
}
