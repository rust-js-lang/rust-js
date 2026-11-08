// A value of a const parameter's type, `Buffer<SIZE>`, in a `RefCell` an
// `Rc` holds, borrowed: what its impls may run was asked of std's impl of
// `[u8; SIZE]` in an environment that doesn't say what `SIZE` is, a rustc
// ICE, "cannot find SIZE/#0 in param-env"; a type of a const parameter is
// no std impl's to select there, what it may run asked of its parts
// (#1017).
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone, Copy, Debug, PartialEq)]
struct Buffer<const SIZE: usize> {
    bytes: [u8; SIZE],
    len: usize,
}

impl<const SIZE: usize> Buffer<SIZE> {
    fn new() -> Self {
        Buffer {
            bytes: [0; SIZE],
            len: 0,
        }
    }
}

fn made<const SIZE: usize>() -> usize {
    let buffer = Rc::new(RefCell::new(Buffer::<SIZE>::new()));
    buffer.borrow_mut().len += 1;
    let copy = *buffer.borrow();
    buffer.borrow().bytes.len() + buffer.borrow().len + usize::from(copy == *buffer.borrow())
}

fn main() {
    println!("{} {}", made::<4>(), made::<0>());
}
