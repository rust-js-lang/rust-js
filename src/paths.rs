//! Paths as every phase compares them: absolute, with what exists of them
//! resolved, so two names for one file are one, and one seen from another.
//! No phase's own: artifact planning, libraries, hooks and settings ask.

use std::path::{Path, PathBuf};

/// A file's directory, as a path that works even for a bare file name.
pub(crate) fn parent_dir(path: &Path) -> &Path {
    match path.parent() {
        Some(dir) if !dir.as_os_str().is_empty() => dir,
        _ => Path::new("."),
    }
}

/// `to` as seen from directory `from`, e.g. `../examples/fib.rs`. Resolve
/// existing ancestors without requiring the output directory to exist yet.
pub(crate) fn relative(from: &Path, to: &Path) -> String {
    relative_resolved(&resolve(from), &resolve(to))
}

/// A path with what exists of it resolved, as `relative` compares them.
pub(crate) fn resolve(path: &Path) -> PathBuf {
    absolute(path).unwrap_or_else(|_| path.to_path_buf())
}

/// `relative` of two paths already resolved.
pub(crate) fn relative_resolved(from: &Path, to: &Path) -> String {
    let from: Vec<_> = from.components().collect();
    let to: Vec<_> = to.components().collect();
    let common = from.iter().zip(&to).take_while(|(a, b)| a == b).count();
    let mut relative: PathBuf = std::iter::repeat_n("..", from.len() - common).collect();
    relative.extend(&to[common..]);
    relative.to_string_lossy().into_owned()
}

/// The file a relative `specifier`, `../util.js`, names from `dir`: each
/// `.` and `..` of it taken as written, as JS resolves it, then `absolute`.
pub(crate) fn specifier_file(dir: &Path, specifier: &str) -> Result<PathBuf, String> {
    let mut path = dir.to_path_buf();
    for part in specifier.split('/') {
        match part {
            // A WASI host's `absolute` is the path as it is, so `./` is dropped here.
            "" | "." => {}
            ".." => {
                path.pop();
            }
            part => path.push(part),
        }
    }
    absolute(&path)
}

/// `path`, absolute, through each symlink of what exists of it.
pub(crate) fn absolute(path: &Path) -> Result<PathBuf, String> {
    if cfg!(target_os = "wasi") {
        // The virtual filesystem has absolute preopened paths; realpath is not
        // provided by all WASI hosts (including browser shims).
        return Ok(path.to_path_buf());
    }
    // Resolve existing ancestors too, so aliases through symlinks collide.
    if path.exists() {
        std::fs::canonicalize(path).map_err(|e| e.to_string())
    } else {
        let path = std::path::absolute(path).map_err(|e| e.to_string())?;
        let parent = absolute(parent_dir(&path))?;
        Ok(parent.join(path.file_name().ok_or("output has no filename")?))
    }
}
