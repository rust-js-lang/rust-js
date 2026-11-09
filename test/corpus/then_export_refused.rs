//@ compile-fail: `then` is exported as `then` in JS, which makes its module a thenable
// A function a module exports as `then` makes the module a thenable: an
// `import()` of it, a host's or a bundler's lazy chunk, calls it as
// `then(resolve, reject)` and gives what that resolves, not the module
// (#1068). Refused, saying to give it another JS name; a method named
// `then`, its type's, and a function its module keeps to itself are no
// export of the module, and compile.

pub struct Chunk {
    pub value: u32,
}

impl Chunk {
    pub fn then(&self) -> u32 {
        self.value
    }
}

fn kept(n: u32) -> u32 {
    n + 1
}

pub fn then(chunk: &Chunk) -> u32 {
    kept(chunk.then())
}

fn main() {
    println!("{}", then(&Chunk { value: 1 }));
}
