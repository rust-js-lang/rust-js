// A union's constant, and its fields read: of one type, `u64`, each reads
// what it holds, whose bytes are the same (ADR 0406). From rustc's
// union-const-codegen.rs.
union U {
    a: u64,
    b: u64,
}

const C: U = U { b: 10 };

fn main() {
    let (a, b) = unsafe { (C.a, C.b) };
    println!("{a} {b}");
}
