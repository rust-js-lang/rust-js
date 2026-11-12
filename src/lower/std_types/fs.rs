//! std's files (#1156, ADR 0425 amended): Node's fs where it runs, each
//! function that reaches it a helper of the runtime's `fs.js`, which in a
//! browser panics, saying which (ADR 0391, amended). A `File` is `{ fd, pos,
//! path, read, write, append }`, its descriptor and the position std's would
//! keep, a `u64` the runtime passes to each read and write, so `Seek` moves
//! it, and the path and modes it was opened with, which `{:?}` shows
//! (#1397); its `Read`, `Write` and `Seek` are `$fileReader`'s and
//! `$fileWriter`'s, a `&File`'s too. It's closed once it's collected: rustc
//! checks a program for a target whose `File` drops nothing (ADR 0262, a
//! decided exception). An `OpenOptions` is the runtime's `$OpenOptions`,
//! whose setters give it back, std's `&mut Self`, so a chain of them is a
//! chain of methods. A `Metadata` is Node's `Stats` of `bigint`s, a
//! `FileType` the name of the kind it says, `"dir"`, and a `DirEntry` `{
//! path, name, type }` (#1397). An error of the OS's is std's `Os` error, its
//! kind and message std's of its code.

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
    /// `f.metadata()`: `$fileMetadata(f)`, `fstat`'s `Stats`.
    FileMetadata,
    /// `f.sync_all()` and `f.sync_data()`: `$fileSyncAll(f)` and
    /// `$fileSyncData(f)`, their errors thrown.
    SyncAll,
    SyncData,
    /// `f.set_len(size)`: `$fileSetLen(f, size)`, its error thrown.
    SetLen,
    /// `fs::metadata(path)` and `fs::symlink_metadata(path)`:
    /// `$fsMetadata(path)` and `$fsSymlinkMetadata(path)`, `stat`'s and
    /// `lstat`'s `Stats`.
    Metadata,
    SymlinkMetadata,
    /// `fs::create_dir(path)` and `fs::create_dir_all(path)`:
    /// `$fsCreateDir(path)` and `$fsCreateDirAll(path)`, std's steps word for
    /// word, their errors thrown.
    CreateDir,
    CreateDirAll,
    /// `fs::remove_dir(path)`: `$fsRemoveDir(path)`, `rmdir`, its error
    /// thrown.
    RemoveDir,
    /// `fs::read_dir(path)`: `$fsReadDir(path)`, `Ok` of its entries, a
    /// JS iterator that reads each as it's asked.
    ReadDir,
    /// `fs::canonicalize(path)` and `p.canonicalize()`:
    /// `$fsCanonicalize(path)` and `$pathCanonicalize(path)`, `realpath`'s,
    /// `Ok` of the absolute path, each link followed, or `Err` of the OS's
    /// error, quickcheck's (#1420).
    Canonicalize,
    PathCanonicalize,
    /// What a `Metadata`'s or a `DirEntry`'s method gives that its value
    /// holds as it is: `m.len()`'s `m.size`, `entry.path()`'s `entry.path`.
    Field(&'static str),
    /// What a `Metadata`'s `Stats` answers by its method: `m.is_dir()`'s
    /// `m.isDirectory()`.
    StatsMethod(&'static str),
    /// `m.file_type()`: `$fileType(m)`, the kind's name.
    FileTypeOf,
    /// `t.is_dir()` of a `FileType` and its siblings: `t === "dir"`.
    FileTypeIs(&'static str),
    /// `m.modified()` and `m.accessed()`: `$modified(m)` and
    /// `$accessed(m)`, `Ok` of a `SystemTime`.
    Time(&'static str),
    /// `entry.file_type()`: `Ok` of the type it holds, `{ TAG: "Ok", _0:
    /// entry.type }`.
    EntryType,
    /// `entry.metadata()`: `$entryMetadata(entry)`, `lstat`'s.
    EntryMetadata,
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
            FsOp::FileMetadata => "$fileMetadata",
            FsOp::SyncAll => "$fileSyncAll",
            FsOp::SyncData => "$fileSyncData",
            FsOp::SetLen => "$fileSetLen",
            FsOp::Metadata => "$fsMetadata",
            FsOp::SymlinkMetadata => "$fsSymlinkMetadata",
            FsOp::CreateDir => "$fsCreateDir",
            FsOp::CreateDirAll => "$fsCreateDirAll",
            FsOp::RemoveDir => "$fsRemoveDir",
            FsOp::ReadDir => "$fsReadDir",
            FsOp::Canonicalize => "$fsCanonicalize",
            FsOp::PathCanonicalize => "$pathCanonicalize",
            FsOp::FileTypeOf => "$fileType",
            FsOp::Time(helper) => helper,
            FsOp::EntryMetadata => "$entryMetadata",
            FsOp::OptionsSet(method) | FsOp::StatsMethod(method) => return Called::Method(method),
            FsOp::OptionsOpen => return Called::Method("open"),
            FsOp::Field(field) => return Called::Field(field),
            FsOp::FileTypeIs(kind) => return Called::Kind(kind),
            FsOp::EntryType => return Called::OkOf("type"),
        })
    }
}

