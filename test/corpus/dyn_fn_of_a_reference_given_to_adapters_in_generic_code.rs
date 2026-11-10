// A boxed `dyn FnMut(&u32)` and a `fn(&u32)` pointer given to std's
// adapters in generic code: what std's code runs of them is asked of trait
// selection, of a type inside their binders, `(&u32,)`, whose region the
// binder binds, erased as codegen's are (#1224, #1111).
fn kept<I: IntoIterator<Item = u32>>(items: I) -> Vec<u32> {
    let keep: Box<dyn FnMut(&u32) -> bool> = Box::new(|x| *x > 1);
    items.into_iter().filter(keep).collect()
}

fn first_even<I: Iterator<Item = u32>>(mut items: I) -> Option<usize> {
    let even: fn(u32) -> bool = |x| x % 2 == 0;
    items.position(even)
}

fn seen<I: Iterator<Item = u32>>(items: I) -> Vec<u32> {
    let mut log = Vec::new();
    let shown: Box<dyn FnMut(&u32)> = Box::new(|x| log.push(*x));
    items.inspect(shown).for_each(drop);
    log
}

fn any_large<T: Copy + Into<u64>>(items: &[T]) -> bool {
    let large: &dyn Fn(&T) -> bool = &|x| (*x).into() > 2;
    items.iter().any(large)
}

fn main() {
    println!("{:?} {:?}", kept(vec![1, 2, 3]), kept(0..5u32));
    println!("{:?} {:?}", first_even([1u32, 3, 4].into_iter()), first_even(1..2u32));
    println!("{:?}", seen(0..3u32));
    println!("{} {}", any_large(&[1u8, 3]), any_large(&[1u16]));
}
