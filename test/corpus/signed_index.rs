// An `isize` tested `>= 0` before `v[s as usize]` is a JS index as it is,
// and `len() as isize - 1` is `length - 1` (ADR 0697): React's stack-frame
// matching, `describeNativeComponentFrame`'s loop, its indices below zero
// where it runs out. One not tested is `usize`'s, and past the end it panics,
// as `-1 as usize` does.
use std::hint::black_box;

fn frame(sample_stack: &str, control_stack: &str) -> String {
    let sample_lines: Vec<&str> = sample_stack.split('\n').collect();
    let control_lines: Vec<&str> = control_stack.split('\n').collect();
    let mut s = 0;
    let mut c = 0;
    while s < sample_lines.len() && !sample_lines[s].contains("Root") {
        s += 1;
    }
    while c < control_lines.len() && !control_lines[c].contains("Root") {
        c += 1;
    }
    let mut s = s as isize;
    let mut c = c as isize;
    if s as usize == sample_lines.len() || c as usize == control_lines.len() {
        s = sample_lines.len() as isize - 1;
        c = control_lines.len() as isize - 1;
        while s >= 1 && c >= 0 && sample_lines[s as usize] != control_lines[c as usize] {
            c -= 1;
        }
    }
    while s >= 1 && c >= 0 {
        if sample_lines[s as usize] != control_lines[c as usize] {
            if s != 1 || c != 1 {
                loop {
                    s -= 1;
                    c -= 1;
                    if c < 0 || sample_lines[s as usize] != control_lines[c as usize] {
                        return sample_lines[s as usize].replacen(" at new ", " at ", 1);
                    }
                    if !(s >= 1 && c >= 0) {
                        break;
                    }
                }
            }
            break;
        }
        s -= 1;
        c -= 1;
    }
    String::new()
}

fn last(items: &[i32]) -> isize {
    items.len() as isize - 1
}

fn at(items: &[i32], i: isize) -> i32 {
    items[i as usize]
}

fn main() {
    let sample = "Error\n    at new Button (app.js:3)\n    at Root (react.js:9)\n    at main";
    let control = "Error\n    at fake (react.js:5)\n    at Root (react.js:9)\n    at main";
    println!("{:?}", frame(sample, control));
    // No common root: the stacks matched from their ends.
    let sample = "Error\n    at Inner (a.js:1)\n    at Outer (a.js:2)\n    at run";
    let control = "Error\n    at fake (b.js:1)\n    at run";
    println!("{:?}", frame(sample, control));
    println!("{:?}", frame("Error", "Error"));
    println!("{:?}", frame("", "Error\nRoot"));
    println!("{} {}", last(&[]), last(&[1, 2, 3]));
    println!("{}", at(&[4, 5], black_box(1)));
    let negative = std::panic::catch_unwind(|| at(&[4, 5], black_box(-1)));
    println!("{}", negative.is_err());
}
