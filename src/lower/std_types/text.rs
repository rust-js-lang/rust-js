//! More of `char`'s and `str`'s methods, `parse`, and slicing by a range
//! (ADR 0063). A `char` is a one-character string (ADR 0034), so its
//! questions are regular expressions of the Unicode properties Rust uses:
//! `c.is_whitespace()` is `/^\p{White_Space}$/u.test(c)`.

use crate::js;
use crate::js::{Expr, Op, Prop, Stmt, StmtKind};
use crate::lower::calls::Call;
use crate::lower::recognition::{Std, trait_method};
use crate::lower::representation::Num;
use crate::lower::std_types::range::RangeKind;
use crate::lower::std_types::result;
use crate::lower::{FnCx, R};
use rustc_hir::attrs::lang_items::LangItem;
use rustc_middle::ty::{self, Ty};
use rustc_span::Span;

/// A `String` changed in place, its place given the new string, as JS
/// strings don't change (ADR 0149).
#[derive(Clone, Copy, PartialEq)]
pub(in crate::lower) enum StringEdit {
    /// `s.pop()`: the last `char`, or `None`.
    Pop,
    /// `s.remove(at)`: the `char` at a byte offset.
    Remove,
    Truncate,
    /// `s.insert(at, c)` and `s.insert_str(at, t)`.
    Insert,
    Retain,
    Clear,
    /// `s.split_off(at)`, `s.drain(range)`: what they take out (ADR 0323).
    SplitOff,
    Drain,
    ReplaceRange,
    ExtendFromWithin,
    /// A `char`'s `make_ascii_uppercase()` or `make_ascii_lowercase()` (ADR 0327).
    AsciiCase {
        upper: bool,
    },
    /// A `PathBuf`'s text changed (ADR 0405): `push(p)` and
    /// `set_file_name(n)`; `pop()` and `set_extension(e)`, whether it did.
    PathPush,
    PathPop,
    SetExtension,
    SetFileName,
}

