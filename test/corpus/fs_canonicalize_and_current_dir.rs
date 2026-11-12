// A path made canonical, `fs::canonicalize` and `Path::canonicalize`, and
// the working directory, `env::current_dir()`, quickcheck 0.9's `PathBuf`
// (#1420): `realpath`'s, each `.`, `..` and link resolved, and the OS's
// error of a missing file. Each file is the case's own, beside the file its
// outcome goes to.
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

fn report<T: std::fmt::Debug>(what: &str, result: io::Result<T>) {
    match result {
        Ok(value) => println!("{what}: {value:?}"),
        Err(e) => println!("{what}: {e} | {:?}", e.kind()),
    }
}

fn main() {
    if let Err(e) = run() {
        println!("failed: {e}");
    }
}

fn run() -> io::Result<()> {
    let base = std::env::args().nth(1).expect("the outcome's file");
    let root = PathBuf::from(format!("{base}.canonical"));
    let _ = fs::remove_file(root.join("x"));
    let _ = fs::remove_dir(&root);
    fs::create_dir(&root)?;
    fs::write(root.join("x"), "x")?;
    let real = fs::canonicalize(&root)?;
    println!("absolute {}", real.is_absolute());
    let dotted = root.join(".").join("x");
    let through = Path::new(&format!("{}/../{}/x", root.display(), root.file_name().unwrap().to_str().unwrap()))
        .canonicalize()?;
    println!("x {} {}", dotted.canonicalize()? == real.join("x"), through == real.join("x"));
    report("missing", fs::canonicalize(root.join("missing")));
    report("missing by its method", root.join("y/z").canonicalize());
    let here = std::env::current_dir()?;
    println!("current {} {}", here.is_absolute(), fs::canonicalize(".")? == fs::canonicalize(&here)?);
    fs::remove_file(root.join("x"))?;
    fs::remove_dir(&root)?;
    Ok(())
}
