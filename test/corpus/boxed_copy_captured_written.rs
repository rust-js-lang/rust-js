// A closure's write of a `Copy` struct it took through a box, `|w|
// (*frame).size = Size { w }`, which takes the place `(*frame).size`, the
// box being the value it holds: the struct's type is changed in place, as
// through a variable it took, so a copy read out before, `Box::new((*frame)
// .size)`, is its own (#997).
#[derive(Debug, Clone, Copy)]
struct Size {
    w: i32,
}

#[derive(Debug)]
struct Frame {
    size: Size,
    depth: i32,
}

fn main() {
    let mut frame = Box::new(Frame {
        size: Size { w: 0 },
        depth: 10,
    });
    let kept = Box::new((*frame).size);
    let mut set = |w| (*frame).size = Size { w };
    set(2);
    println!("{kept:?} {frame:?} {}", kept.w);
}
