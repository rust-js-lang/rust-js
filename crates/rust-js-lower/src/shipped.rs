//! The API of a crate rustc builds that rust-js writes the JS of, as it
//! writes std's: memchr's searches (ADR 0529), over the byte arrays slices
//! are, whose indices are the slice's; getrandom's bytes, Web Crypto's,
//! and its `Error`, its code (#2326); and serde_path_to_error's read, its
//! `Error` serde_json's and where in the value it was made (ADR 1188).

use super::recognition::Std;
use super::{Expr, FnCx};
use rust_js_program::js;
use rust_js_program::js::Op;

/// One of memchr's searches: the first or last index of one of its needles,
/// or each, from either end.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MemchrOp {
    /// `memchr`, `memchr2` and `memchr3`, or of `back`, their `memrchr`s.
    Find { back: bool },
    /// Their iterators, `memchr_iter`, or of `back`, `memrchr_iter`, its
    /// `rev()`.
    Iter { back: bool },
}

/// Which of getrandom's APIs a qualified one has, by its version (#2326):
/// 0.1's and 0.2's `getrandom` and `Error` of a `NonZeroU32`, each with its
/// own table of what its internal codes say, or 0.3's and 0.4's `fill`,
/// `u32` and `u64` and `Error` of a `NonZeroI32`, whose tables are one
/// under the custom backend rustc checks them with.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GetrandomVersion {
    V01,
    V02,
    V03,
}

impl GetrandomVersion {
    /// The API of a qualified version, `0.2.17`'s.
    pub(crate) fn of(version: &str) -> Option<Self> {
        match version.split('.').take(2).collect::<Vec<_>>()[..] {
            ["0", "1"] => Some(Self::V01),
            ["0", "2"] => Some(Self::V02),
            ["0", "3" | "4"] => Some(Self::V03),
            _ => None,
        }
    }

    /// Whether its `Error` has a `code()`, a `NonZeroU32`, and is made of
    /// one: 0.1's and 0.2's.
    pub(crate) fn has_code(self) -> bool {
        self != Self::V03
    }

    /// Its name to the runtime, whose table of its `Error`'s words it picks.
    pub(crate) fn key(self) -> &'static str {
        match self {
            Self::V01 => "0.1",
            Self::V02 => "0.2",
            Self::V03 => "0.3",
        }
    }
}

/// One of getrandom's functions rust-js writes (#2326).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GetrandomOp {
    /// 0.1's and 0.2's `getrandom`, 0.3's and 0.4's `fill`, or of `uninit`,
    /// 0.2's `getrandom_uninit` and their `fill_uninit`: the slice filled
    /// with Web Crypto's bytes in place, `Ok` of nothing, or of the slice.
    Fill {
        uninit: bool,
        version: GetrandomVersion,
    },
    /// 0.3's and 0.4's `u32()` and `u64()`: a random number.
    U32,
    U64,
    /// An `Error`'s `code()`, and one made of a code, `From<NonZeroU32>`:
    /// the code itself, which it is.
    Code,
    /// An `Error`'s `raw_os_error()`: its OS's number, as its version reads
    /// it of its code.
    RawOsError(GetrandomVersion),
    /// 0.3's and 0.4's `Error::new_custom(n)`: its code, past `CUSTOM_START`.
    NewCustom,
}

