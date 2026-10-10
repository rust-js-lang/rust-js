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
use crate::lower::{FnCx, R};
use crate::runtime::Helper;
use rustc_hir::attrs::lang_items::LangItem;
use rustc_middle::thir::{ExprId, ExprKind};
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
    /// `s.split(|c| ..)` and `s.contains(|c| ..)`: a closure as the pattern.
    SplitBy,
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
    pub(in crate::lower) fn trimmed(&mut self, s: Expr, start: bool, end: bool) -> Expr {
        self.runtime.insert(Helper::Trim);
        let name = match (start, end) {
            (true, true) => "$trim",
            (true, false) => "$trimStart",
            _ => "$trimEnd",
        };
        Expr::call(Expr::var(name), vec![s])
    }

    /// A `String` changed in place (ADR 0149): its place given the new
    /// string, `s = $insertStr(s, 0, "[")`, and what `pop` and `remove` take
    /// out, `popped[1]`. A JS string doesn't change; its place does.
    pub(in crate::lower) fn string_edit(
        &mut self,
        edit: StringEdit,
        args: &[ExprId],
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let ExprKind::Borrow { arg: place, .. } = self.thir[self.strip(args[0])].kind else {
            return Err(self.unsupported(span, "changing this string"));
        };
        // `s[a..b].make_ascii_uppercase()`: `s` given its new text, that
        // part changed, `s = $asciiCase(s, true, a, b)` (ADR 0334).
        if let StringEdit::AsciiCase { upper } = edit
            && let ExprKind::Deref { arg: part } = self.thir[self.strip(place)].kind
            && let ExprKind::Call {
                fun, args: ref given, ..
            } = self.thir[self.strip(part)].kind
            && let Some(Std::Text(TextOp::StrPart { bounds })) = self.std_fn(fun)
            && let Some(whole) = self.mut_borrowed(given[0])
            && !self.returned(whole)
        {
            let (target, _) = self.prepare_assignment_target(whole, true, Expr::undefined(), span, out)?;
            let (start, end) = match bounds {
                true => {
                    let [start, end]: [Expr; 2] = self.operands(&given[1..], out)?.try_into().ok().expect("two bounds");
                    (start, Some(end))
                }
                false => self.range_bounds(given[1], span, out)?,
            };
            let mut list = vec![target.read(), Expr::bool(upper)];
            if start.as_int() != Some(0) || end.is_some() {
                list.push(start);
                list.extend(end);
            }
            self.runtime.insert(Helper::AsciiCase);
            target.write(Expr::call(Expr::var("$asciiCase"), list), self.js_span(span), out);
            return Ok(Expr::undefined());
        }
        // A byte range's bounds, and `replace_range`'s text (ADR 0323).
        let given = match edit {
            StringEdit::Drain | StringEdit::ReplaceRange | StringEdit::ExtendFromWithin => {
                let (start, end) = self.range_bounds(args[1], span, out)?;
                let mut given = vec![start, end.unwrap_or_else(Expr::undefined)];
                given.extend(self.operands(&args[2..], out)?);
                given
            }
            _ => self.operands(&args[1..], out)?,
        };
        let (target, _) = self.prepare_assignment_target(place, true, Expr::undefined(), span, out)?;
        let current = target.read();
        let js_span = self.js_span(span);
        let helper = |cx: &mut Self, helper: Helper, name: &str, mut list: Vec<Expr>| {
            cx.runtime.insert(helper);
            list.insert(0, current.clone());
            Expr::call(Expr::var(name), list)
        };
        let (edited, taken) = match edit {
            StringEdit::Clear => (Expr::str(""), None),
            StringEdit::Truncate => (helper(self, Helper::StrTruncate, "$strTruncate", given), None),
            StringEdit::Insert => (helper(self, Helper::InsertStr, "$insertStr", given), None),
            StringEdit::ReplaceRange => (helper(self, Helper::StrEdits, "$replaceRange", given), None),
            StringEdit::ExtendFromWithin => (helper(self, Helper::StrEdits, "$strExtendWithin", given), None),
            StringEdit::AsciiCase { upper } => (
                helper(self, Helper::AsciiCase, "$asciiCase", vec![Expr::bool(upper)]),
                None,
            ),
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
            StringEdit::Pop | StringEdit::Remove | StringEdit::SplitOff | StringEdit::Drain => {
                let (helper_id, name, label) = match edit {
                    StringEdit::Pop => (Helper::StrPop, "$strPop", "popped"),
                    StringEdit::SplitOff => (Helper::StrEdits, "$strSplitOff", "split"),
                    StringEdit::Drain => (Helper::StrEdits, "$strDrain", "drained"),
                    _ => (Helper::StrRemove, "$strRemove", "removed"),
                };
                let pair = helper(self, helper_id, name, given);
                let pair = self.spill(label, pair, out);
                (
                    Expr::index(pair.clone(), Expr::int(0)),
                    Some(Expr::index(pair, Expr::int(1))),
                )
            }
        };
        target.write(edited, js_span, out);
        Ok(taken.unwrap_or_else(Expr::undefined))
    }

    pub(in crate::lower) fn text_call(
        &mut self,
        op: TextOp,
        args: &[ExprId],
        generic_args: ty::GenericArgsRef<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        if matches!(
            op,
            TextOp::Slice | TextOp::StrSlice | TextOp::StrGet | TextOp::SliceGet | TextOp::Drain
        ) {
            return self.slice_range(op, args, span, out);
        }
        // A range of a `Vec`'s, checked as `drain`'s and `&v[..]`'s are:
        // replaced, `$splice(v, a, b, items)`, or pushed again,
        // `v.push(...$slice(v, a, b))` (ADR 0315).
        if matches!(op, TextOp::Splice | TextOp::ExtendFromWithin) {
            let read = if op == TextOp::Splice {
                TextOp::Drain
            } else {
                TextOp::Slice
            };
            let range = self.slice_range(read, &args[..2], span, out)?;
            let js::ExprKind::Call(callee, mut list) = range.kind else {
                unreachable!("a range's read is a helper's call");
            };
            let items = list[0].clone();
            if op == TextOp::ExtendFromWithin {
                return Ok(Expr::call(
                    Expr::member(items, "push"),
                    vec![Expr::spread(Expr::call(*callee, list))],
                ));
            }
            if list.len() == 2 {
                list.push(Expr::member(items, "length"));
            }
            list.extend(self.operands(&args[2..], out)?);
            self.runtime.insert(Helper::Splice);
            return Ok(Expr::call(Expr::var("$splice"), list));
        }
        if matches!(
            op,
            TextOp::StrPart { .. } | TextOp::StrGetMut | TextOp::StrSplitAtMut { .. }
        ) {
            return self.str_part(op, args, span, out);
        }
        let mut values = self.operands(args, out)?;
        // A set of `char`s as a pattern is the predicate of being one of
        // them, as a closure or a function is a predicate (ADR 0157).
        if matches!(
            op,
            TextOp::SplitBy
                | TextOp::ContainsBy
                | TextOp::FindBy(_)
                | TextOp::StartsBy { .. }
                | TextOp::TrimMatches { .. }
        ) && let Some(&pattern) = args.get(1)
            && let ty::Array(item, _) | ty::Slice(item) = self.thir[pattern].ty.peel_refs().kind()
            && item.is_char()
        {
            let set = values[1].clone();
            let one_of = Expr::call(Expr::member(set, "includes"), vec![Expr::var("c")]);
            values[1] = Expr::arrow(
                vec!["c".into()],
                vec![StmtKind::Return(Some(one_of)).at(js::Span::NONE)],
            );
        }
        let mut values = values.into_iter();
        let mut arg = || values.next().expect("rustc checked the arguments");
        let method = |object: Expr, name: &str, list: Vec<Expr>| Expr::call(Expr::member(object, name), list);
        let call = |cx: &mut Self, helper: Helper, name: &str, list: Vec<Expr>| {
            cx.runtime.insert(helper);
            Expr::call(Expr::var(name), list)
        };
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
                self.runtime.insert(Helper::ToDigit);
                let digit = Expr::call(Expr::var("$toDigit"), vec![arg(), arg()]);
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
            TextOp::SplitBy => {
                self.runtime.insert(Helper::SplitBy);
                Expr::call(Expr::var("$splitBy"), vec![arg(), arg()])
            }
            TextOp::ContainsBy => {
                let chars = Expr::call(Expr::member(Expr::var("Array"), "from"), vec![arg()]);
                method(chars, "some", vec![arg()])
            }
            TextOp::Lines => {
                self.runtime.insert(Helper::Lines);
                Expr::call(Expr::var("$lines"), vec![arg()])
            }
            // `Array.from(new TextEncoder().encode(s))`: an array of `u8`s, as a
            // slice of them is.
            TextOp::Bytes => {
                let encoder = Expr::new_(Expr::var("TextEncoder"), Vec::new());
                let encoded = Expr::call(Expr::member(encoder, "encode"), vec![arg()]);
                Expr::call(Expr::member(Expr::var("Array"), "from"), vec![encoded])
            }
            TextOp::SplitN => call(self, Helper::SplitN, "$splitN", vec![arg(), arg(), arg()]),
            TextOp::Rsplit(limited) => {
                let mut list = vec![arg()];
                let limit = limited.then(&mut arg);
                list.push(arg());
                list.extend(limit);
                call(self, Helper::Rsplit, "$rsplit", list)
            }
            TextOp::SplitTerminator => call(self, Helper::SplitTerminator, "$splitTerminator", vec![arg(), arg()]),
            TextOp::SplitInclusive => call(self, Helper::StrSearch, "$splitInclusive", vec![arg(), arg()]),
            TextOp::RsplitTerminator => call(self, Helper::StrSearch, "$rsplitTerminator", vec![arg(), arg()]),
            TextOp::RmatchIndices => call(self, Helper::StrSearch, "$rmatchIndices", vec![arg(), arg()]),
            TextOp::Rmatches => call(self, Helper::StrSearch, "$rmatches", vec![arg(), arg()]),
            TextOp::StrSplitAtChecked => call(self, Helper::StrSearch, "$splitAtChecked", vec![arg(), arg()]),
            TextOp::EncodeUtf16 => call(self, Helper::StrSearch, "$encodeUtf16", vec![arg()]),
            TextOp::Escape { kind: "default", .. } => call(self, Helper::CharEscape, "$escapeDefault", vec![arg()]),
            TextOp::Escape { kind: "unicode", .. } => call(self, Helper::CharEscape, "$escapeUnicode", vec![arg()]),
            TextOp::Escape { str, .. } => call(self, Helper::CharEscape, "$escapeDebug", vec![arg(), Expr::bool(str)]),
            // Its `&mut str`, a cell of the `char`'s text (ADR 0099).
            TextOp::EncodeUtf8 => {
                let written = call(self, Helper::CharEscape, "$encodeUtf8", vec![arg(), arg()]);
                Expr::object(vec![Prop::Field("value".into(), written)])
            }
            TextOp::DecodeUtf16 => call(self, Helper::CharEscape, "$decodeUtf16", vec![arg()]),
            TextOp::FromUtf16 { lossy: false } => call(self, Helper::StrSearch, "$fromUtf16", vec![arg()]),
            TextOp::FromUtf16 { lossy: true } => call(self, Helper::StrSearch, "$fromUtf16Lossy", vec![arg()]),
            TextOp::CharBoundaryNear { ceil: false } => {
                call(self, Helper::StrSearch, "$floorCharBoundary", vec![arg(), arg()])
            }
            TextOp::CharBoundaryNear { ceil: true } => {
                call(self, Helper::StrSearch, "$ceilCharBoundary", vec![arg(), arg()])
            }
            // ASCII whitespace, `u8::is_ascii_whitespace`'s, which isn't JS's.
            TextOp::TrimAscii { start, end } => {
                let regex = match (start, end) {
                    (true, true) => "/^[\\t\\n\\f\\r ]+|[\\t\\n\\f\\r ]+$/g",
                    (true, false) => "/^[\\t\\n\\f\\r ]+/",
                    _ => "/[\\t\\n\\f\\r ]+$/",
                };
                Expr::call(Expr::member(arg(), "replace"), vec![Expr::regex(regex), Expr::str("")])
            }
            TextOp::SplitAt => call(self, Helper::SplitAt, "$splitAt", vec![arg(), arg()]),
            TextOp::StrSliceBounds => call(self, Helper::StrSlice, "$strSlice", vec![arg(), arg(), arg()]),
            TextOp::StrPart { .. } | TextOp::StrGetMut | TextOp::StrSplitAtMut { .. } => {
                unreachable!("a part of a string is made above")
            }
            TextOp::SliceSplitAt { checked } => {
                let mut list = vec![arg(), arg()];
                if checked {
                    list.push(Expr::bool(true));
                }
                call(self, Helper::SliceSplitAt, "$sliceSplitAt", list)
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
                call(self, Helper::Utf8, "$fromUtf8", list)
            }
            TextOp::Utf8Unchecked => call(self, Helper::Utf8, "$utf8Decode", vec![arg()]),
            TextOp::FromUtf8Mut { unchecked } => {
                let mut list = vec![arg()];
                if unchecked {
                    list.push(Expr::bool(true));
                }
                call(self, Helper::Utf8, "$fromUtf8Mut", list)
            }
            TextOp::Utf8Lossy => call(self, Helper::Utf8, "$utf8Lossy", vec![arg()]),
            TextOp::Utf8Part(name) => Expr::member(arg(), name),
            TextOp::ParseErrorKind => call(self, Helper::DebugParseError, "$parseErrorKind", vec![arg()]),
            TextOp::SliceStartsWith { end } => {
                let mut list = vec![arg(), arg()];
                if end {
                    list.push(Expr::bool(true));
                }
                call(self, Helper::SliceStartsWith, "$sliceStartsWith", list)
            }
            TextOp::BytesAsciiEq => call(self, Helper::BytesAsciiEq, "$bytesAsciiEq", vec![arg(), arg()]),
            TextOp::MatchIndices => call(self, Helper::MatchIndices, "$matchIndices", vec![arg(), arg()]),
            TextOp::Matches => call(self, Helper::Matches, "$matches", vec![arg(), arg()]),
            TextOp::TrimMatches { start, end } => {
                let mut list = vec![arg(), arg()];
                if !(start && end) {
                    list.extend([Expr::bool(start), Expr::bool(end)]);
                }
                call(self, Helper::TrimMatches, "$trimMatches", list)
            }
            TextOp::Parse => {
                let target = generic_args
                    .types()
                    .next()
                    .ok_or_else(|| self.unsupported(span, "this `parse`"))?;
                self.parse_as(arg(), target, span, out)?
            }
            TextOp::ByteLen => {
                self.runtime.insert(Helper::ByteLen);
                Expr::call(Expr::var("$byteLen"), vec![arg()])
            }
            TextOp::FindBy(last) => call(self, Helper::FindBy, "$findBy", vec![arg(), arg(), Expr::bool(last)]),
            TextOp::StartsBy { end } => call(self, Helper::StartsBy, "$startsBy", vec![arg(), arg(), Expr::bool(end)]),
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
                let unit = call(self, Helper::CharBoundary, "$charBoundary", vec![arg(), arg()]);
                Expr::bin(Op::Ne, unit, Expr::undefined())
            }
            TextOp::CharLen { utf16: false } => call(self, Helper::ByteLen, "$byteLen", vec![arg()]),
            TextOp::CharLen { utf16: true } => Expr::member(arg(), "length"),
            TextOp::CharFromByte => Expr::call(Expr::member(Expr::var("String"), "fromCharCode"), vec![arg()]),
            TextOp::Find(last) => {
                let (helper, name) = if last {
                    (Helper::Rfind, "$rfind")
                } else {
                    (Helper::Find, "$find")
                };
                self.runtime.insert(helper);
                Expr::call(Expr::var(name), vec![arg(), arg()])
            }
            TextOp::CharIndices => {
                self.runtime.insert(Helper::CharIndices);
                Expr::call(Expr::var("$charIndices"), vec![arg()])
            }
            TextOp::Slice
            | TextOp::StrSlice
            | TextOp::StrGet
            | TextOp::SliceGet
            | TextOp::Drain
            | TextOp::Splice
            | TextOp::ExtendFromWithin => {
                unreachable!("handled above")
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
                self.runtime.insert(Helper::ParseF64);
                return Ok(Expr::call(Expr::var("$parseF64"), vec![text]));
            }
            // The digits' nearest `f32`, which through an `f64` alone can be
            // missed (ADR 0122).
            if num == Num::F32 {
                self.runtime.insert(Helper::ParseF32);
                return Ok(Expr::call(Expr::var("$parseF32"), vec![text]));
            }
            let (lo, hi) = num.range();
            // A 64-bit one is read as a BigInt, exactly (ADR 0086).
            let (helper, name) = if num.big() {
                (Helper::ParseBig, "$parseBig")
            } else {
                (Helper::ParseInt, "$parseInt")
            };
            self.runtime.insert(helper);
            let parsed = Expr::call(Expr::var(name), vec![text, num.literal(lo), num.literal(hi as i128)]);
            // A `NonZero`'s, an error of `0` (ADR 0177).
            if crate::lower::recognition::is_non_zero_ty(target) {
                self.runtime.insert(Helper::NonZeroOk);
                let zero = Expr::str("number would be zero for non-zero type");
                return Ok(Expr::call(Expr::var("$nonZeroOk"), vec![parsed, zero]));
            }
            return Ok(parsed);
        }
        if target.is_bool() {
            self.runtime.insert(Helper::ParseBool);
            return Ok(Expr::call(Expr::var("$parseBool"), vec![text]));
        }
        if target.is_char() {
            self.runtime.insert(Helper::ParseChar);
            return Ok(Expr::call(Expr::var("$parseChar"), vec![text]));
        }
        if self.is_lang_adt(target, LangItem::String) {
            let ok = Expr::object(vec![
                Prop::Field("TAG".into(), Expr::str("Ok")),
                Prop::Field("_0".into(), text),
            ]);
            return Ok(ok);
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

    /// `&v[a..b]`, `&v[a..]`, `&v[..b]`, `&v[..]`: a copy, which a shared
    /// slice can be, since nothing changes `v` while it's borrowed. Out of
    /// bounds, it panics, as Rust does.
    fn slice_range(&mut self, op: TextOp, args: &[ExprId], span: Span, out: &mut Vec<Stmt>) -> R<Expr> {
        let (helper, name) = match op {
            TextOp::Drain => (Helper::Drain, "$drain"),
            TextOp::StrSlice => (Helper::StrSlice, "$strSlice"),
            TextOp::StrGet => (Helper::StrGet, "$strGet"),
            TextOp::SliceGet => (Helper::SliceGet, "$sliceGet"),
            _ => (Helper::SliceRange, "$slice"),
        };
        let items = self.operands(&[args[0]], out)?.remove(0);
        let (start, end) = self.range_bounds(args[1], span, out)?;
        // `&s[..]` of a string: all of it, which is never out of bounds, nor
        // inside a character.
        if matches!(op, TextOp::StrSlice | TextOp::StrGet) && start.as_int() == Some(0) && end.is_none() {
            return Ok(items);
        }
        self.runtime.insert(helper);
        let mut list = vec![items, start];
        list.extend(end);
        Ok(Expr::call(Expr::var(name), list))
    }

    /// A part of a string as a `&mut str` (ADR 0334): `$strPart(cell, a, b)`
    /// of the cell its `&mut` is, `$strGetMut` and `$strSplitAtMut`.
    fn str_part(&mut self, op: TextOp, args: &[ExprId], span: Span, out: &mut Vec<Stmt>) -> R<Expr> {
        let mut list = vec![self.str_cell(args[0], span, out)?];
        let name = match op {
            TextOp::StrSplitAtMut { checked } => {
                list.extend(self.operands(&args[1..], out)?);
                if checked {
                    list.push(Expr::bool(true));
                }
                "$strSplitAtMut"
            }
            TextOp::StrPart { bounds: true } => {
                list.extend(self.operands(&args[1..], out)?);
                "$strPart"
            }
            _ => {
                let (start, end) = self.range_bounds(args[1], span, out)?;
                list.push(start);
                list.extend(end);
                match op {
                    TextOp::StrGetMut => "$strGetMut",
                    _ => "$strPart",
                }
            }
        };
        self.runtime.insert(Helper::StrPart);
        Ok(Expr::call(Expr::var(name), list))
    }

    /// What a `&mut str` is written through (ADR 0334): a box or a part it
    /// already is, or a handle on the place it borrows, `s` of `&mut s`.
    fn str_cell(&mut self, arg: ExprId, span: Span, out: &mut Vec<Stmt>) -> R<Expr> {
        let Some(place) = self.mut_borrowed(arg) else {
            return self.expr(arg, out);
        };
        if let ExprKind::Deref { arg: inner } = self.thir[self.strip(place)].kind
            && self.is_cell_value(inner)
        {
            return self.expr(inner, out);
        }
        Ok(Expr::handle(self.fixed_place(place, "cell", span, out)?))
    }

    /// Is `e` a call making a part of a string, `&mut s[a..b]`?
    pub(in crate::lower) fn is_str_part(&self, e: ExprId) -> bool {
        matches!(self.thir[self.strip(e)].kind, ExprKind::Call { fun, .. }
            if matches!(self.std_fn(fun), Some(Std::Text(TextOp::StrPart { .. }))))
    }

    /// A range argument's bounds, in order: its start, `0` where it has none,
    /// and its end past what it holds, `a..=b`'s `b + 1`, or `None` where it
    /// runs to the end.
    pub(in crate::lower) fn range_bounds(
        &mut self,
        range: ExprId,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<(Expr, Option<Expr>)> {
        let range = self.strip(range);
        let range_ty = self.thir[range].ty;
        let kind = self.range_kind(range_ty);
        // `..=1` is `..2`.
        let past = |end: Expr| match end.as_int() {
            Some(n) => Expr::int(n + 1),
            None => Expr::bin(Op::Add, end, Expr::int(1)),
        };
        let fields: Vec<(usize, ExprId)> = match self.thir[range].kind {
            ExprKind::Adt(ref adt) if kind != Some(RangeKind::ToInclusive) => {
                adt.fields.iter().map(|f| (f.name.as_usize(), f.expr)).collect()
            }
            // A range kept as a value, or `a..=b` or `..=b` (ADR 0129): its bounds.
            _ => {
                let Some(kind) = kind else {
                    return Err(self.unsupported(span, "slicing by this range"));
                };
                let value = self.operands(&[range], out)?.remove(0);
                let parts = self.range_parts(value, kind, out);
                return Ok(match (kind, parts.as_slice()) {
                    (RangeKind::Exclusive, [start, end]) => (start.clone(), Some(end.clone())),
                    (RangeKind::Inclusive, [start, end]) => (start.clone(), Some(past(end.clone()))),
                    (RangeKind::From, [start]) => (start.clone(), None),
                    (RangeKind::To, [end]) => (Expr::int(0), Some(end.clone())),
                    (RangeKind::ToInclusive, [end]) => (Expr::int(0), Some(past(end.clone()))),
                    _ => (Expr::int(0), None),
                });
            }
        };
        let bound = |i: usize| fields.iter().find(|&&(n, _)| n == i).map(|&(_, e)| e);
        let (start, end) = if self.is_lang_adt(range_ty, LangItem::Range) {
            (bound(0), bound(1))
        } else if self.is_lang_adt(range_ty, LangItem::RangeFrom) {
            (bound(0), None)
        } else if self.is_lang_adt(range_ty, LangItem::RangeTo) {
            (None, bound(0))
        } else if self.is_lang_adt(range_ty, LangItem::RangeFull) {
            (None, None)
        } else {
            return Err(self.unsupported(span, "slicing by this range"));
        };
        let list: Vec<ExprId> = start.into_iter().chain(end).collect();
        let mut values = self.operands(&list, out)?.into_iter();
        let start = match start {
            Some(_) => values.next().expect("a start"),
            None => Expr::int(0),
        };
        Ok((start, end.map(|_| values.next().expect("an end"))))
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
            args,
            span,
            ..
        } = call;
        let mut arg = || values.next().expect("rustc checked the arguments");
        Ok(Some(match known {
            Std::Concat => Expr::bin(Op::Add, arg(), arg()),
            Std::Method(name) => {
                let this = arg();
                let mut rest: Vec<Expr> = (1..args.len()).map(|_| arg()).collect();
                // `replacen`'s count: of one, the first, JS's `replace`.
                if name == "replace" {
                    if !matches!(rest[2].kind, js::ExprKind::Num(n) if n == 1.0) {
                        return Err(self.tcx.dcx().span_err(
                            span,
                            "rust-js does not support `replacen` of a count other than 1 yet: `replace` replaces each, `replacen(p, r, 1)` the first",
                        ));
                    }
                    rest.truncate(2);
                }
                // A pattern that may be empty, which Rust matches at each
                // char's boundary and JS between UTF-16 units (ADR 0063).
                let may_be_empty = matches!(name, "replaceAll" | "split")
                    && !generic_args.types().next().is_some_and(|p| p.is_char())
                    && !matches!(&rest[0].kind, js::ExprKind::Str(s) if !s.is_empty());
                if may_be_empty {
                    self.runtime.insert(Helper::EmptyPattern);
                    let helper = if name == "split" { "$split" } else { "$replace" };
                    return Ok(Some(Expr::call(Expr::var(helper), [vec![this], rest].concat())));
                }
                if matches!(name, "replaceAll" | "replace") {
                    rest[1] = replacement(rest[1].clone());
                }
                Expr::call(Expr::member(this, name), rest)
            }
            Std::StripPrefix | Std::StripSuffix | Std::SplitOnce | Std::RsplitOnce => {
                let (helper, name) = match known {
                    Std::StripPrefix => (Helper::StripPrefix, "$stripPrefix"),
                    Std::StripSuffix => (Helper::StripSuffix, "$stripSuffix"),
                    Std::SplitOnce => (Helper::SplitOnce, "$splitOnce"),
                    _ => (Helper::RsplitOnce, "$rsplitOnce"),
                };
                self.runtime.insert(helper);
                Expr::call(Expr::var(name), vec![arg(), arg()])
            }
            Std::StripCircumfix => {
                self.runtime.insert(Helper::StripCircumfix);
                Expr::call(Expr::var("$stripCircumfix"), vec![arg(), arg(), arg()])
            }
            Std::Chars => Expr::call(Expr::member(Expr::var("Array"), "from"), vec![arg()]),
            Std::StringNew => Expr::str(""),
            Std::Trim { start, end } => self.trimmed(arg(), start, end),
            Std::AsciiCase { upper } => {
                self.runtime.insert(Helper::AsciiCase);
                let mut list = vec![arg()];
                if upper {
                    list.push(Expr::bool(true));
                }
                Expr::call(Expr::var("$asciiCase"), list)
            }
            Std::AsciiEq => {
                self.runtime.insert(Helper::AsciiCase);
                let (a, b) = (arg(), arg());
                Expr::bin(
                    Op::Eq,
                    Expr::call(Expr::var("$asciiCase"), vec![a]),
                    Expr::call(Expr::var("$asciiCase"), vec![b]),
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
