//! What a consumer compiles itself of a library's (ADR 0914), as rand
//! compiles rand_core's `OsRng` and ppv-lite86's lanes: `?` that converts
//! its error with the library's own `From`, and `transmute`s of bytes to a
//! struct and back, which rustc's MIR makes casts (#1420).
use std::num::NonZeroU32;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lanes([u32; 4]);

pub use error::{Error, ShortInput};

/// The errors, a module of their own, as rand_core's: its `From`, called
/// from another module, its JS exports.
pub mod error {
    use std::num::NonZeroU32;

    #[derive(Debug)]
    pub struct ShortInput(pub usize);

    #[derive(Debug, PartialEq)]
    pub struct Error {
        pub code: NonZeroU32,
    }

    impl From<ShortInput> for Error {
        fn from(short: ShortInput) -> Self {
            Error {
                code: NonZeroU32::new(short.0 as u32 + 100).unwrap(),
            }
        }
    }
}

pub const TOO_LONG: Error = Error {
    code: NonZeroU32::new(1).unwrap(),
};

fn sixteen(input: &[u8]) -> Result<&[u8], ShortInput> {
    if input.len() < 16 {
        return Err(ShortInput(input.len()));
    }
    Ok(input)
}

/// `input`'s first byte, of 16: the library's own `?`, whose `From` its JS
/// exports.
pub fn first(input: &[u8]) -> Result<u8, Error> {
    Ok(sixteen(input)?[0])
}

/// `input`'s first 16 bytes as lanes, and `keep`, which the library's JS
/// takes no drop of: its caller compiles it.
pub fn read_lanes<D>(input: &[u8], keep: D) -> Result<(Lanes, D), Error> {
    let bytes = sixteen(input)?;
    if bytes.len() > 16 {
        return Err(TOO_LONG);
    }
    let lanes: Lanes = unsafe { std::mem::transmute(std::ptr::read(bytes as *const [u8] as *const [u8; 16])) };
    Ok((lanes, keep))
}

/// `lanes`' bytes written to `out`, and `keep` given back.
pub fn write_lanes<D>(lanes: Lanes, out: &mut [u8], keep: D) -> D {
    assert_eq!(out.len(), 16);
    let swapped = Lanes([lanes.0[3], lanes.0[2], lanes.0[1], lanes.0[0]]);
    unsafe { std::ptr::write(out as *mut [u8] as *mut [u8; 16], std::mem::transmute(swapped)) }
    keep
}

/// What an error says, matched against the library's constant.
pub fn describe<D>(error: &Error, keep: D) -> (&'static str, D) {
    let said = match *error {
        TOO_LONG => "too long",
        _ => "short",
    };
    (said, keep)
}
