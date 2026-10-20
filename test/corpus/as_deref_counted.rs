// `as_deref()` of an `Rc` whose count is read, `{ value, strong, weak }`
// (ADR 0320): its `value`, one layer of it.
use std::rc::Rc;

fn main() {
    // Counted, `{ value, strong, weak }`, as its count is read.
    let counted: Option<Rc<Vec<u8>>> = Some(Rc::new(vec![1, 2]));
    let again = counted.clone();
    println!("{:?} {}", counted.as_deref(), Rc::strong_count(again.as_ref().unwrap()));
    // Two counted layers: `as_deref` takes off one.
    let nested: Option<Rc<Rc<String>>> = Some(Rc::new(Rc::new("in".to_string())));
    let inner: Option<&Rc<String>> = nested.as_deref();
    println!("{:?} {}", inner.map(|rc| Rc::strong_count(rc)), Rc::strong_count(nested.as_ref().unwrap()));
    let counted_ok: Result<Rc<Vec<u8>>, u8> = Ok(Rc::new(vec![3]));
    println!("{:?}", counted_ok.as_deref());
}
