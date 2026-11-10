//@ native: wasm32
//@ library-differs
// A C string hashed through a `Hasher`, and its errors cloned and compared
// (#1157, ADR 1119). A `CStr`'s derived `Hash` is `[c_char]`'s and a
// `CString`'s its `Box<[u8]>`'s: its length, then its bytes with the nul,
// the same bytes whether `c_char` is signed or not; through std's
// `DefaultHasher`, a derived `Hash` of a struct that holds one, generic code's
// of a `T: Hash`, and a `BuildHasher`'s `hash_one`. Each error's derived
// `Clone` and `PartialEq` are its fields', `FromVecWithNulError`'s kind's
// too, and a clone's bytes are its own. Each was refused.
use std::collections::hash_map::DefaultHasher;
use std::ffi::{CStr, CString, FromBytesUntilNulError, IntoStringError, NulError};
use std::hash::{BuildHasher, BuildHasherDefault, Hash, Hasher};

fn hash_of<T: Hash + ?Sized>(value: &T) -> u64 {
    let mut h = DefaultHasher::new();
    value.hash(&mut h);
    h.finish()
}

#[derive(Hash)]
struct Named<'a> {
    name: &'a CStr,
    owned: CString,
}

#[derive(Clone, Debug, PartialEq)]
struct Failed {
    nul: NulError,
    until: FromBytesUntilNulError,
}

fn main() {
    // Hashed, a byte past 0x7f as its byte, the empty one its nul.
    let mut h = DefaultHasher::new();
    c"hi".hash(&mut h);
    println!("{}", h.finish());
    println!("{} {}", hash_of(c""), hash_of(c"\xff\x80a"));
    let owned = CString::new("hi").unwrap();
    println!("{} {}", hash_of(&owned), hash_of(owned.as_c_str()) == hash_of(c"hi"));
    println!("{}", hash_of(&(c"a", CString::new("b").unwrap())));
    let named = Named {
        name: c"n",
        owned: CString::new(vec![0x7fu8, 0xff]).unwrap(),
    };
    println!("{}", hash_of(&named));
    println!("{}", hash_of(&[c"x", c"yz"][..]));
    println!("{}", hash_of(&Some(Box::new(CString::default()))));
    let built = BuildHasherDefault::<DefaultHasher>::default();
    println!("{}", built.hash_one(c"one"));
    let mut both = DefaultHasher::new();
    c"ab".hash(&mut both);
    owned.hash(&mut both);
    println!("{}", both.finish());

    // Each error cloned and compared.
    let with = CStr::from_bytes_with_nul(b"a\0b\0").unwrap_err();
    let unended = CStr::from_bytes_with_nul(b"ab").unwrap_err();
    println!("{:?} {} {}", with.clone(), with == with.clone(), with == unended);
    let until = CStr::from_bytes_until_nul(b"x").unwrap_err();
    println!("{:?} {}", until.clone(), until == until.clone());
    let nul = CString::new("a\0b").unwrap_err();
    let copy = nul.clone();
    println!("{} {}", copy == nul, copy == CString::new("a\0c").unwrap_err());
    let mut bytes = copy.into_vec();
    bytes.push(9);
    println!("{bytes:?} {:?} {}", nul, nul.nul_position());
    let into: IntoStringError = CString::from_vec_with_nul(vec![0xc3, 0])
        .unwrap()
        .into_string()
        .unwrap_err();
    let again = into.clone();
    println!("{} {:?}", into == again, again.utf8_error());
    let mut inner = again.into_cstring().into_bytes_with_nul();
    inner[0] = b'A';
    println!("{inner:?} {:?}", into.into_cstring());
    let vec_err = CString::from_vec_with_nul(b"a\0b\0".to_vec()).unwrap_err();
    let mut other = vec_err.clone();
    println!("{:?} {:?} {}", other.as_bytes(), vec_err, vec_err == other);
    other = CString::from_vec_with_nul(b"ab".to_vec()).unwrap_err();
    println!("{} {:?}", vec_err == other, other.clone().into_bytes());
    let mut taken = vec_err.clone().into_bytes();
    taken.clear();
    println!("{taken:?} {:?}", vec_err.into_bytes());

    // Held, in a `Vec` and in a struct of the crate's that derives both.
    let errs = vec![
        CString::new("x\0").unwrap_err(),
        CString::new(vec![1u8, 0]).unwrap_err(),
    ];
    let copies = errs.clone();
    println!("{errs:?} {}", errs == copies);
    let failed = Failed {
        nul: CString::new("\0").unwrap_err(),
        until: CStr::from_bytes_until_nul(b"").unwrap_err(),
    };
    let kept = failed.clone();
    println!("{kept:?} {}", kept == failed);
}
