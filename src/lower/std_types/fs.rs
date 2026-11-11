//! std's files (#1156, ADR 0425 amended): Node's fs where it runs, each
//! function that reaches it a helper of the runtime's `fs.js`, which in a
//! browser panics, saying which (ADR 0391, amended). A `File` is `{ fd, pos
//! }`, its descriptor and the position std's would keep, a `u64` the
//! runtime passes to each read and write, so `Seek` moves it; its `Read`,
//! `Write` and `Seek` are `$fileReader`'s and `$fileWriter`'s. It's closed
//! once it's collected: rustc checks a program for a target whose `File`
//! drops nothing (ADR 0262, a decided exception). An `OpenOptions` is the
//! runtime's `$OpenOptions`, whose setters give it back, std's `&mut Self`,
//! so a chain of them is a chain of methods. An error of the OS's is std's
//! `Os` error, its kind and message std's of its code.

use crate::js::Expr;
use crate::lower::calls::Call;
use crate::lower::recognition::{Std, StdItem};
use crate::lower::{FnCx, R};
use rustc_middle::ty;

/// What's asked of std's files.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(in crate::lower) enum FsOp {
    /// `File::open(path)`: `$fileOpen(path)`, `Ok` of the file or `Err` of
    /// the OS's error.
    Open,
    /// `File::create(path)`: `$fileCreate(path)`, written from its start,
    /// made where there's none.
    Create,
    /// `fs::read(path)`: `$fsRead(path)`, `Ok` of its bytes.
    Read,
    /// `fs::read_to_string(path)`: `$fsReadToString(path)`, `Ok` of its
    /// text, `InvalidData` where it isn't UTF-8.
    ReadToString,
    /// `fs::write(path, contents)`: `$fsWrite(path, bytes)`, of what
    /// `contents`' `AsRef<[u8]>` gives; an `io::Result<()>`, its error
    /// thrown (ADR 0462).
    Write,
    /// `fs::remove_file(path)`: `$fsRemoveFile(path)`, its error thrown.
    RemoveFile,
    /// `OpenOptions::new()` and `File::options()`: `$openOptions()`, every
    /// flag `false`.
    OptionsNew,
    /// `options.read(on)` and its siblings: the method of its name,
    /// `options.read(on)`, `createNew` of `create_new`, which gives back the
    /// options, as std's `&mut Self` is.
    OptionsSet(&'static str),
    /// `options.open(path)`: its method, std's flags of them, or std's
    /// error where they ask nothing it can open.
    OptionsOpen,
}

impl FsOp {
    /// The runtime's function that does it, in `fs.js`, or the method of
    /// an `$OpenOptions` that does.
    fn called(self) -> Called {
        Called::Helper(match self {
            FsOp::Open => "$fileOpen",
            FsOp::Create => "$fileCreate",
            FsOp::Read => "$fsRead",
            FsOp::ReadToString => "$fsReadToString",
            FsOp::Write => "$fsWrite",
            FsOp::RemoveFile => "$fsRemoveFile",
            FsOp::OptionsNew => "$openOptions",
            FsOp::OptionsSet(method) => return Called::Method(method),
            FsOp::OptionsOpen => return Called::Method("open"),
        })
    }
}

/// How an `FsOp` is called: a helper given its arguments, or a method of
/// its first.
enum Called {
    Helper(&'static str),
    Method(&'static str),
}

/// Which of `OpenOptions`'s functions std's `path` is, that open nothing:
/// `new`, and each flag's setter, of its method's name.
pub(in crate::lower) fn open_options_op(path: &str) -> Option<FsOp> {
    Some(match path {
        "std::fs::OpenOptions::new" | "std::fs::File::options" => FsOp::OptionsNew,
        "std::fs::OpenOptions::read" => FsOp::OptionsSet("read"),
        "std::fs::OpenOptions::write" => FsOp::OptionsSet("write"),
        "std::fs::OpenOptions::append" => FsOp::OptionsSet("append"),
        "std::fs::OpenOptions::truncate" => FsOp::OptionsSet("truncate"),
        "std::fs::OpenOptions::create" => FsOp::OptionsSet("create"),
        "std::fs::OpenOptions::create_new" => FsOp::OptionsSet("createNew"),
        _ => return None,
    })
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// A call of std's files: its helper or method, given what it's given,
    /// a path as its text (`path_given`), contents as their bytes. `None` if
    /// `known` is another.
    pub(in crate::lower) fn fs_call(
        &mut self,
        known: Std,
        call: Call<'_, 'tcx>,
        values: &mut std::vec::IntoIter<Expr>,
    ) -> R<Option<Expr>> {
        let Std::Fs(op) = known else {
            return Ok(None);
        };
        // Where its path is, an `AsRef<Path>`: after the options it opens, or
        // first; a flag's setter has none.
        let path_at = match op {
            FsOp::OptionsOpen => Some(1),
            FsOp::OptionsNew | FsOp::OptionsSet(_) => None,
            FsOp::Open | FsOp::Create | FsOp::Read | FsOp::ReadToString | FsOp::Write | FsOp::RemoveFile => Some(0),
        };
        let mut given = Vec::new();
        for (at, (value, &ty)) in values.by_ref().zip(call.tys).enumerate() {
            given.push(match () {
                _ if Some(at) == path_at => self.path_given(value, ty, (call.def_id, at), call.span)?,
                // `fs::write`'s contents, an `AsRef<[u8]>`.
                _ if op == FsOp::Write => self.bytes_given(value, ty, call)?,
                // The options, and a flag's `bool`.
                _ => value,
            });
        }
        let called = match op.called() {
            Called::Helper(helper) => Expr::call(Expr::helper(helper), given),
            Called::Method(method) => {
                let mut given = given.into_iter();
                let options = given.next().expect("the options");
                Expr::call(Expr::member(options, method), given.collect())
            }
        };
        // `fs::write`'s and `remove_file`'s error, thrown, caught where
        // it's a value (ADR 0462).
        Ok(Some(self.fmt_result_value(call.def_id, call.generic_args, called)))
    }

    /// What `contents` of type `ty`, an `AsRef<[u8]>`, gives as bytes: its
    /// impl's, through its dictionary, as `Cursor<T>`'s is.
    fn bytes_given(&mut self, contents: Expr, ty: ty::Ty<'tcx>, call: Call<'_, 'tcx>) -> R<Expr> {
        let as_ref = crate::lower::recognition::std_item(self.tcx, StdItem::AsRef);
        let bytes = ty::Ty::new_slice(self.tcx, self.tcx.types.u8);
        let dictionary = self.dictionary(ty::TraitRef::new(self.tcx, as_ref, [ty, bytes]), call.span)?;
        Ok(Expr::call(Expr::member(dictionary, "asRef"), vec![contents]))
    }
}
