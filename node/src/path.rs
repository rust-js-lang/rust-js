//! [`path`](https://nodejs.org/api/path.html): file paths, as this
//! platform writes them; [`posix`]'s or [`win32`]'s, as one platform does.

use core::marker::PhantomData;

use js::JsObject;

unsafe extern "Rust" {

    /// [`path.sep`](https://nodejs.org/api/path.html#pathsep): the separator
    /// of a path's parts, `"/"` or `"\\"`.
    #[link_name = "path#sep"]
    #[allow(non_upper_case_globals)]
    pub safe static sep: &'static str;

    /// [`path.delimiter`](https://nodejs.org/api/path.html#pathdelimiter): the
    /// delimiter of `PATH`'s paths, `":"` or `";"`.
    #[link_name = "path#delimiter"]
    #[allow(non_upper_case_globals)]
    pub safe static delimiter: &'static str;

    /// [`path.posix`](https://nodejs.org/api/path.html#pathposix): POSIX's
    /// paths, on any platform.
    #[link_name = "path#posix"]
    #[allow(non_upper_case_globals)]
    pub safe static posix: &'static PlatformPath;

    /// [`path.win32`](https://nodejs.org/api/path.html#pathwin32): Windows'
    /// paths, on any platform.
    #[link_name = "path#win32"]
    #[allow(non_upper_case_globals)]
    pub safe static win32: &'static PlatformPath;
}

/// [`path.normalize(path)`](https://nodejs.org/api/path.html#pathnormalizepath):
/// `path`, its `..` and `.` and repeated separators resolved.
#[cfg_attr(rust_js, rust_js::link_name = "path#normalize")]
pub fn normalize(path: &str) -> String {
    unreachable!()
}

/// [`path.join(...paths)`](https://nodejs.org/api/path.html#pathjoinpaths):
/// `paths`, joined by the separator, and normalized.
#[cfg_attr(rust_js, rust_js::link_name = "path#join")]
#[cfg_attr(rust_js, rust_js::variadic)]
pub fn join(paths: &[&str]) -> String {
    unreachable!()
}

/// [`path.resolve(...paths)`](https://nodejs.org/api/path.html#pathresolvepaths):
/// the absolute path `paths` make, right to left, from the working
/// directory.
#[cfg_attr(rust_js, rust_js::link_name = "path#resolve")]
#[cfg_attr(rust_js, rust_js::variadic)]
pub fn resolve(paths: &[&str]) -> String {
    unreachable!()
}

/// [`path.matchesGlob(path, pattern)`](https://nodejs.org/api/path.html#pathmatchesglobpath-pattern):
/// whether `path` matches the glob `pattern`.
#[cfg_attr(rust_js, rust_js::link_name = "path#matchesGlob")]
pub fn matches_glob(path: &str, pattern: &str) -> bool {
    unreachable!()
}

/// [`path.isAbsolute(path)`](https://nodejs.org/api/path.html#pathisabsolutepath).
#[cfg_attr(rust_js, rust_js::link_name = "path#isAbsolute")]
pub fn is_absolute(path: &str) -> bool {
    unreachable!()
}

/// [`path.relative(from, to)`](https://nodejs.org/api/path.html#pathrelativefrom-to):
/// the path from `from` to `to`.
#[cfg_attr(rust_js, rust_js::link_name = "path#relative")]
pub fn relative(from: &str, to: &str) -> String {
    unreachable!()
}

/// [`path.dirname(path)`](https://nodejs.org/api/path.html#pathdirnamepath):
/// the directory `path` is in.
#[cfg_attr(rust_js, rust_js::link_name = "path#dirname")]
pub fn dirname(path: &str) -> String {
    unreachable!()
}

/// [`path.basename(path)`](https://nodejs.org/api/path.html#pathbasenamepath-suffix):
/// `path`'s last part.
#[cfg_attr(rust_js, rust_js::link_name = "path#basename")]
pub fn basename(path: &str) -> String {
    unreachable!()
}

