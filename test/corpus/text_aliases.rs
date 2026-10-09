// `String`'s and `str`'s methods JS already has, by their other names: the
// deprecated `trim_left` and `lines_any`, bytes, boxes, and capacity, which
// a JS string doesn't have (ADR 0322).

#[allow(deprecated)]
fn main() {
    let text = "  héllo  ";
    println!("[{}] [{}]", text.trim_left(), text.trim_right());
    println!("[{}] [{}]", "xxaxx".trim_left_matches('x'), "xxaxx".trim_right_matches("x"));
    println!("{:?}", "a\nb\r\nc".lines_any().collect::<Vec<_>>());

    let mut owned = String::with_capacity(4);
    owned.reserve(10);
    owned.push_str("héllo");
    owned.shrink_to_fit();
    owned.reserve_exact(1);
    owned.shrink_to(2);
    println!("{:?} {:?}", owned.as_bytes(), owned.clone().into_bytes());
    let boxed: Box<str> = owned.into_boxed_str();
    let back: String = boxed.clone().into_string();
    println!("{boxed} {back}");
    println!("{:?}", boxed.into_boxed_bytes());
    let bytes = vec![104, 105];
    let made = unsafe { String::from_utf8_unchecked(bytes) };
    println!("{made}");

    let parts: Box<[i32]> = vec![1, 2].into_boxed_slice();
    let mut list = parts.into_vec();
    list.push(3);
    println!("{list:?} {}", ["a", "b"].connect("-"));
    let nested = [vec![1, 2], vec![3]];
    println!("{:?} {:?} {:?}", nested.join(&0), nested.join(&[7, 8][..]), nested.concat());
    let words = [vec!["a".to_string()], vec!["b".to_string()]];
    let mut joined = words.join(&"-".to_string());
    joined[0].push('!');
    println!("{joined:?} {words:?}");
}
