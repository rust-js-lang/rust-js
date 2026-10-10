// A slice's `split_off`, `split_off_first` and `split_off_last` take what
// they give out of the slice they're given a `&mut` to, which is then the
// rest: of a shared slice, copies; of a `&mut` one, views and items.

fn take_word<'a>(input: &mut &'a [u8]) -> Option<&'a [u8]> {
    if input.is_empty() {
        return None;
    }
    let end = input.iter().position(|b| *b == b' ').unwrap_or(input.len());
    let word = input.split_off(..end)?;
    input.split_off_first();
    Some(word)
}

fn main() {
    let v = [1, 2, 3, 4, 5, 6];
    let mut s: &[i32] = &v;
    println!("{:?} {s:?}", s.split_off(..2));
    println!("{:?} {s:?}", s.split_off(3..));
    println!("{:?} {s:?}", s.split_off(..=0));
    println!("{:?} {s:?}", s.split_off(..9));
    println!("{:?} {:?} {s:?}", s.split_off_first(), s.split_off_last());
    println!("{:?} {s:?}", s.split_off_first());

    let mut text: &[u8] = b"to be or";
    while let Some(word) = take_word(&mut text) {
        print!("{} ", word.len());
    }
    println!();

    let mut w = vec![1, 2, 3, 4, 5, 6];
    let mut rest: &mut [i32] = &mut w;
    if let Some(head) = rest.split_off_mut(..2) {
        head[0] = 10;
    }
    if let Some(first) = rest.split_off_first_mut() {
        *first *= 100;
    }
    if let Some(last) = rest.split_off_last_mut() {
        *last = -1;
    }
    rest.reverse();
    println!("{w:?}");
    let mut names = vec![String::from("a"), String::from("b")];
    let mut all: &mut [String] = &mut names;
    if let Some(name) = all.split_off_last_mut() {
        name.push('!');
    }
    all[0].push('?');
    println!("{names:?}");
    let mut none: &mut [i32] = &mut [];
    println!("{:?} {:?}", none.split_off_first_mut(), none.split_off_mut(1..));
}
