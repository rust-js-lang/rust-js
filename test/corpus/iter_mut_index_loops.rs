// A loop over an array's items through their `&mut`s, `iter_mut()`, its
// `enumerate()` and its `zip` with another array, is a loop over its
// indices, each written in its place, `state[i] = ..`, as a person writes
// it (#320, #354): of a zip, as long as the shorter, asked once. A handle a
// closure keeps, or one given on, keeps its form.

fn bump(state: &mut [u32; 4]) {
    for s in state.iter_mut() {
        *s += 1;
    }
}

fn xor(state: &mut [bool; 4], chunk: [bool; 3]) {
    for (s, c) in state.iter_mut().zip(chunk) {
        *s ^= c;
    }
}

fn add(into: &mut Vec<i32>, from: &[i32]) {
    for (a, b) in into.iter_mut().zip(from.iter()) {
        *a += *b;
    }
}

fn fill(row: &mut Vec<usize>) {
    for (i, cell) in row.iter_mut().enumerate() {
        *cell = i * i;
    }
}

fn until_big(values: &mut [u32]) -> u32 {
    let mut seen = 0;
    for v in values.iter_mut() {
        if *v > 10 {
            break;
        }
        *v *= 2;
        seen += 1;
    }
    seen
}

fn kept(values: &mut [u32]) -> u32 {
    let mut total = 0;
    for v in values.iter_mut() {
        let mut add = |n: u32| *v += n;
        add(1);
        add(2);
        total += *v;
    }
    total
}

fn given(values: &mut [u32]) {
    fn double(n: &mut u32) {
        *n *= 2;
    }
    for v in values.iter_mut() {
        *v += 1;
        double(v);
    }
}

fn main() {
    let mut state = [1u32, 2, 3, u32::MAX];
    bump(&mut state);
    println!("{state:?}");
    let mut bits = [true, false, true, false];
    xor(&mut bits, [true, true, true]);
    println!("{bits:?}");
    let mut into = vec![1, 2, 3];
    add(&mut into, &[10, 20]);
    println!("{into:?}");
    let mut row = vec![0usize; 4];
    fill(&mut row);
    println!("{row:?}");
    let mut values = vec![1u32, 5, 11, 2];
    println!("{} {values:?}", until_big(&mut values));
    let mut values = vec![1u32, 2];
    println!("{} {values:?}", kept(&mut values));
    given(&mut values);
    println!("{values:?}");
}
