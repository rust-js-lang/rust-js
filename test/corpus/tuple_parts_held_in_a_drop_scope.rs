// A tuple's parts held apart in a body a drop's scope wraps: the scope is
// made, then the stages that read temporaries in place run again, and
// `tuple reads` holds each part of the tuple of calls by a name of its
// own, of the tuple's, which lowering made and preparation spells. A part
// of one not spelled yet was spelled of its word, `value#58$0`, which
// nothing spelled: tidy stopped after that run gave no JS (#2371).
fn id<T>(x: T) -> T {
    x
}

struct Noisy(u32);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn main() {
    println!("{:?}", (id(0i32), id(254u8)));
    let _noisy = Noisy(id(21));
    println!("{:#x}", id(10i32));
}
