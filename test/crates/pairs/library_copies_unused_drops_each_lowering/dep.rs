//! A library's generic functions, each lowered as two copies (ADR 0051),
//! `Chunked::new` and `Chunked::encode` of a sized engine and of one that
//! may not be: each copy declares the engine's drop, which neither uses, a
//! name of its own (ADR 1150), and each leaves it out (ADR 0300). base64's
//! `ChunkedEncoder`, whose copy kept it and wasn't merged with the other,
//! left its `const`s' hole unfilled and its crate refused.

pub trait Sink {
    type Error;
    fn write(&mut self, encoded: &[u8]) -> Result<(), Self::Error>;
}

pub trait Engine {
    fn pad(&self) -> bool;

    fn encode_string(&self, input: &[u8], out: &mut String) {
        fn inner<E>(engine: &E, input: &[u8], out: &mut String)
        where
            E: Engine + ?Sized,
        {
            let mut sink = StringSink { out };
            Chunked::new(engine)
                .encode(input, &mut sink)
                .expect("a String takes it");
        }
        inner(self, input, out)
    }
}

pub struct Chunked<'e, E: Engine + ?Sized> {
    engine: &'e E,
}

impl<'e, E: Engine + ?Sized> Chunked<'e, E> {
    pub fn new(engine: &'e E) -> Chunked<'e, E> {
        Chunked { engine }
    }

    pub fn encode<S: Sink>(&self, bytes: &[u8], sink: &mut S) -> Result<(), S::Error> {
        const BUF_SIZE: usize = 8;
        const CHUNK_SIZE: usize = BUF_SIZE / 4 * 3;
        let mut buf = [0u8; BUF_SIZE];
        for chunk in bytes.chunks(CHUNK_SIZE) {
            let mut len = chunk.len();
            buf[..len].copy_from_slice(chunk);
            if chunk.len() != CHUNK_SIZE && self.engine.pad() {
                buf[len] = b'=';
                len += 1;
            }
            sink.write(&buf[..len])?;
        }
        Ok(())
    }
}

pub struct StringSink<'a> {
    out: &'a mut String,
}

impl Sink for StringSink<'_> {
    type Error = ();
    fn write(&mut self, encoded: &[u8]) -> Result<(), ()> {
        self.out.push_str(std::str::from_utf8(encoded).unwrap());
        Ok(())
    }
}

pub struct Display<'a, 'e, E: Engine> {
    bytes: &'a [u8],
    chunked: Chunked<'e, E>,
}

impl<'a, 'e, E: Engine> Display<'a, 'e, E> {
    pub fn new(bytes: &'a [u8], engine: &'e E) -> Display<'a, 'e, E> {
        Display {
            bytes,
            chunked: Chunked::new(engine),
        }
    }
}

impl<E: Engine> std::fmt::Display for Display<'_, '_, E> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mut sink = FormatterSink { f };
        self.chunked.encode(self.bytes, &mut sink)
    }
}

struct FormatterSink<'a, 'b> {
    f: &'a mut std::fmt::Formatter<'b>,
}

impl Sink for FormatterSink<'_, '_> {
    type Error = std::fmt::Error;
    fn write(&mut self, encoded: &[u8]) -> Result<(), std::fmt::Error> {
        self.f.write_str(std::str::from_utf8(encoded).unwrap())
    }
}
