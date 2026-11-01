use std::rc::Rc;

struct Context {
    depth: u32,
}

fn read_context(context: &Context) -> u32 {
    context.depth
}

fn kind_of(n: u32) -> &'static str {
    if n % 2 == 0 { "object" } else { "number" }
}

fn use_value(usable: Option<u32>) -> u32 {
    // Only an object is usable.
    if let Some(usable) = usable
        && kind_of(usable) == "object"
    {
        if usable > 100 {
            // A large one is its own.
            return usable;
        } else if usable == 4 {
            panic!("four is recoverable");
        } else if usable > 10 {
            let context = Rc::new(Context { depth: usable / 2 });
            return read_context(&context);
        }
    }

    panic!("An unsupported type was passed to use(): {usable:?}");
}

fn main() {
    println!("{}", use_value(Some(200)));
    println!("{}", use_value(Some(20)));
    for n in [None, Some(3), Some(4), Some(6)] {
        let caught = std::panic::catch_unwind(|| use_value(n));
        println!("{n:?} {}", caught.is_err());
    }
}
