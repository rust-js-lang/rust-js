pub trait Make {
    type Out;
    fn make(&self) -> Self::Out;
}

pub fn made_twice<M: Make>(m: &M) -> (M::Out, M::Out) {
    (m.make(), m.make())
}
