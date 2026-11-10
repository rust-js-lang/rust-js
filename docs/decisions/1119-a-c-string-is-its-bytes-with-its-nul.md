# 1119. A C string is its bytes with its nul

Status: Accepted (2026-11-11, #1157, the user's decision of 2026-11-10).
Amended the same day: hashed through a `Hasher`, and its errors cloned and
compared.

Case ([0262](0262-when-rust-and-js-disagree.md)): none observable. Each
constructor, accessor, error and comparison gives what native Rust gives,
held against it by the corpus.

## Context

`CStr`, `CString` and `c".."` literals had no JS form. They were refused:
"this constant, from its MIR" for `c"hi"`, and "calling
`std::ffi::CStr::to_bytes`" for the rest. chrono's `from_bytes_until_nul`
makes a `&CStr` of a byte slice and reads `to_bytes()` (#1153). std's bodies
for these types read memory by where it is. `CStr` is
`#[repr(transparent)] struct CStr { inner: [c_char] }`, and its `to_bytes`
casts that slice's pointer to `*const [u8]`. `CString` is `{ inner: Box<[u8]> }`.

## Decision

**A `CStr` is its bytes with its nul, as a `[u8]` is, a JS array of
numbers; a `CString` is its `Vec<u8>` with the nul.** `c"hi"` is
`[104, 105, 0]`.

- **To the rest of the compiler, each is its bytes**
  (`Recognition::transparent`, `std_wrapped`). So what std derives of
  them works on the array: `==`, `Clone`, a set's and a map's key, and a
  B-tree's order, which is unsigned, a byte past `0x7f` after ASCII.
- **Their identity is recognition's** (`recognition/c_strs.rs`), and so
  is the catalogue of their functions. What each function is in JS is
  `std_types/c_str.rs`'s, with its runtime helpers in `c_str.js`.
- **The accessors**:
  - `to_bytes()` and `as_bytes()` give a part without the nul, as
    `&bytes[..n]` is (`$cstrBytes`).
  - `to_bytes_with_nul()`, `as_c_str()`, `Deref`, `AsRef<CStr>` and
    `Borrow<CStr>` give the array itself.
  - `count_bytes()` is `c.length - 1`, and `is_empty()` is `c.length === 1`.
  - `to_str()` is `str::from_utf8`'s, its `Utf8Error` std's.
    `to_string_lossy()` is `String::from_utf8_lossy`'s.
- **The constructors**:
  - `from_bytes_with_nul`, `from_bytes_until_nul` and `from_vec_with_nul`
    check as std's do.
  - `CString::new(t)` takes `t`'s bytes as std's `Into<Vec<u8>>` lowers
    them, then checks for an interior nul.
  - The `_unchecked` ones give what they're given, or it with a nul.
  - `to_owned()` and `CString::from(c)` copy the bytes.
  - `into_bytes()` and `Vec::from(s)` drop the nul, without changing in
    place an array that another may hold too.
- **The errors**:
  - `FromBytesWithNulError` is an enum, as std's enums are.
  - `FromBytesUntilNulError(())` and `NulError(position, bytes)` are tuple
    structs' arrays.
  - `IntoStringError` is `{ inner, error }`, and `FromVecWithNulError` is
    `{ errorKind, bytes }`.
  - Each has std's derived `Debug`, pretty as asked, and std's `Display`.
- **`{:?}`** is std's `ByteStr`'s, of the bytes without the nul:
  - each ASCII character escaped as `escape_ascii` does it;
  - another character escaped as `escape_debug` does it, so a combining mark
    shows as `\u{301}`;
  - each byte that isn't UTF-8 shown as `\xNN`.

Refused at compile time, as reading memory: a C string's pointer,
`as_ptr`, `from_ptr`, `into_raw` and `from_raw` (#1291).

## Alternatives

- **A JS string.** It can't hold bytes that aren't UTF-8, so it would answer
  wrongly for them. Rejected in the issue.
- **std's bodies for the accessors.** They cast `[c_char]` to `[u8]`
  through raw pointers. `c_char` is `i8` on wasm32, so the bytes would be
  read with their sign.

## Consequences

The corpus case `c_strings.rs`, both builds and through `METAMORPHIC=all`,
holds each constructor, accessor and error against native Rust, including
text that isn't UTF-8, an interior nul, a missing nul, and keys of a hash
map, a hash set and a B-tree. `std_types/map.rs`'s key questions moved to
`std_types/map_keys.rs` first, as their own change.

## Amendment: hashed, and its errors cloned and compared

The first change refused hashing a C string through a `Hasher` and cloning
one of its errors, as the files they change, `recognition/types.rs` and
`std_impls.rs`, were at the 1,500-line cap. Each was split by concept
first, in a commit of its own (`recognition/type_facts.rs` and
`recognition/js_items.rs`; `std_impls/clones.rs`).

- **Hashed**, a C string writes what std's derived `Hash` writes:
  `[c_char]`'s for a `CStr`, and `Box<[u8]>`'s for a `CString`. That is
  its length, then its bytes with the nul in one write, the same bytes
  whether `c_char` is signed or not. So `hashed_as` answers
  `Hashed::Slice(u8)`, through std's `DefaultHasher`, a derived `Hash`,
  generic code's `T: Hash`, and a `BuildHasher`'s `hash_one`.
- **Its errors' `Clone` and `PartialEq`** are std's derived ones, their
  fields'. Each error is made in its fields' shape, and so is the kind
  `FromVecWithNulError` holds, alloc's private `FromBytesWithNulErrorKind`.
  So they are `std_impls.rs`'s `is_known_std`, as `Option` and `Result`
  are: a clone copies what may change, the bytes a `NulError` holds, and
  `==` is `$eq`'s.

Supporting them found #1290: a clone of a `CString`, as of a `Box<[T]>`,
was the same array where `into_bytes_with_nul()` made a `Vec` of it that
changed in place. ADR 0530's amendment fixes that.

`c_strings_hashed_and_their_errors_cloned`, built for wasm32 natively as
std's hash of a `usize` is a 32-bit target's, holds each against native
Rust, both builds and through `METAMORPHIC=all`.