/// One of serde_path_to_error's functions rust-js writes, or serde_json's
/// reader of text, which its `deserialize` reads (ADR 1188).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum JsonPathOp {
    /// serde_json's `Deserializer::from_str(text)`: the runtime's reader of
    /// it, `new $JsonReader(text)`.
    Reader,
    /// `deserialize::<_, T>(reader)`: `$jsonPathRead(reader, read)`, `read`
    /// the function that reads a `T`.
    Read,
    /// An `Error`'s `path()`: its segments.
    Path,
    /// An `Error`'s `inner()` and `into_inner()`: serde_json's error.
    Inner,
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// A call of serde_path_to_error's, or serde_json's reader of text made,
    /// `given` its one argument (ADR 1188).
    pub fn json_path_call(
        &mut self,
        op: JsonPathOp,
        given: Expr,
        args: rustc_middle::ty::GenericArgsRef<'tcx>,
        span: rustc_span::Span,
    ) -> crate::R<Expr> {
        Ok(match op {
            JsonPathOp::Reader => Expr::new_(Expr::helper("$JsonReader"), vec![given]),
            JsonPathOp::Read => {
                let ty = args.types().nth(1).expect("`deserialize::<D, T>`");
                self.json_path_value(given, ty, span)?
            }
            JsonPathOp::Path => Expr::member(given, "path"),
            JsonPathOp::Inner => Expr::member(given, "original"),
        })
    }

    /// A call of getrandom's (#2326): `$getrandom(dest, ..)` of a fill, the
    /// runtime's; a code as it is. `None` if `known` is another.
    pub(crate) fn getrandom_call(&mut self, known: Std, values: &mut std::vec::IntoIter<Expr>) -> Option<Expr> {
        let Std::Getrandom(op) = known else {
            return None;
        };
        let mut given: Vec<Expr> = values.collect();
        Some(match op {
            GetrandomOp::Fill { uninit, version } => {
                let dest = given.pop().expect("rustc checked the arguments");
                let mut args = vec![dest, Expr::str(version.key())];
                if uninit {
                    args.push(Expr::bool(true));
                }
                Expr::call_helper("$getrandom", args)
            }
            GetrandomOp::U32 => Expr::call_helper("$getrandomU32", Vec::new()),
            GetrandomOp::U64 => Expr::call_helper("$getrandomU64", Vec::new()),
            GetrandomOp::Code => given.pop().expect("rustc checked the arguments"),
            GetrandomOp::RawOsError(version) => {
                let error = given.pop().expect("rustc checked the arguments");
                Expr::call_helper("$getrandomRawOsError", vec![error, Expr::str(version.key())])
            }
            GetrandomOp::NewCustom => {
                let n = given.pop().expect("rustc checked the arguments");
                Expr::bin(Op::Add, Expr::int(1 << 17), n)
            }
        })
    }

    /// A call of memchr's: its needles, then the haystack, the runtime's
    /// helper given the haystack and the needles, `$memchr(text, [111])`.
    /// `None` if `known` is another.
    pub(crate) fn memchr_call(&mut self, known: Std, values: &mut std::vec::IntoIter<Expr>) -> Option<Expr> {
        let Std::Memchr(op) = known else {
            return None;
        };
        let mut given: Vec<Expr> = values.collect();
        let haystack = given.pop().expect("rustc checked the arguments");
        let needles = Expr::array(given);
        Some(match op {
            MemchrOp::Find { back: false } => Expr::call_helper("$memchr", vec![haystack, needles]),
            MemchrOp::Find { back: true } => Expr::call_helper("$memrchr", vec![haystack, needles]),
            MemchrOp::Iter { back } => {
                let it = Expr::call_helper("$memchrIter", vec![haystack, needles]);
                match back {
                    true => self.held_rev(it),
                    false => it,
                }
            }
        })
    }

    /// A `Rev` made of `it`, a JS iterator that knows its back, given where
    /// it's held, `memrchr_iter`'s: std's struct, as every held `Rev` is,
    /// `iterator_rev(it, { next: $next }, { next_back: $nextBack })` (ADR
    /// 0871).
    fn held_rev(&mut self, it: Expr) -> Expr {
        let rev = super::recognition::compiled_provided_method(self.tcx, "Iterator", "rev");
        let dictionary =
            |name: &str, step: &'static str| Expr::object(vec![js::Prop::Field(name.into(), Expr::helper(step))]);
        Expr::call(
            self.fn_ref(rev),
            vec![it, dictionary("next", "$next"), dictionary("next_back", "$nextBack")],
        )
    }
}
