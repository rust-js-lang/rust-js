// std's files shared (#1397): a `&File` reads, writes and seeks as the
// `File` does, at the one position each handle to it moves; and `{:?}` of a
// `File` is std's of unix, `File { fd, path, read, write }`, its path what
// it resolves to, its descriptor the platform's own, so it's left out here,
// as is the path of the outcome's file. Each file is the case's own, beside
// the file its outcome goes to.
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::panic::{AssertUnwindSafe, catch_unwind};

fn read_all<R: Read>(mut from: R) -> io::Result<String> {
    let mut text = String::new();
    from.read_to_string(&mut text)?;
    Ok(text)
}

fn tell<S: Seek>(mut at: S) -> io::Result<u64> {
    at.stream_position()
}

fn log_to<W: Write>(mut out: W, words: &[&str]) -> io::Result<()> {
    for word in words {
        write!(out, "{word};")?;
    }
    out.flush()
}

// What `{:?}` shows of it, its descriptor's number and the path of the
// outcome's file left out, neither the same twice: a descriptor is `N`, and
// how wide it's shown where it's padded, its padding's and its digits'.
fn shown(text: &str, base: &str) -> String {
    let mut out = String::new();
    let mut rest = text.replace(base, "<base>");
    while let Some(at) = rest.find("fd: ") {
        let after = &rest[at + 4..];
        let padding = after.len() - after.trim_start().len();
        let wide = padding + after[padding..].chars().take_while(char::is_ascii_digit).count();
        let padded = if padding > 0 { format!(" of width {wide}") } else { String::new() };
        out.push_str(&format!("{}N{padded}", &rest[..at + 4]));
        rest = rest[at + 4 + wide..].to_string();
    }
    out + &rest
}

fn main() {
    if let Err(e) = run() {
        println!("failed: {e}");
    }
}

fn run() -> io::Result<()> {
    let base = std::env::args().nth(1).expect("the outcome's file");
    let path = format!("{base}.shared.txt");

    // Written through `&File`, as it is and through generic code.
    let file = File::create(&path)?;
    (&file).write_all(b"one\ntwo\n")?;
    let mut writer = &file;
    writeln!(writer, "three")?;
    log_to(&file, &["four", "five"])?;
    println!("{:?} {:?}", (&file).stream_position()?, tell(&file)?);
    drop(file);

    // Two `&File`s of one file read on from where the other stopped.
    let file = File::open(&path)?;
    let mut a = &file;
    let mut b = &file;
    let mut four = [0u8; 4];
    a.read_exact(&mut four)?;
    b.read_exact(&mut four)?;
    println!("{four:?} {:?}", a.stream_position()?);
    a.seek(SeekFrom::Start(0))?;
    let lines: Vec<String> = BufReader::new(&file).lines().collect::<io::Result<_>>()?;
    println!("{lines:?}");
    println!("{:?}", tell(&file)?);
    (&file).rewind()?;
    println!("{:?}", read_all(&file)?);
    println!("{:?}", read_all(&file)?);

    // Written and read through one `&File`, opened to both.
    let both = OpenOptions::new().read(true).write(true).open(&path)?;
    (&both).seek(SeekFrom::End(-5))?;
    (&both).write_all(b"FIVE")?;
    (&both).seek(SeekFrom::Current(-4))?;
    println!("{:?}", read_all(&both)?);

    // `{:?}` of a file, as std shows it: its path and modes.
    let reader = File::open(&path)?;
    println!("{}", shown(&format!("{reader:?}"), &base));
    println!("{}", shown(&format!("{both:?}"), &base));
    println!("{}", shown(&format!("{:#?}", File::create(&path)?), &base));
    println!("{}", shown(&format!("[{both:5?}]"), &base));
    let appending = OpenOptions::new().append(true).open(&path)?;
    println!("{}", shown(&format!("{appending:?}"), &base));
    println!("{}", shown(&format!("{:?}", File::open(&path)), &base));

    // `unwrap_err()` of a file opened: its panic shows the file.
    let opened = File::open(&path);
    let message = match catch_unwind(AssertUnwindSafe(|| opened.unwrap_err())) {
        Ok(e) => format!("no panic: {e}"),
        Err(payload) => payload.downcast_ref::<String>().cloned().unwrap_or_default(),
    };
    println!("{}", shown(&message, &base));
    let missing = File::open(format!("{base}.missing")).unwrap_err();
    println!("{:?}", missing.kind());

    fs::remove_file(&path)?;
    Ok(())
}