/// `path.basename(path, suffix)`: `path`'s last part, without `suffix`.
#[cfg_attr(rust_js, rust_js::link_name = "path#basename")]
pub fn basename_with_suffix(path: &str, suffix: &str) -> String {
    unreachable!()
}

/// [`path.extname(path)`](https://nodejs.org/api/path.html#pathextnamepath):
/// `path`'s extension, from its last `.`, `".md"`, or `""`.
#[cfg_attr(rust_js, rust_js::link_name = "path#extname")]
pub fn extname(path: &str) -> String {
    unreachable!()
}

/// [`path.parse(path)`](https://nodejs.org/api/path.html#pathparsepath):
/// `path`'s parts.
#[cfg_attr(rust_js, rust_js::link_name = "path#parse")]
pub fn parse(path: &str) -> ParsedPath {
    unreachable!()
}

/// [`path.format(pathObject)`](https://nodejs.org/api/path.html#pathformatpathobject):
/// the path of `path_object`'s parts.
#[cfg_attr(rust_js, rust_js::link_name = "path#format")]
pub fn format(path_object: FormatInputPathObject<'_>) -> String {
    unreachable!()
}

/// [`path.toNamespacedPath(path)`](https://nodejs.org/api/path.html#pathtonamespacedpathpath):
/// on Windows, `path` as a namespace-prefixed path; elsewhere `path`.
#[cfg_attr(rust_js, rust_js::link_name = "path#toNamespacedPath")]
pub fn to_namespaced_path(path: &str) -> String {
    unreachable!()
}

/// What [`parse`] gives: a path's parts.
pub struct ParsedPath {
    /// `"/"`, `"C:\\"`, or `""`.
    pub root: String,
    /// The directory it's in.
    pub dir: String,
    /// Its last part, `"file.txt"`.
    pub base: String,
    /// Its extension, `".txt"`.
    pub ext: String,
    /// Its last part without the extension, `"file"`.
    pub name: String,
}

/// What [`format`] takes: a path's parts, each `None` but what's given.
/// `dir` is before `root`, and `base` before `name` and `ext`.
#[derive(Default)]
pub struct FormatInputPathObject<'a> {
    pub root: Option<&'a str>,
    pub dir: Option<&'a str>,
    pub base: Option<&'a str>,
    pub ext: Option<&'a str>,
    pub name: Option<&'a str>,
}

/// One platform's paths, [`posix`] or [`win32`]: `path`'s functions, of
/// its separator.
pub struct PlatformPath(PhantomData<JsObject>);

impl PlatformPath {
    #[cfg_attr(rust_js, rust_js::link_name = "normalize")]
    pub fn normalize(&self, path: &str) -> String {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "join")]
    #[cfg_attr(rust_js, rust_js::variadic)]
    pub fn join(&self, paths: &[&str]) -> String {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "resolve")]
    #[cfg_attr(rust_js, rust_js::variadic)]
    pub fn resolve(&self, paths: &[&str]) -> String {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "matchesGlob")]
    pub fn matches_glob(&self, path: &str, pattern: &str) -> bool {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "isAbsolute")]
    pub fn is_absolute(&self, path: &str) -> bool {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "relative")]
    pub fn relative(&self, from: &str, to: &str) -> String {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "dirname")]
    pub fn dirname(&self, path: &str) -> String {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "basename")]
    pub fn basename(&self, path: &str) -> String {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "basename")]
    pub fn basename_with_suffix(&self, path: &str, suffix: &str) -> String {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "extname")]
    pub fn extname(&self, path: &str) -> String {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "get sep")]
    pub fn sep(&self) -> String {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "get delimiter")]
    pub fn delimiter(&self) -> String {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "parse")]
    pub fn parse(&self, path: &str) -> ParsedPath {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "format")]
    pub fn format(&self, path_object: FormatInputPathObject<'_>) -> String {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "toNamespacedPath")]
    pub fn to_namespaced_path(&self, path: &str) -> String {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "get posix")]
    pub fn posix(&self) -> &'static PlatformPath {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "get win32")]
    pub fn win32(&self) -> &'static PlatformPath {
        unreachable!()
    }
}
