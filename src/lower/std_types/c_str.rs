//! C strings (#1157, the user's decision): a `CStr` is its bytes with its
//! nul, as a `[u8]` is, a JS array of numbers, `c"hi"` `[104, 105, 0]`; a
//! `CString` is its `Vec<u8>` with the nul. Each is its bytes to the rest
//! of the compiler (`Recognition::transparent`), so what std derives of
//! them, `==`, `Hash`, `Clone`, is of the array; what std's bodies read
//! memory by where it is for is here, its runtime helpers `c_str.js`'s.

use crate::js::{self, Expr, Stmt};
use crate::lower::calls::Call;
use crate::lower::recognition::Std;
use crate::lower::{FnCx, R};

/// What's asked of a `CStr` or a `CString`.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(in crate::lower) enum CStrOp {
    /// `CStr::from_bytes_with_nul(bytes)`: `Ok` of the bytes, or `Err` of a
    /// `FromBytesWithNulError`.
    WithNul,
    /// `CStr::from_bytes_until_nul(bytes)`: `Ok` of the bytes to the first
    /// nul, or `Err` of a `FromBytesUntilNulError`.
    UntilNul,
    /// `c.to_bytes()`, `s.as_bytes()`: the bytes without the nul, a part.
    Bytes,
    /// `c.count_bytes()`: how many bytes before the nul.
    Count,
    /// `c.is_empty()`: whether there are none before the nul.
    IsEmpty,
    /// `c.to_str()`: `str::from_utf8` of its bytes.
    ToStr,
    /// `c.to_string_lossy()`: `String::from_utf8_lossy` of its bytes.
    Lossy,
    /// `CString::new(t)`: `t`'s bytes as `Into<Vec<u8>>` gives them, `Ok`
    /// of them with a nul, or `Err` of a `NulError`.
    New,
    /// `CString::from_vec_unchecked(v)`: `v` with a nul.
    FromVec,
    /// `CString::from_vec_with_nul(v)`: `Ok` of `v`, which ends in its one
    /// nul, or `Err` of a `FromVecWithNulError`.
    FromVecWithNul,
    /// `s.into_bytes()`, `Vec::from(s)`: its bytes without the nul.
    IntoBytes,
    /// `s.into_string()`: `Ok` of its text, or `Err` of an
    /// `IntoStringError`.
    IntoString,
    /// `c.to_owned()`, `CString::from(c)`: a copy of its bytes.
    Owned,
    /// A part of an error, `NulError`'s `nul_position()` and `into_vec()`,
    /// its items, `IntoStringError`'s `into_cstring()` and `utf8_error()`,
    /// its fields.
    Item(i128),
    Field(&'static str),
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// A C string's call: `None` if `known` is another.
    pub(in crate::lower) fn c_str_call(
        &mut self,
        known: Std,
        call: Call<'_, 'tcx>,
        values: &mut std::vec::IntoIter<Expr>,
        out: &mut Vec<Stmt>,
    ) -> R<Option<Expr>> {
        let Std::CStr(op) = known else {
            return Ok(None);
        };
        let mut arg = || values.next().expect("rustc checked the arguments");
        let helper = |name: &'static str, list: Vec<Expr>| Expr::call(Expr::helper(name), list);
        // Its bytes without the nul: a part, as `&bytes[..n]` is.
        let bytes = |c: Expr| helper("$cstrBytes", vec![c]);
        Ok(Some(match op {
            CStrOp::WithNul => helper("$cstrWithNul", vec![arg()]),
            CStrOp::UntilNul => helper("$cstrUntilNul", vec![arg()]),
            CStrOp::Bytes => bytes(arg()),
            CStrOp::Count => Expr::bin(js::Op::Sub, Expr::member(arg(), "length"), Expr::int(1)),
            CStrOp::IsEmpty => Expr::bin(js::Op::Eq, Expr::member(arg(), "length"), Expr::int(1)),
            CStrOp::ToStr => helper("$fromUtf8", vec![bytes(arg())]),
            CStrOp::Lossy => helper("$utf8Lossy", vec![bytes(arg())]),
            CStrOp::New => {
                let given = arg();
                let bytes = self.given_bytes(given, &call, out)?;
                helper("$cstringNew", vec![bytes])
            }
            CStrOp::FromVec => helper("$cstringFromVec", vec![arg()]),
            CStrOp::FromVecWithNul => helper("$cstringWithNul", vec![arg()]),
            CStrOp::IntoBytes => helper("$cstringIntoBytes", vec![arg()]),
            CStrOp::IntoString => helper("$cstringIntoString", vec![arg()]),
            CStrOp::Owned => Expr::call(Expr::member(arg(), "slice"), Vec::new()),
            CStrOp::Item(at) => Expr::index(arg(), Expr::int(at)),
            CStrOp::Field(name) => Expr::member(arg(), name),
        }))
    }

    /// `CString::new(t)`'s bytes: `t` as std's `Into<Vec<u8>>` makes it,
    /// lowered as that call is, each its own array.
    fn given_bytes(&mut self, given: Expr, call: &Call<'_, 'tcx>, out: &mut Vec<Stmt>) -> R<Expr> {
        let tcx = self.tcx;
        let from = call.generic_args.type_at(0);
        // Its bound's `Vec<u8>`, `T: Into<Vec<u8>>`, and that `into`.
        let (into, bytes) = crate::lower::recognition::into_bound(tcx, call.def_id).expect("`T: Into<Vec<u8>>`");
        let args = tcx.mk_args(&[from.into(), bytes.into()]);
        let Some(known) = self.recognition().classify(into, args) else {
            let what = format!("`CString::new` of a `{from}`, whose bytes rust-js doesn't know");
            return Err(self.unsupported(call.span, &what));
        };
        let tys = [from];
        // Text's bytes, a string's method's (`String`'s `into_bytes()`).
        if let Std::Text(op) = known {
            return self.text_values(op, vec![given], &tys, args, call.span, out);
        }
        let converted = Call {
            def_id: into,
            generic_args: args,
            tys: &tys,
            ..*call
        };
        self.std_values(known, converted, vec![given], false, bytes, out)
    }
}
