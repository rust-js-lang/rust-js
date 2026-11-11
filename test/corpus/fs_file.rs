// std's files (#1156), through Node's fs: a `File` is `{ fd, pos }`, read
// and written at its position, which `Seek` moves; `OpenOptions` are std's
// flags, and an error of the OS's is std's `Os` error, its kind and message
// std's. Each file is the case's own, beside the file its outcome goes to.
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, Read, Seek, SeekFrom, Write};

fn save<W: Write>(out: &mut W, lines: &[&str]) -> io::Result<()> {
    for line in lines {
        writeln!(out, "{line}")?;
    }
    out.flush()
}

fn first_word<R: Read>(mut from: R) -> io::Result<String> {
    let mut text = String::new();
    from.read_to_string(&mut text)?;
    Ok(text.split_whitespace().next().unwrap_or("").to_string())
}

fn report(what: &str, e: &io::Error) {
    println!("{what}: {e} | {e:?} | {:?} {:?}", e.kind(), e.raw_os_error());
}

fn refused(options: &OpenOptions, path: &str) {
    let e = options.open(path).err().expect("refused");
    println!("{e} | {e:?} | {:?}", e.raw_os_error());
}

fn main() {
    if let Err(e) = run() {
        println!("failed: {e}");
    }
}

fn run() -> io::Result<()> {
    let base = std::env::args().nth(1).expect("the outcome's file");
    let notes = format!("{base}.notes.txt");

    // Written, then read back whole, by line, and from generic code.
    let mut file = File::create(&notes)?;
    file.write_all(b"alpha beta\n")?;
    save(&mut file, &["gamma", "delta"])?;
    write!(file, "{}-{}", 1, 2)?;
    println!("{:?}", file.stream_position()?);
    drop(file);
    println!("{:?}", fs::read_to_string(&notes)?);
    let lines: Vec<String> = BufReader::new(File::open(&notes)?).lines().collect::<io::Result<_>>()?;
    println!("{lines:?}");
    println!("{:?}", first_word(File::open(&notes)?)?);

    // Read in parts, and moved about.
    let mut file = File::open(&notes)?;
    let mut four = [0u8; 4];
    file.read_exact(&mut four)?;
    println!("{four:?} {:?}", file.stream_position()?);
    println!("{:?}", file.seek(SeekFrom::End(-3))?);
    let mut rest = Vec::new();
    println!("{:?} {rest:?}", file.read_to_end(&mut rest)?);
    println!("{:?}", file.read(&mut four)?);
    file.rewind()?;
    println!("{:?}", file.seek(SeekFrom::Current(6))?);
    let e = file.seek(SeekFrom::Current(-7)).unwrap_err();
    report("before its start", &e);
    let e = file.read_exact(&mut [0u8; 64]).unwrap_err();
    println!("{e} {:?}", e.kind());

    // Past its end, a write leaves a hole of zeros.
    let mut file = OpenOptions::new().read(true).write(true).open(&notes)?;
    file.seek(SeekFrom::End(2))?;
    file.write_all(b"!")?;
    file.rewind()?;
    let mut all = Vec::new();
    file.read_to_end(&mut all)?;
    println!("{:?}", &all[all.len() - 4..]);

    // Appended, wherever it's moved to; created anew, or refused where it is.
    let log = format!("{base}.log");
    let mut options = OpenOptions::new();
    options.append(true).create(true);
    for word in ["one", "two"] {
        let mut file = options.open(&log)?;
        file.seek(SeekFrom::Start(0))?;
        file.write_all(word.as_bytes())?;
        println!("{:?}", file.stream_position()?);
    }
    println!("{:?}", fs::read(&log)?);
    match File::options().write(true).create_new(true).open(&log) {
        Ok(_) => println!("made"),
        Err(e) => report("made again", &e),
    }
    let mut file = File::options().write(true).truncate(true).open(&log)?;
    file.write_all(b"z")?;
    println!("{:?}", fs::read_to_string(&log)?);

    // What std refuses before it opens anything.
    refused(&OpenOptions::new(), &log);
    refused(OpenOptions::new().create(true), &log);
    refused(OpenOptions::new().read(true).truncate(true), &log);
    refused(OpenOptions::new().append(true).truncate(true), &log);

    // The OS's errors: a file that isn't there, a write to a file opened to
    // read, a directory read, bytes that aren't text.
    let missing = format!("{base}.missing");
    report("open", &File::open(&missing).err().expect("missing"));
    report("read", &fs::read(&missing).unwrap_err());
    report("remove", &fs::remove_file(&missing).unwrap_err());
    let e = File::open(&notes)?.write_all(b"x").unwrap_err();
    report("write to a reader", &e);
    let e = File::create(&log)?.read(&mut four).unwrap_err();
    report("read a writer", &e);
    let dir = std::path::Path::new(&base).parent().expect("its directory").to_path_buf();
    report("directory", &fs::read_to_string(&dir).unwrap_err());
    fs::write(&log, [0xffu8, 0xfe])?;
    let e = fs::read_to_string(&log).unwrap_err();
    println!("{e} | {e:?}");
    let mut text = String::from("kept");
    let e = File::open(&log)?.read_to_string(&mut text).unwrap_err();
    println!("{e} {text:?}");

    fs::write(&log, "text")?;
    println!("{:?}", fs::read(&log)?);
    fs::remove_file(&log)?;
    fs::remove_file(&notes)?;
    println!("{} {}", File::open(&log).is_err(), File::open(&notes).is_err());
    Ok(())
}
