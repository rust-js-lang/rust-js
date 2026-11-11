//@ library-differs
// std's directories and what the OS says of a file (#1397): `create_dir`,
// `create_dir_all` and `read_dir`, its entries in the OS's order, each
// entry's path, name, type and metadata; a file's `metadata()`, `set_len`,
// `sync_all` and `sync_data`; `remove_dir`; and their errors, the OS's, and
// std's own of a length past an `off_t`. Each file is the case's own, beside
// the file its outcome goes to.
use std::fs::{self, File};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::time::SystemTime;

fn report<T: std::fmt::Debug>(what: &str, result: io::Result<T>) {
    match result {
        Ok(value) => println!("{what}: {value:?}"),
        Err(e) => println!("{what}: {e} | {e:?} | {:?}", e.kind()),
    }
}

const NAMES: [&str; 6] = ["zeta", "alpha", "Mid", "b2", "b10", "é"];

// What a run before left, taken away: each file, then each directory, the
// deepest first.
fn clear(root: &str) {
    for name in NAMES {
        let _ = fs::remove_file(format!("{root}/a/{name}"));
    }
    for dir in ["a/b", "a", ""] {
        let _ = fs::remove_dir(format!("{root}/{dir}"));
    }
}

fn main() {
    if let Err(e) = run() {
        println!("failed: {e}");
    }
}

fn run() -> io::Result<()> {
    let base = std::env::args().nth(1).expect("the outcome's file");
    let root = format!("{base}.dirs");
    clear(&root);

    // Made, its parents first, or found there; refused where a parent's
    // missing or a file's in the way.
    report("create_dir of a missing parent", fs::create_dir(format!("{root}/a/b")));
    report("create_dir_all", fs::create_dir_all(format!("{root}/a/b")));
    report("create_dir_all again", fs::create_dir_all(format!("{root}/a/b/")));
    report("create_dir again", fs::create_dir(format!("{root}/a")));
    for name in NAMES {
        fs::write(format!("{root}/a/{name}"), name)?;
    }
    report("create_dir_all through a file", fs::create_dir_all(format!("{root}/a/zeta/x")));
    report("create_dir of a file", fs::create_dir(format!("{root}/a/zeta")));
    report("create_dir_all of nothing", fs::create_dir_all(""));

    // Read in the OS's order, each entry's path, name and type.
    let names: Vec<_> = fs::read_dir(format!("{root}/a"))?.map(|entry| entry.map(|e| e.file_name())).collect::<io::Result<_>>()?;
    let mut sorted = names.clone();
    sorted.sort();
    println!("{names:?} {sorted:?}");
    for entry in fs::read_dir(format!("{root}/a"))? {
        let entry = entry?;
        let kind = entry.file_type()?;
        let len = if kind.is_file() { entry.metadata()?.len() } else { 0 };
        println!("{:?} {:?} {} {} {} {len}", entry.path().strip_prefix(&root), entry.file_name(), kind.is_file(), kind.is_dir(), kind.is_symlink());
    }
    let mut entries = fs::read_dir(format!("{root}/a/"))?;
    let first = entries.next().expect("an entry")?;
    println!("{:?} {}", first.path().strip_prefix(&root), entries.count());
    report("read_dir of a missing one", fs::read_dir(format!("{root}/missing")).map(|_| ()));
    report("read_dir of a file", fs::read_dir(format!("{root}/a/zeta")).map(|_| ()));

    // What the OS says of a file and a directory.
    let m = fs::metadata(format!("{root}/a/alpha"))?;
    println!("{} {} {} {}", m.len(), m.is_file(), m.is_dir(), m.is_symlink());
    let t = m.file_type();
    println!("{} {} {:?} {}", t.is_file(), t.is_dir(), t, t == fs::metadata(&root)?.file_type());
    let d = fs::metadata(format!("{root}/a"))?;
    println!("{} {} {:?} {:#?}", d.is_dir(), d.file_type().is_dir(), d.file_type(), d.file_type());
    println!("{}", fs::symlink_metadata(format!("{root}/a"))?.is_dir());
    report("metadata of a missing one", fs::metadata(format!("{root}/nope")).map(|m| m.len()));
    let before = SystemTime::now();
    let modified = m.modified()?;
    println!("{}", modified <= before && m.accessed()? <= before);

    // A file's length set, its position kept; written, then synced.
    let path = format!("{root}/a/alpha");
    let mut f = File::options().read(true).write(true).open(&path)?;
    f.seek(SeekFrom::Start(3))?;
    f.set_len(10)?;
    println!("{:?} {}", f.stream_position()?, f.metadata()?.len());
    f.set_len(2)?;
    f.seek(SeekFrom::End(0))?;
    f.write_all(b"!")?;
    f.sync_all()?;
    f.sync_data()?;
    let mut all = Vec::new();
    f.rewind()?;
    f.read_to_end(&mut all)?;
    println!("{all:?} {}", (&f).metadata()?.is_file());
    let reader = File::open(&path)?;
    report("set_len of a reader", reader.set_len(1));
    report("set_len past an off_t", f.set_len(u64::MAX));

    // Taken away, a file at a time, then each directory, the deepest first.
    for name in NAMES {
        fs::remove_file(format!("{root}/a/{name}"))?;
    }
    report("remove_dir of one that holds one", fs::remove_dir(format!("{root}/a")));
    fs::remove_dir(format!("{root}/a/b"))?;
    fs::remove_dir(format!("{root}/a"))?;
    fs::remove_dir(&root)?;
    report("remove_dir of a missing one", fs::remove_dir(&root));
    println!("{}", fs::metadata(&root).is_err());
    Ok(())
}
