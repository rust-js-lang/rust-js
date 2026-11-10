use std::mem::MaybeUninit;

// What the bytes are differs each run, natively and in JS: the report says
// only what holds of any run, that each fill wrote its bytes and two
// fills differ, which a fill that wrote nothing, or wrote one place for
// another, wouldn't.
pub fn report() -> String {
    let mut a = [0u8; 32];
    getrandom::fill(&mut a).expect("a fill");
    let mut b = [0u8; 32];
    getrandom::fill(&mut b).expect("a fill");
    let mut uninit = [MaybeUninit::<u8>::uninit(); 32];
    let filled = getrandom::fill_uninit(&mut uninit).expect("a fill");
    let filled_len = filled.len();
    let filled_set = filled.iter().any(|&x| x != 0);
    let mut empty = [0u8; 0];
    getrandom::fill(&mut empty).expect("an empty fill");
    format!(
        "set {} {} differ {} uninit {filled_len} {filled_set}",
        a.iter().any(|&x| x != 0),
        b.iter().any(|&x| x != 0),
        a != b,
    )
}
