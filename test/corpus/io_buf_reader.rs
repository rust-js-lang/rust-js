//@ library-differs
// `io::BufReader<R>` (#1156): std's fields flattened, `{ inner, buf, pos,
// filled }`, its `buf` an array of its capacity. Each method of its `Read`,
// `BufRead` and `Seek` is std's body over its buffer: a read as large as the
// buffer goes past it where nothing's buffered, `fill_buf` is one read of the
// whole buffer, `read_exact`, `read_to_end` and `read_to_string` use what's
// buffered first, and `seek` and `seek_relative` keep what's buffered when
// they can. What its inner is asked shows in where it is.
use std::io::{self, BufRead, BufReader, Cursor, Read, Seek, SeekFrom};

fn pos(r: &BufReader<Cursor<Vec<u8>>>) -> u64 {
    r.get_ref().position()
}

fn cursor(bytes: &[u8]) -> Cursor<Vec<u8>> {
    Cursor::new(bytes.to_vec())
}

fn total<R: Read>(mut r: R) -> usize {
    let mut all = Vec::new();
    r.read_to_end(&mut all).unwrap();
    all.len()
}

fn first_line<B: BufRead + ?Sized>(r: &mut B) -> String {
    let mut line = String::new();
    r.read_line(&mut line).unwrap();
    line
}

fn at<S: Seek>(s: &mut S) -> u64 {
    s.stream_position().unwrap()
}

// A `BufReader` of a type parameter: its `Read` asks `R`'s `Read`, its `Seek`
// `R`'s `Seek`, and each bound alone is enough.
fn fill<R: Read>(r: &mut BufReader<R>) -> Vec<u8> {
    r.fill_buf().unwrap().to_vec()
}

fn tell<R: Seek>(r: &mut BufReader<R>) -> u64 {
    r.seek_relative(1).unwrap();
    r.stream_position().unwrap()
}

fn read_and_seek<R: Read + Seek>(mut r: BufReader<R>) -> (Vec<u8>, u64) {
    let mut two = [0u8; 2];
    r.read_exact(&mut two).unwrap();
    r.seek(SeekFrom::Current(-1)).unwrap();
    (two.to_vec(), r.stream_position().unwrap())
}

fn reads() {
    let mut r = BufReader::with_capacity(4, cursor(b"abcdefghij"));
    println!("{} {:?} {}", r.capacity(), r.buffer(), pos(&r));
    println!("{:?}", r.fill_buf().unwrap());
    println!("{:?} {}", r.buffer(), pos(&r));
    r.consume(1);
    println!("{:?} {}", r.buffer(), pos(&r));
    // What's buffered answers a small read: nothing more is read from the
    // inner.
    let mut two = [0u8; 2];
    println!("{} {two:?}", r.read(&mut two).unwrap());
    println!("{:?} {two:?} {}", r.buffer(), pos(&r));
    let mut one = [0u8; 1];
    println!("{} {one:?} {:?} {}", r.read(&mut one).unwrap(), r.buffer(), pos(&r));
    // Nothing's buffered and the read is as large as the buffer: it goes
    // past it.
    let mut four = [0u8; 4];
    println!("{} {four:?} {:?} {}", r.read(&mut four).unwrap(), r.buffer(), pos(&r));
    // A consume past what's buffered stops at it.
    println!("{:?}", r.fill_buf().unwrap());
    r.consume(10);
    println!("{:?} {}", r.buffer(), pos(&r));
    println!("{}", r.stream_position().unwrap());
    println!("{:?}", r.fill_buf().unwrap());
    println!("{:?}", r.buffer());
    let inner = r.into_inner();
    println!("{}", inner.position());

    let mut r = BufReader::new(cursor(b"hello"));
    println!("{} {}", r.capacity(), BufReader::with_capacity(0, cursor(b"")).capacity());
    let mut big = [0u8; 8];
    println!("{} {big:?} {:?}", r.read(&mut big).unwrap(), r.buffer());
    println!("{}", r.read(&mut big).unwrap());

    // A buffer of no room reads from the inner each time.
    let mut none = BufReader::with_capacity(0, cursor(b"xyz"));
    println!("{:?}", none.fill_buf().unwrap());
    println!("{}", none.read(&mut one).unwrap());
    println!("{one:?} {}", pos(&none));
}

