// A late `let` the program declares before an `if` it wrote with an `else`
// whose first branch returns stays before the `if`, in the scope that
// reads it after: `let x; if c { return 0; } else if let Some(p) = o { x =
// p + 1; } else { x = 7; } x * 2`, as React's Flight server's
// renderFunctionComponent writes its `debugInfo`. Turning MIR's guard back
// into the `else` it was made it the `else`'s, and the read after threw
// `x is not defined` (rd578).
pub struct Task {
    pub debug: Option<Vec<u32>>,
}

fn emit(out: &mut Vec<u32>, n: u32) -> u32 {
    out.push(n);
    n
}

fn f(c: bool, o: Option<u32>) -> u32 {
    let x;
    if c {
        return 0;
    } else if let Some(p) = o {
        x = p + 1;
    } else {
        x = 7;
    }
    x * 2
}

fn outlined(c: bool, task: &Task, out: &mut Vec<u32>) -> u32 {
    let debug_info;
    emit(out, 100);
    if c {
        return 0;
    } else if let Some(p) = &task.debug {
        debug_info = emit(out, p.len() as u32);
    } else {
        debug_info = emit(out, 7);
    }
    emit(out, debug_info);
    debug_info + 1
}

fn plain_else(c: bool, out: &mut Vec<u32>) -> u32 {
    let x;
    if c {
        return 1;
    } else {
        x = emit(out, 2);
    }
    emit(out, x) + x
}

fn main() {
    let mut out = Vec::new();
    let t = Task { debug: Some(vec![1, 2]) };
    let n = Task { debug: None };
    println!("{} {} {}", f(true, Some(3)), f(false, Some(3)), f(false, None));
    println!(
        "{} {} {}",
        outlined(true, &t, &mut out),
        outlined(false, &t, &mut out),
        outlined(false, &n, &mut out)
    );
    println!("{} {}", plain_else(true, &mut out), plain_else(false, &mut out));
    println!("{out:?}");
}
