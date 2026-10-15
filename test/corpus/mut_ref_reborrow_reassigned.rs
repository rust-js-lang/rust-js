// A reborrow of a reference, `&mut *x`, made before `x` is reassigned and
// used after: of what `x` referred to then, held, not of what it refers to
// when it's used (ADR 0099). An argument's, and an assignment's through a
// `&mut &mut`, rustc's `mir_assign_eval_order`.
fn add(a: &mut i32, b: i32) {
    *a += b;
}

fn argument() {
    let mut p = 1;
    let mut q = 2;
    let mut x = &mut p;
    add(&mut *x, {
        x = &mut q;
        10
    });
    *x += 100;
    println!("{p} {q}");
}

fn assignment() {
    let mut x = &mut 1;
    let y = &mut &mut 2;
    let z = &mut 3;
    *{
        x = z;
        &mut *y
    } = x;
    println!("{} {}", *x, **y);
}

fn main() {
    argument();
    assignment();
}
