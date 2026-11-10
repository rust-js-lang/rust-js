use dep::{by_call, by_dyn, by_pointer};

pub fn main() {
    let d: &dyn Fn(()) -> Option<()> = &by_dyn;
    println!("{:?} {}", d(()), d(()).is_none());
    let n: &dyn Fn(Option<u8>) -> Option<Option<u8>> = &by_dyn;
    println!("{:?}", n(None));
    let p: fn(()) -> Option<()> = by_pointer;
    println!("{:?}", p(()));
    println!("{:?} {:?}", by_call(()), by_call(None::<u8>));
}