#[derive(Clone, Copy, PartialEq)]
pub(in crate::lower) enum TextOp {
    /// A `char` question, as a regular expression it matches.
    Is(&'static str),
    IsAscii,
    ToAsciiUpper,
    ToAsciiLower,
    ToDigit,
    IsDigit,
    SplitWhitespace,
    /// `c.to_uppercase()`: its `char`s, as JS's own mapping gives them (`ß` is `SS`).
    CharCase(bool),
    Lines,
    /// `s.as_bytes()` and `s.bytes()`: its UTF-8 bytes, a copy, as nothing writes through it (ADR 0126).
    Bytes,
    /// `s.as_ptr()`: a pointer to the first of those bytes, where it's only
    /// read through, which a copy reads the same (ADR 0407).
    BytesPointer,
    /// `s.split(|c| ..)` and `s.contains(|c| ..)`: a closure as the pattern.
    SplitBy,
    /// `s.splitn(n, |c| ..)`: of a closure, its first pieces and the rest whole.
    SplitNBy,
    ContainsBy,
    /// `find(p)` or `rfind(p)`, `starts_with(p)` or `ends_with(p)`, of a
    /// predicate of a `char` (ADR 0157).
    FindBy(bool),
    StartsBy {
        end: bool,
    },
    SplitAsciiWhitespace,
    /// `s.get(range)`: `&s[range]`, or `None` where that would panic.
    StrGet,
    /// `&mut s[range]` of a string, `get_unchecked_mut(range)`, and
    /// `slice_mut_unchecked(a, b)` (`bounds`): that part of it, written
    /// in place (ADR 0334).
    StrPart {
        bounds: bool,
    },
    /// `s.get_mut(range)`: that part, or `None` where `&mut s[range]`
    /// would panic.
    StrGetMut,
    /// `s.split_at_mut(at)`, or `split_at_mut_checked` (`checked`): the
    /// parts before and after the byte `at`.
    StrSplitAtMut {
        checked: bool,
    },
    /// `s.slice_unchecked(a, b)`: `&s[a..b]`.
    StrSliceBounds,
    /// `v.get(range)` of a slice: `&v[range]`, or `None` where that would panic.
    SliceGet,
    /// `v.split_at(mid)` of a slice: `(&v[..mid], &v[mid..])`; or
    /// `split_at_checked` (`checked`), `None` past the end.
    SliceSplitAt {
        checked: bool,
    },
    /// `v.split_first()`, or `split_last()` (`last`): the item and a copy
    /// of the rest, or `None` of an empty slice.
    SliceSplitFirst {
        last: bool,
    },
    /// `char::from_u32_unchecked(n)`: the code point `n` is.
    CharFromCode,
    /// `str::from_utf8(bytes)`, or `String::from_utf8` (`owned`) (ADR 0172).
    FromUtf8 {
        owned: bool,
    },
    /// `str::from_utf8_unchecked(bytes)`: valid UTF-8's text.
    Utf8Unchecked,
    /// `str::from_utf8_mut(bytes)`, or `from_utf8_unchecked_mut`
    /// (`unchecked`): a `&mut str` read and written through the bytes
    /// (ADR 0334).
    FromUtf8Mut {
        unchecked: bool,
    },
    /// `String::from_utf8_lossy(bytes)`: a `Cow`.
    Utf8Lossy,
    /// A byte slice's `escape_ascii()`: its text (ADR 0341).
    EscapeAscii,
    /// A byte slice's `utf8_chunks()`: each run of valid text and the bad
    /// bytes after it, `{ valid, invalid }`.
    Utf8Chunks,
    /// A part of a `Utf8Error`, `FromUtf8Error` or `Cow<str>`, as the runtime
    /// makes it: `valid_up_to`, `error`, `bytes`, `_0`.
    Utf8Part(&'static str),
    /// `e.kind()` of a `ParseIntError`, which is its message: its kind.
    ParseErrorKind,
    /// `v.starts_with(prefix)`, or `ends_with` (`end`), of a slice whose items
    /// compare by value.
    SliceStartsWith {
        end: bool,
    },
    /// `a.eq_ignore_ascii_case(b)` of bytes.
    BytesAsciiEq,
    IsCharBoundary,
    /// A `char`'s `len_utf8()`, or `len_utf16()`.
    CharLen {
        utf16: bool,
    },
    /// `char::from(b)` of a `u8`: the `char` of that code point.
    CharFromByte,
    Parse,
    /// `&v[a..b]` of a slice, an array or a `Vec`.
    Slice,
    /// `&s[a..b]` of a string: by its UTF-8 bytes (ADR 0138).
    StrSlice,
    /// `s.len()`: its UTF-8 bytes.
    ByteLen,
    /// `s.find(p)`, or `s.rfind(p)` if `true`: where, in UTF-8 bytes.
    Find(bool),
    /// `s.char_indices()`: each `char`, with where it starts in UTF-8 bytes.
    CharIndices,
    /// `v.drain(a..b)`: those items, taken out of `v`.
    Drain,
    /// `v.splice(a..b, items)`: those items, taken out of `v`, `items` in
    /// their place (ADR 0315).
    Splice,
    /// `v.extend_from_within(a..b)`: a copy of those items, pushed (ADR 0315).
    ExtendFromWithin,
    /// `s.splitn(n, p)`: its first pieces, and the rest whole (ADR 0150).
    SplitN,
    /// `s.rsplit(p)`, or `s.rsplitn(n, p)` if `true`: searched from the end.
    Rsplit(bool),
    SplitTerminator,
    /// A `str`'s splits and searches from the end, its ASCII trims, and
    /// UTF-16 (ADR 0323).
    SplitInclusive,
    RsplitTerminator,
    RmatchIndices,
    Rmatches,
    StrSplitAtChecked,
    EncodeUtf16,
    FromUtf16 {
        lossy: bool,
    },
    CharBoundaryNear {
        ceil: bool,
    },
    TrimAscii {
        start: bool,
        end: bool,
    },
    /// A `char`'s or a `str`'s `escape_default()`, `escape_debug()` or
    /// `escape_unicode()`, as text; a `char`'s `encode_utf8(&mut buf)`, and
    /// `char::decode_utf16(units)` (ADR 0327).
    Escape {
        kind: &'static str,
        str: bool,
    },
    EncodeUtf8,
    DecodeUtf16,
    /// `s.split_at(at)`: by a UTF-8 byte offset.
    SplitAt,
    /// `s.match_indices(p)`: where, in UTF-8 bytes, and what.
    MatchIndices,
    Matches,
    /// `s.trim_matches(p)`, or its `start` or `end` only.
    TrimMatches {
        start: bool,
        end: bool,
    },
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// `s.trim()`, `trim_start()` or `trim_end()`, by Unicode's White_Space, as
    /// Rust's `char::is_whitespace` has it: `$trim(s)`, which trims U+0085
    /// and keeps U+FEFF, where JS's `trim()` does the other (ADR 0183).
    /// The text a string is extended by, `s.extend(items)`'s: its items',
    /// each a `char` or a string, joined, `items.join("")` of an array,
    /// `Array.from(items).join("")` of another iterable, a range's stepped.
    /// `ty` is the items', or none where they're any iterable already, a
    /// generic `Extend`'s dictionary's (ADR 0445).
    pub(in crate::lower) fn joined_text(&mut self, items: Expr, ty: Option<Ty<'tcx>>, span: Span) -> R<Expr> {
        let ty = ty.map(|ty| ty.peel_refs());
        let items = match ty {
            Some(ty) if self.range_kind(ty).is_some() => self.as_js_iterator(items, ty, span)?,
            _ => items,
        };
        let array = ty.is_some_and(|ty| ty.is_array() || ty.is_slice() || self.is_vec_like(ty));
        let items = match array {
            true => items,
            false => Expr::call(Expr::member(Expr::var("Array"), "from"), vec![items]),
        };
        Ok(Expr::call(Expr::member(items, "join"), vec![Expr::str("")]))
    }

    pub(in crate::lower) fn trimmed(&mut self, s: Expr, start: bool, end: bool) -> Expr {
        let name = match (start, end) {
            (true, true) => "$trim",
            (true, false) => "$trimStart",
            _ => "$trimEnd",
        };
        Expr::call(Expr::helper(name), vec![s])
    }

    /// `edit` of a string whose text is `current`, of what it's `given`:
    /// its new text, and what it took out, if it takes anything (ADR 0364).
    pub(in crate::lower) fn string_edit_values(
        &mut self,
        edit: StringEdit,
        current: Expr,
        given: Vec<Expr>,
        out: &mut Vec<Stmt>,
    ) -> (Expr, Option<Expr>) {
        let helper = |name: &'static str, mut list: Vec<Expr>| {
            list.insert(0, current.clone());
            Expr::call(Expr::helper(name), list)
        };
        match edit {
            StringEdit::Clear => (Expr::str(""), None),
            StringEdit::Truncate => (helper("$strTruncate", given), None),
            StringEdit::Insert => (helper("$insertStr", given), None),
            StringEdit::ReplaceRange => (helper("$replaceRange", given), None),
            StringEdit::ExtendFromWithin => (helper("$strExtendWithin", given), None),
            StringEdit::AsciiCase { upper } => (helper("$asciiCase", vec![Expr::bool(upper)]), None),
            StringEdit::PathPush => (helper("$pathJoin", given), None),
            StringEdit::SetFileName => (helper("$pathSetFileName", given), None),
            StringEdit::Retain => {
                let kept = Expr::call(
                    Expr::member(
                        Expr::call(Expr::member(Expr::var("Array"), "from"), vec![current.clone()]),
                        "filter",
                    ),
                    given,
                );
                (Expr::call(Expr::member(kept, "join"), vec![Expr::str("")]), None)
            }
            StringEdit::Pop
            | StringEdit::Remove
            | StringEdit::SplitOff
            | StringEdit::Drain
            | StringEdit::PathPop
            | StringEdit::SetExtension => {
                let (name, label) = match edit {
                    StringEdit::Pop => ("$strPop", "popped"),
                    StringEdit::SplitOff => ("$strSplitOff", "split"),
                    StringEdit::Drain => ("$strDrain", "drained"),
                    StringEdit::PathPop => ("$pathPop", "popped"),
                    StringEdit::SetExtension => ("$pathSetExtension", "set"),
                    _ => ("$strRemove", "removed"),
                };
                let pair = helper(name, given);
                let pair = self.spill(label, pair, out);
                (
                    Expr::index(pair.clone(), Expr::int(0)),
                    Some(Expr::index(pair, Expr::int(1))),
                )
            }
        }
    }

    /// A string's or a slice's method of its arguments' values, `tys`
    /// their types (ADR 0364).
    pub(in crate::lower) fn text_values(
        &mut self,
        op: TextOp,
        mut values: Vec<Expr>,
        tys: &[Ty<'tcx>],
        generic_args: ty::GenericArgsRef<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        // A set of `char`s as a pattern is the predicate of being one of
        // them, as a closure or a function is a predicate (ADR 0157).
        // Its pattern's argument, after `splitn`'s count.
        let at = if op == TextOp::SplitNBy { 2 } else { 1 };
        if matches!(
            op,
            TextOp::SplitBy
                | TextOp::SplitNBy
                | TextOp::ContainsBy
                | TextOp::FindBy(_)
                | TextOp::StartsBy { .. }
                | TextOp::TrimMatches { .. }
        ) && let Some(pattern) = tys.get(at)
            && let ty::Array(item, _) | ty::Slice(item) = pattern.peel_refs().kind()
            && item.is_char()
        {
            let set = values[at].clone();
            let one_of = Expr::call(Expr::member(set, "includes"), vec![Expr::var("c")]);
            values[at] = Expr::arrow(
                vec!["c".into()],
                vec![StmtKind::Return(Some(one_of)).at(js::Span::NONE)],
            );
        }
        let mut values = values.into_iter();
        let mut arg = || values.next().expect("rustc checked the arguments");
        let method = |object: Expr, name: &str, list: Vec<Expr>| Expr::call(Expr::member(object, name), list);
        let call = |name: &'static str, list: Vec<Expr>| Expr::call(Expr::helper(name), list);
        Ok(match op {
            TextOp::Is(regex) => method(Expr::regex(regex), "test", vec![arg()]),
            TextOp::IsAscii => {
                let code = method(arg(), "charCodeAt", vec![Expr::int(0)]);
                Expr::bin(Op::Lt, code, Expr::int(128))
            }
            TextOp::ToAsciiUpper | TextOp::ToAsciiLower => {
                let (letters, case) = match op {
                    TextOp::ToAsciiUpper => ("/[a-z]/", "toUpperCase"),
                    _ => ("/[A-Z]/", "toLowerCase"),
                };
                let letter = Expr::arrow(
                    vec!["letter".into()],
                    vec![
                        StmtKind::Return(Some(method(Expr::var("letter"), case, Vec::new()))).at(crate::js::Span::NONE),
                    ],
                );
                method(arg(), "replace", vec![Expr::regex(letters), letter])
            }
            TextOp::ToDigit | TextOp::IsDigit => {
                let digit = Expr::call(Expr::helper("$toDigit"), vec![arg(), arg()]);
                if op == TextOp::IsDigit {
                    Expr::bin(Op::Ne, digit, Expr::undefined())
                } else {
                    digit
                }
            }
            // Rust's whitespace is Unicode's `White_Space`: JS's `\s` less U+FEFF.
            TextOp::SplitWhitespace => {
                let words = method(arg(), "split", vec![Expr::regex("/\\p{White_Space}+/u")]);
                let word = Expr::arrow(
                    vec!["word".into()],
                    vec![
                        StmtKind::Return(Some(Expr::bin(Op::Ne, Expr::var("word"), Expr::str(""))))
                            .at(crate::js::Span::NONE),
                    ],
                );
                method(words, "filter", vec![word])
            }
            TextOp::CharCase(upper) => {
                let mapped = method(arg(), if upper { "toUpperCase" } else { "toLowerCase" }, Vec::new());
                Expr::call(Expr::member(Expr::var("Array"), "from"), vec![mapped])
            }
            TextOp::SplitBy => Expr::call(Expr::helper("$splitBy"), vec![arg(), arg()]),
            TextOp::SplitNBy => {
                let (s, n, matches) = (arg(), arg(), arg());
                Expr::call(Expr::helper("$splitBy"), vec![s, matches, n])
            }
            TextOp::ContainsBy => {
                let chars = Expr::call(Expr::member(Expr::var("Array"), "from"), vec![arg()]);
                method(chars, "some", vec![arg()])
            }
            TextOp::Lines => Expr::call(Expr::helper("$lines"), vec![arg()]),
            // `Array.from(new TextEncoder().encode(s))`: an array of `u8`s, as a
            // slice of them is.
            TextOp::Bytes | TextOp::BytesPointer => {
                let bytes = utf8_bytes(arg());
                match op {
                    TextOp::BytesPointer => call("$ptr", vec![bytes, Expr::int(0)]),
                    _ => bytes,
                }
            }
            TextOp::SplitN => call("$splitN", vec![arg(), arg(), arg()]),
            TextOp::Rsplit(limited) => {
                let mut list = vec![arg()];
                let limit = limited.then(&mut arg);
                list.push(arg());
                list.extend(limit);
                call("$rsplit", list)
            }
            TextOp::SplitTerminator => call("$splitTerminator", vec![arg(), arg()]),
            TextOp::SplitInclusive => call("$splitInclusive", vec![arg(), arg()]),
            TextOp::RsplitTerminator => call("$rsplitTerminator", vec![arg(), arg()]),
            TextOp::RmatchIndices => call("$rmatchIndices", vec![arg(), arg()]),
            TextOp::Rmatches => call("$rmatches", vec![arg(), arg()]),
            TextOp::StrSplitAtChecked => call("$splitAtChecked", vec![arg(), arg()]),
            TextOp::EncodeUtf16 => call("$encodeUtf16", vec![arg()]),
            TextOp::Escape { kind: "default", .. } => call("$escapeDefault", vec![arg()]),
            TextOp::Escape { kind: "unicode", .. } => call("$escapeUnicode", vec![arg()]),
            TextOp::Escape { str, .. } => call("$escapeDebug", vec![arg(), Expr::bool(str)]),
            // Its `&mut str`, a cell of the `char`'s text (ADR 0099).
            TextOp::EncodeUtf8 => {
                let written = call("$encodeUtf8", vec![arg(), arg()]);
                Expr::object(vec![Prop::Field(js::HELD.into(), written)])
            }
            TextOp::DecodeUtf16 => call("$decodeUtf16", vec![arg()]),
            TextOp::FromUtf16 { lossy: false } => call("$fromUtf16", vec![arg()]),
            TextOp::FromUtf16 { lossy: true } => call("$fromUtf16Lossy", vec![arg()]),
            TextOp::CharBoundaryNear { ceil: false } => call("$floorCharBoundary", vec![arg(), arg()]),
            TextOp::CharBoundaryNear { ceil: true } => call("$ceilCharBoundary", vec![arg(), arg()]),
            // ASCII whitespace, `u8::is_ascii_whitespace`'s, which isn't JS's.
            TextOp::TrimAscii { start, end } => {
                let regex = match (start, end) {
                    (true, true) => "/^[\\t\\n\\f\\r ]+|[\\t\\n\\f\\r ]+$/g",
                    (true, false) => "/^[\\t\\n\\f\\r ]+/",
                    _ => "/[\\t\\n\\f\\r ]+$/",
                };
                Expr::call(Expr::member(arg(), "replace"), vec![Expr::regex(regex), Expr::str("")])
            }
            TextOp::SplitAt => call("$splitAt", vec![arg(), arg()]),
            TextOp::StrSliceBounds => call("$strSlice", vec![arg(), arg(), arg()]),
            TextOp::StrPart { .. } | TextOp::StrGetMut | TextOp::StrSplitAtMut { .. } => {
                return Err(self.unsupported(span, "a part of a string, of its value alone"));
            }
            TextOp::SliceSplitAt { checked } => {
                let mut list = vec![arg(), arg()];
                if checked {
                    list.push(Expr::bool(true));
                }
                call("$sliceSplitAt", list)
            }
            // `v.length === 0 ? undefined : [v[0], v.slice(1)]`.
            TextOp::SliceSplitFirst { last } => {
                let items = arg();
                let items = if items.reads_same() {
                    items
                } else {
                    self.spill("items", items, out)
                };
                let length = Expr::member(items.clone(), "length");
                let empty = Expr::bin(Op::Eq, length.clone(), Expr::int(0));
                let (item, rest) = match last {
                    false => (Expr::int(0), vec![Expr::int(1)]),
                    true => (
                        Expr::bin(Op::Sub, length, Expr::int(1)),
                        vec![Expr::int(0), Expr::int(-1)],
                    ),
                };
                let split = Expr::array(vec![
                    Expr::index(items.clone(), item),
                    Expr::call(Expr::member(items, "slice"), rest),
                ]);
                Expr::cond(empty, Expr::undefined(), split)
            }
            TextOp::CharFromCode => Expr::call(Expr::member(Expr::var("String"), "fromCodePoint"), vec![arg()]),
            TextOp::FromUtf8 { owned } => {
                let mut list = vec![arg()];
                if owned {
                    list.push(Expr::bool(true));
                }
                call("$fromUtf8", list)
            }
            TextOp::Utf8Unchecked => call("$utf8Decode", vec![arg()]),
            TextOp::FromUtf8Mut { unchecked } => {
                let mut list = vec![arg()];
                if unchecked {
                    list.push(Expr::bool(true));
                }
                call("$fromUtf8Mut", list)
            }
            TextOp::Utf8Lossy => call("$utf8Lossy", vec![arg()]),
            TextOp::EscapeAscii => call("$escapeAscii", vec![arg()]),
            TextOp::Utf8Chunks => call("$utf8Chunks", vec![arg()]),
            TextOp::Utf8Part(name) => Expr::member(arg(), name),
            TextOp::ParseErrorKind => call("$parseErrorKind", vec![arg()]),
            TextOp::SliceStartsWith { end } => {
                let mut list = vec![arg(), arg()];
                let eq = self.item_eq((generic_args.type_at(0), generic_args.type_at(0)), span)?;
                if end || eq.is_some() {
                    list.push(Expr::bool(end));
                }
                list.extend(eq);
                call("$sliceStartsWith", list)
            }
            TextOp::BytesAsciiEq => call("$bytesAsciiEq", vec![arg(), arg()]),
            TextOp::MatchIndices => call("$matchIndices", vec![arg(), arg()]),
            TextOp::Matches => call("$matches", vec![arg(), arg()]),
            TextOp::TrimMatches { start, end } => {
                let mut list = vec![arg(), arg()];
                if !(start && end) {
                    list.extend([Expr::bool(start), Expr::bool(end)]);
                }
                call("$trimMatches", list)
            }
            TextOp::Parse => {
                let target = generic_args
                    .types()
                    .next()
                    .ok_or_else(|| self.unsupported(span, "this `parse`"))?;
                self.parse_as(arg(), target, span, out)?
            }
            TextOp::ByteLen => Expr::call(Expr::helper("$byteLen"), vec![arg()]),
            TextOp::FindBy(last) => call("$findBy", vec![arg(), arg(), Expr::bool(last)]),
            TextOp::StartsBy { end } => call("$startsBy", vec![arg(), arg(), Expr::bool(end)]),
            // Rust's ASCII whitespace: space, tab, line feed, form feed and
            // carriage return, not JS's `\s`.
            TextOp::SplitAsciiWhitespace => {
                let pieces = method(arg(), "split", vec![Expr::regex("/[\\t\\n\\f\\r ]+/")]);
                let word = Expr::bin(Op::Ne, Expr::var("word"), Expr::str(""));
                let nonempty = Expr::arrow(
                    vec!["word".into()],
                    vec![StmtKind::Return(Some(word)).at(js::Span::NONE)],
                );
                method(pieces, "filter", vec![nonempty])
            }
            TextOp::IsCharBoundary => {
                let unit = call("$charBoundary", vec![arg(), arg()]);
                Expr::bin(Op::Ne, unit, Expr::undefined())
            }
            TextOp::CharLen { utf16: false } => call("$byteLen", vec![arg()]),
            TextOp::CharLen { utf16: true } => Expr::member(arg(), "length"),
            TextOp::CharFromByte => Expr::call(Expr::member(Expr::var("String"), "fromCharCode"), vec![arg()]),
            TextOp::Find(last) => {
                let name = if last { "$rfind" } else { "$find" };
                Expr::call(Expr::helper(name), vec![arg(), arg()])
            }
            TextOp::CharIndices => Expr::call(Expr::helper("$charIndices"), vec![arg()]),
            TextOp::Slice
            | TextOp::StrSlice
            | TextOp::StrGet
            | TextOp::SliceGet
            | TextOp::Drain
            | TextOp::Splice
            | TextOp::ExtendFromWithin => {
                return Err(self.unsupported(span, "a range of a string or a slice, of its value alone"));
            }
        })
    }

    /// `s.parse::<T>()`: a `Result`, whose `Err` is the error's message,
    /// which is what its `to_string()` gives.
    pub(in crate::lower) fn parse_as(
        &mut self,
        text: Expr,
        target: Ty<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        if let Some(num) = Num::of(target) {
            if num == Num::F64 {
                return Ok(Expr::call(Expr::helper("$parseF64"), vec![text]));
            }
            // The digits' nearest `f32`, which through an `f64` alone can be
            // missed (ADR 0122).
            if num == Num::F32 {
                return Ok(Expr::call(Expr::helper("$parseF32"), vec![text]));
            }
            let (lo, hi) = num.range();
            // A 64-bit one is read as a BigInt, exactly (ADR 0086).
            let name = if num.big() { "$parseBig" } else { "$parseInt" };
            let parsed = Expr::call(Expr::helper(name), vec![text, num.literal(lo), num.literal(hi as i128)]);
            // A `NonZero`'s, an error of `0` (ADR 0177).
            if crate::lower::recognition::is_non_zero_ty(target) {
                let zero = Expr::str("number would be zero for non-zero type");
                return Ok(Expr::call(Expr::helper("$nonZeroOk"), vec![parsed, zero]));
            }
            return Ok(parsed);
        }
        if target.is_bool() {
            return Ok(Expr::call(Expr::helper("$parseBool"), vec![text]));
        }
        if target.is_char() {
            return Ok(Expr::call(Expr::helper("$parseChar"), vec![text]));
        }
        if self.is_lang_adt(target, LangItem::String) {
            return Ok(result::ok(text));
        }
        // A type's own `FromStr`: its `from_str`, called (ADR 0159), or a
        // `T`'s, its dictionary's (ADR 0161). Not std's of another type,
        // which has no dictionary but this.
        if let Some(from_str) = self.recognition().std_from_str()
            && (matches!(target.kind(), ty::Param(_)) || self.has_user_impl(from_str, target))
        {
            let method = trait_method(self.tcx, from_str, "from_str");
            let args = self.tcx.mk_args(&[target.into()]);
            if let Some(call) = self.trait_call(method, args, vec![text], span, out)? {
                return Ok(call);
            }
        }
        Err(self.unsupported(span, &format!("`parse` to a `{target}`")))
    }

    /// A `Vec`'s range, read as `range_read` reads it, `range`, replaced by
    /// `rest`, `$splice(v, a, b, items)`, or pushed again, `v.push(
    /// ...$copyRange(v, a, b))` (ADR 0315). Of all of it,
    /// `..`, `range` is the `Vec` itself.
    pub(in crate::lower) fn range_edit(&mut self, op: TextOp, range: Expr, rest: Vec<Expr>) -> Expr {
        let mut list = match range.kind {
            js::ExprKind::Call(_, list) => list,
            kind => vec![
                Expr {
                    kind,
                    span: range.span,
                    reads: false,
                },
                Expr::int(0),
            ],
        };
        let items = list[0].clone();
        // Checked as `slice::range` checks it, its end first, not as
        // `&v[a..b]` is: `v.push(...$copyRange(v, a, b))`.
        if op == TextOp::ExtendFromWithin {
            return Expr::call(
                Expr::member(items, "push"),
                vec![Expr::spread(Expr::call(Expr::helper("$copyRange"), list))],
            );
        }
        if list.len() == 2 {
            list.push(Expr::member(items, "length"));
        }
        list.extend(rest);
        Expr::call(Expr::helper("$splice"), list)
    }

    /// `items` in `range`, a `range_ty`, as `op` takes them, of their
    /// values (ADR 0364).
    pub(in crate::lower) fn slice_range_values(
        &mut self,
        op: TextOp,
        items: Expr,
        (range, range_ty): (Expr, Ty<'tcx>),
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let (start, end) = self.range_value_bounds(range, range_ty, span, out)?;
        Ok(self.sliced(op, items, start, end))
    }

    /// `items` from `start` to `end`, or to their end, as `op` takes them.
    fn sliced(&mut self, op: TextOp, items: Expr, start: Expr, end: Option<Expr>) -> Expr {
        let name = match op {
            TextOp::Drain => "$drain",
            TextOp::StrSlice => "$strSlice",
            TextOp::StrGet => "$strGet",
            TextOp::SliceGet => "$sliceGet",
            _ => "$slice",
        };
        // `&s[..]` of a string: all of it, which is never out of bounds, nor
        // inside a character.
        if matches!(op, TextOp::StrSlice | TextOp::StrGet) && start.as_int() == Some(0) && end.is_none() {
            return items;
        }
        // `&v[..]` of a slice: all of it, never out of bounds (ADR 0335).
        if name == "$slice" && start.as_int() == Some(0) && end.is_none() {
            return items;
        }
        let mut list = vec![items, start];
        list.extend(end);
        Expr::call_helper(name, list)
    }

    /// Of `cell`, a `&mut str`'s, and what's `given`, a range's bounds or a
    /// `split_at_mut`'s index: the part.
    pub(in crate::lower) fn str_part_values(&mut self, op: TextOp, cell: Expr, given: Vec<Expr>) -> Expr {
        let mut list = vec![cell];
        list.extend(given);
        let name = match op {
            TextOp::StrSplitAtMut { checked } => {
                if checked {
                    list.push(Expr::bool(true));
                }
                "$strSplitAtMut"
            }
            TextOp::StrGetMut => "$strGetMut",
            _ => "$strPart",
        };
        Expr::call(Expr::helper(name), list)
    }

    /// `range_bounds`'s, of a range's value, a `range_ty`.
    pub(in crate::lower) fn range_value_bounds(
        &mut self,
        value: Expr,
        range_ty: Ty<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<(Expr, Option<Expr>)> {
        let Some(kind) = self.range_kind(range_ty) else {
            return Err(self.unsupported(span, "slicing by this range"));
        };
        // `..=1` is `..2`.
        let past = |end: Expr| match end.as_int() {
            Some(n) => Expr::int(n + 1),
            None => Expr::bin(Op::Add, end, Expr::int(1)),
        };
        let parts = self.range_parts(value, kind, out);
        Ok(match (kind, parts.as_slice()) {
            (RangeKind::Exclusive, [start, end]) => (start.clone(), Some(end.clone())),
            (RangeKind::Inclusive, [start, end]) => (start.clone(), Some(past(end.clone()))),
            (RangeKind::From, [start]) => (start.clone(), None),
            (RangeKind::To, [end]) => (Expr::int(0), Some(end.clone())),
            (RangeKind::ToInclusive, [end]) => (Expr::int(0), Some(past(end.clone()))),
            _ => (Expr::int(0), None),
        })
    }
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// A string's method (ADRs 0034, 0063): `None` if `known` is another.
    pub(in crate::lower) fn string_call(
        &mut self,
        known: Std,
        call: Call<'_, 'tcx>,
        values: &mut std::vec::IntoIter<Expr>,
    ) -> R<Option<Expr>> {
        let Call {
            generic_args,
            tys,
            span,
            ..
        } = call;
        let mut arg = || values.next().expect("rustc checked the arguments");
        Ok(Some(match known {
            Std::Concat => Expr::bin(Op::Add, arg(), arg()),
            Std::Method(name) => {
                let this = arg();
                let mut rest: Vec<Expr> = (1..tys.len()).map(|_| arg()).collect();
                // `replacen`'s count: of one, the first, JS's `replace`; of
                // another, `$replacen`'s first that many.
                if name == "replace" {
                    if !matches!(rest[2].kind, js::ExprKind::Num(n) if n == 1.0) {
                        return Ok(Some(Expr::call(Expr::helper("$replacen"), [vec![this], rest].concat())));
                    }
                    rest.truncate(2);
                }
                // A pattern that may be empty, which Rust matches at each
                // char's boundary and JS between UTF-16 units (ADR 0063).
                let may_be_empty = matches!(name, "replaceAll" | "split")
                    && !generic_args.types().next().is_some_and(|p| p.is_char())
                    && !matches!(&rest[0].kind, js::ExprKind::Str(s) if !s.is_empty());
                if may_be_empty {
                    let helper = if name == "split" { "$split" } else { "$replace" };
                    return Ok(Some(Expr::call(Expr::helper(helper), [vec![this], rest].concat())));
                }
                if matches!(name, "replaceAll" | "replace") {
                    rest[1] = replacement(rest[1].clone());
                }
                Expr::read(this, name, rest)
            }
            Std::StripPrefix | Std::StripSuffix | Std::SplitOnce | Std::RsplitOnce => {
                let name = match known {
                    Std::StripPrefix => "$stripPrefix",
                    Std::StripSuffix => "$stripSuffix",
                    Std::SplitOnce => "$splitOnce",
                    _ => "$rsplitOnce",
                };
                Expr::call(Expr::helper(name), vec![arg(), arg()])
            }
            Std::StripCircumfix => Expr::call(Expr::helper("$stripCircumfix"), vec![arg(), arg(), arg()]),
            Std::Chars => Expr::call(Expr::member(Expr::var("Array"), "from"), vec![arg()]),
            Std::StringNew => Expr::str(""),
            Std::Trim { start, end } => self.trimmed(arg(), start, end),
            Std::AsciiCase { upper } => {
                let mut list = vec![arg()];
                if upper {
                    list.push(Expr::bool(true));
                }
                Expr::call(Expr::helper("$asciiCase"), list)
            }
            Std::AsciiEq => {
                let (a, b) = (arg(), arg());
                Expr::bin(
                    Op::Eq,
                    Expr::call(Expr::helper("$asciiCase"), vec![a]),
                    Expr::call(Expr::helper("$asciiCase"), vec![b]),
                )
            }
            Std::ToString => {
                let (value, ty) = (arg(), generic_args.type_at(0));
                // A generic `T: ToString`'s, through its dictionary, where it
                // has no `Display` bound to show it with (ADR 0049).
                let display = ty::TraitRef::new(self.tcx, self.display_trait(), [ty]);
                let to_string_trait = self
                    .to_string_trait()
                    .expect("a `to_string` call's crate has `ToString`");
                let to_string = ty::TraitRef::new(self.tcx, to_string_trait, [ty]);
                match self.is_unknown(ty) && !self.has_evidence(display) {
                    true if let Some(dictionary) = self.evidence_for(to_string) => {
                        Expr::call(Expr::member(dictionary, "to_string"), vec![value])
                    }
                    _ => self.display_string(value, ty, span)?,
                }
            }
            _ => return Ok(None),
        }))
    }
}

/// A replacement as the text it is, as Rust's is: JS reads `$&` and `$1` in
/// a string as what's matched, so a `$` written out is doubled, and one not
/// written out is a function's, `() => r`, whose text JS takes as it is
/// (ADR 0034).
fn replacement(to: Expr) -> Expr {
    match &to.kind {
        js::ExprKind::Str(text) => Expr::str(text.replace('$', "$$")),
        _ => Expr::arrow(Vec::new(), vec![js::StmtKind::Return(Some(to)).at(js::Span::NONE)]),
    }
}

/// What a `Vec`'s range is read as for `range_edit`: `splice`'s taken out,
/// as `drain`'s, `extend_from_within`'s copied, as `&v[a..b]`'s.
pub(in crate::lower) fn range_read(op: TextOp) -> TextOp {
    match op {
        TextOp::Splice => TextOp::Drain,
        _ => TextOp::Slice,
    }
}

/// Text's UTF-8 bytes, `Array.from(new TextEncoder().encode(s))`: an array
/// of `u8`s, as a slice of them is, `as_bytes()`'s and `AsRef<[u8]>`'s.
pub(in crate::lower) fn utf8_bytes(text: Expr) -> Expr {
    let encoder = Expr::new_(Expr::var("TextEncoder"), Vec::new());
    let encoded = Expr::call(Expr::member(encoder, "encode"), vec![text]);
    Expr::call(Expr::member(Expr::var("Array"), "from"), vec![encoded])
}