fn exact_and_all() {
    let mut r = BufReader::with_capacity(4, cursor(b"abcdefghij"));
    let mut three = [0u8; 3];
    r.read_exact(&mut three).unwrap();
    println!("{three:?} {:?} {}", r.buffer(), pos(&r));
    r.fill_buf().unwrap();
    r.read_exact(&mut three[..1]).unwrap();
    println!("{three:?} {:?} {}", r.buffer(), pos(&r));
    let mut six = [0u8; 6];
    r.read_exact(&mut six).unwrap();
    println!("{six:?} {:?} {}", r.buffer(), pos(&r));
    let mut more = [0u8; 3];
    let e = r.read_exact(&mut more).unwrap_err();
    println!("{:?} {e} {more:?} {:?} {}", e.kind(), r.buffer(), pos(&r));

    let mut r = BufReader::with_capacity(3, cursor(b"0123456789"));
    r.fill_buf().unwrap();
    r.consume(1);
    let mut v = vec![b'x'];
    println!("{} {v:?} {:?} {}", r.read_to_end(&mut v).unwrap(), r.buffer(), pos(&r));
    println!("{}", v.capacity());
    println!("{}", r.read_to_end(&mut v).unwrap());

    let mut r = BufReader::with_capacity(3, cursor("héllo wörld".as_bytes()));
    r.fill_buf().unwrap();
    let mut s = String::new();
    println!("{} {s:?}", r.read_to_string(&mut s).unwrap());
    let mut r = BufReader::with_capacity(3, cursor("héllo wörld".as_bytes()));
    r.fill_buf().unwrap();
    let mut s = String::from(">> ");
    println!("{} {s:?}", r.read_to_string(&mut s).unwrap());
    println!("{s:?} {} {}", r.read_to_string(&mut s).unwrap(), pos(&r));

    let mut r = BufReader::with_capacity(3, cursor(&[b'a', 0xff, b'b']));
    let mut s = String::new();
    let e = r.read_to_string(&mut s).unwrap_err();
    println!("{:?} {e} {s:?} {}", e.kind(), pos(&r));
    let mut r = BufReader::with_capacity(3, cursor(&[b'a', 0xff, b'b']));
    let mut s = String::from("k");
    let e = r.read_to_string(&mut s).unwrap_err();
    println!("{:?} {e} {s:?} {}", e.kind(), pos(&r));
}

fn lines_and_more() {
    let mut r = BufReader::with_capacity(4, cursor(b"one\ntwo\r\nthree"));
    println!("{:?}", first_line(&mut r));
    println!("{:?} {:?}", r.buffer(), pos(&r));
    let rest: Vec<String> = r.lines().map(|l| l.unwrap()).collect();
    println!("{rest:?}");

    let mut r = BufReader::with_capacity(2, cursor(b"a,bb,,c"));
    let mut part = Vec::new();
    println!("{} {:?}", r.read_until(b',', &mut part).unwrap(), part);
    println!("{}", r.skip_until(b',').unwrap());
    let parts: Vec<Vec<u8>> = r.split(b',').map(|p| p.unwrap()).collect();
    println!("{parts:?}");

    let bytes: Vec<u8> = BufReader::with_capacity(3, cursor(b"pqrst")).bytes().map(|b| b.unwrap()).collect();
    println!("{bytes:?}");
}

fn shown(r: &mut BufReader<Cursor<Vec<u8>>>) {
    let stream = r.stream_position().unwrap();
    println!("{:?} {} {stream}", r.buffer(), pos(r));
}

fn seeks() {
    let mut r = BufReader::with_capacity(4, cursor(b"0123456789"));
    r.fill_buf().unwrap();
    r.consume(1);
    println!("{} {:?}", r.stream_position().unwrap(), r.buffer());
    println!("{}", r.seek(SeekFrom::Current(1)).unwrap());
    println!("{:?} {}", r.buffer(), pos(&r));
    println!("{}", r.seek(SeekFrom::Start(6)).unwrap());
    let mut two = [0u8; 2];
    r.read_exact(&mut two).unwrap();
    println!("{two:?} {} {}", r.stream_position().unwrap(), pos(&r));
    println!("{}", r.seek(SeekFrom::End(-3)).unwrap());
    let e = r.seek(SeekFrom::End(-30)).unwrap_err();
    println!("{:?} {e}", e.kind());
    println!("{} {:?}", r.stream_position().unwrap(), r.buffer());
    r.rewind().unwrap();
    println!("{}", at(&mut r));

    // `seek_relative` stays inside what's buffered where it can.
    let mut r = BufReader::with_capacity(6, cursor(b"abcdefghijklmnop"));
    r.fill_buf().unwrap();
    r.consume(2);
    r.seek_relative(2).unwrap();
    shown(&mut r);
    r.seek_relative(-3).unwrap();
    shown(&mut r);
    r.seek_relative(-1).unwrap();
    shown(&mut r);
    let e = r.seek_relative(-3).unwrap_err();
    println!("{:?}", e.kind());
    shown(&mut r);
    r.seek_relative(10).unwrap();
    shown(&mut r);
    r.seek_relative(-1).unwrap();
    shown(&mut r);
    let e = r.seek_relative(-100).unwrap_err();
    println!("{:?}", e.kind());
    // One byte past what's buffered seeks its inner; the farthest back there
    // is overflows the offset, which moves it back what's buffered first.
    let mut r = BufReader::with_capacity(4, cursor(b"0123456789"));
    r.fill_buf().unwrap();
    r.seek_relative(5).unwrap();
    shown(&mut r);
    r.fill_buf().unwrap();
    let e = r.seek(SeekFrom::Current(i64::MIN)).unwrap_err();
    println!("{:?} {} {:?}", e.kind(), pos(&r), r.buffer());
    r.fill_buf().unwrap();
    // What's buffered is more than where its inner says it is: std's
    // `expect` fails.
    r.get_mut().set_position(1);
    let failed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| r.stream_position()));
    println!("{}", failed.is_err());
}