/// How an `FsOp` is called: a helper given its arguments, a method of its
/// first, a property of it, whether it, a `FileType`, is the kind named,
/// or `Ok` of its property.
enum Called {
    Helper(&'static str),
    Method(&'static str),
    Field(&'static str),
    Kind(&'static str),
    OkOf(&'static str),
}

/// Which of std's files' methods `path` is, that reach no file by a path:
/// `OpenOptions`'s `new` and each flag's setter, of its method's name; a
/// `File`'s that ask the OS of it; and what a `Metadata`, a `FileType` or a
/// `DirEntry` answers (#1397). The one list of them.
pub(in crate::lower) fn fs_method_op(path: &str) -> Option<FsOp> {
    Some(match path {
        "std::fs::OpenOptions::new" | "std::fs::File::options" => FsOp::OptionsNew,
        "std::fs::OpenOptions::read" => FsOp::OptionsSet("read"),
        "std::fs::OpenOptions::write" => FsOp::OptionsSet("write"),
        "std::fs::OpenOptions::append" => FsOp::OptionsSet("append"),
        "std::fs::OpenOptions::truncate" => FsOp::OptionsSet("truncate"),
        "std::fs::OpenOptions::create" => FsOp::OptionsSet("create"),
        "std::fs::OpenOptions::create_new" => FsOp::OptionsSet("createNew"),
        "std::fs::File::metadata" => FsOp::FileMetadata,
        "std::fs::File::sync_all" => FsOp::SyncAll,
        "std::fs::File::sync_data" => FsOp::SyncData,
        "std::fs::File::set_len" => FsOp::SetLen,
        "std::fs::Metadata::len" => FsOp::Field("size"),
        "std::fs::Metadata::is_file" => FsOp::StatsMethod("isFile"),
        "std::fs::Metadata::is_dir" => FsOp::StatsMethod("isDirectory"),
        "std::fs::Metadata::is_symlink" => FsOp::StatsMethod("isSymbolicLink"),
        "std::fs::Metadata::file_type" => FsOp::FileTypeOf,
        "std::fs::Metadata::modified" => FsOp::Time("$modified"),
        "std::fs::Metadata::accessed" => FsOp::Time("$accessed"),
        "std::fs::FileType::is_file" => FsOp::FileTypeIs("file"),
        "std::fs::FileType::is_dir" => FsOp::FileTypeIs("dir"),
        "std::fs::FileType::is_symlink" => FsOp::FileTypeIs("symlink"),
        "std::fs::DirEntry::path" => FsOp::Field("path"),
        "std::fs::DirEntry::file_name" => FsOp::Field("name"),
        "std::fs::DirEntry::file_type" => FsOp::EntryType,
        "std::fs::DirEntry::metadata" => FsOp::EntryMetadata,
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
            FsOp::Open
            | FsOp::Create
            | FsOp::Read
            | FsOp::ReadToString
            | FsOp::Write
            | FsOp::RemoveFile
            | FsOp::Metadata
            | FsOp::SymlinkMetadata
            | FsOp::CreateDir
            | FsOp::CreateDirAll
            | FsOp::RemoveDir
            | FsOp::ReadDir
            | FsOp::Canonicalize
            | FsOp::PathCanonicalize => Some(0),
            FsOp::OptionsNew
            | FsOp::OptionsSet(_)
            | FsOp::FileMetadata
            | FsOp::SyncAll
            | FsOp::SyncData
            | FsOp::SetLen
            | FsOp::Field(_)
            | FsOp::StatsMethod(_)
            | FsOp::FileTypeOf
            | FsOp::FileTypeIs(_)
            | FsOp::Time(_)
            | FsOp::EntryType
            | FsOp::EntryMetadata => None,
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
            called => {
                let mut given = given.into_iter();
                let of = given.next().expect("what it's of");
                match called {
                    Called::Method(method) => Expr::call(Expr::member(of, method), given.collect()),
                    Called::Field(field) => Expr::member(of, field),
                    Called::Kind(kind) => Expr::bin(crate::js::Op::Eq, of, Expr::str(kind)),
                    Called::OkOf(field) => crate::js::result::ok(Expr::member(of, field)),
                    Called::Helper(_) => unreachable!("called above"),
                }
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