fn through_others() {
    let c = (&b"ab"[..]).chain(&b"cd"[..]);
    let mut r = BufReader::new(c);
    println!("{:?}", r.fill_buf().unwrap());
    let mut s = String::new();
    r.read_to_string(&mut s).unwrap();
    println!("{s:?}");

    let mut r = BufReader::with_capacity(3, io::repeat(7).take(5));
    let mut v = Vec::new();
    println!("{} {v:?}", r.read_to_end(&mut v).unwrap());
    println!("{v:?}");

    let mut from_slice = BufReader::with_capacity(2, &b"slice"[..]);
    println!("{:?}", from_slice.fill_buf().unwrap());
    from_slice.consume(1);
    println!("{} {:?}", total(&mut from_slice), from_slice.into_inner());

    let mut nested = BufReader::with_capacity(3, BufReader::with_capacity(2, &b"nested!"[..]));
    println!("{:?}", nested.fill_buf().unwrap());
    println!("{:?}", nested.get_ref().buffer());
    let mut s = String::new();
    nested.read_to_string(&mut s).unwrap();
    println!("{s:?}");

    let mut text = BufReader::new("empty".as_bytes());
    println!("{}", total(&mut text));
    println!("{}", total(BufReader::new(io::empty())));
}

fn through_generics() {
    let mut r = BufReader::with_capacity(3, &b"x\ny\nz"[..]);
    let by_ref: &mut dyn BufRead = &mut r;
    println!("{:?}", first_line(&mut *by_ref));
    let mut boxed: Box<dyn BufRead> = Box::new(BufReader::with_capacity(2, &b"boxed\nlines"[..]));
    println!("{:?}", first_line(&mut boxed));
    println!("{:?}", boxed.lines().map(|l| l.unwrap()).collect::<Vec<_>>());
    let mut dynamic: Box<dyn Read> = Box::new(BufReader::new(&b"read me"[..]));
    println!("{}", total(&mut dynamic));
    let mut seekable: Box<dyn Seek> = Box::new(BufReader::new(cursor(b"seek me")));
    println!("{}", at(&mut seekable));
    println!("{}", total(BufReader::with_capacity(2, &b"count"[..])));
    println!("{}", total(BufReader::new(cursor(b"counted"))));
}

fn generic_readers() {
    let mut r = BufReader::with_capacity(3, cursor(b"abcdefg"));
    println!("{:?} {}", fill(&mut r), tell(&mut r));
    println!("{:?} {}", fill(&mut r), tell(&mut r));
    println!("{:?}", read_and_seek(BufReader::with_capacity(4, cursor(b"wxyz"))));
    println!("{:?}", fill(&mut BufReader::new(&b"slice"[..])));
}

fn capacity_of(n: usize) -> usize {
    println!("capacity asked");
    n
}

fn inner_made() -> &'static [u8] {
    println!("inner made");
    b"made"
}

// Room for a buffer past what memory holds is std's `capacity overflow`,
// asked once both arguments are evaluated: the inner is made first.
fn too_large() {
    let made = std::panic::catch_unwind(|| BufReader::with_capacity(usize::MAX, &b""[..]).capacity());
    println!("{}", made.is_err());
    let made = std::panic::catch_unwind(|| BufReader::with_capacity(capacity_of(usize::MAX), inner_made()).capacity());
    println!("{}", made.is_err());
    let mut fits = BufReader::with_capacity(capacity_of(2), inner_made());
    println!("{:?}", fits.fill_buf().unwrap());
}

fn twice(a: &[u8], b: &[u8]) -> (String, String) {
    let mut one = String::new();
    BufReader::new(a).read_to_string(&mut one).unwrap();
    let mut two = String::new();
    BufReader::new(b).read_to_string(&mut two).unwrap();
    (one, two)
}

fn main() {
    reads();
    exact_and_all();
    lines_and_more();
    seeks();
    through_others();
    through_generics();
    generic_readers();
    too_large();
    println!("{:?}", twice(b"first", b"second"));
}
