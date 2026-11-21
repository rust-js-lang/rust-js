// An `io::Error` (ADR 0462): its kind, an `ErrorKind`'s name, and the
// error it holds, a `dyn Error` pair, `{ value, impl }`, where it was made
// of one, or std's own message, `write_all`'s; or the OS's, its `code`, its
// number, beside its message, the platform's (#1156). A class, as a
// `fmt::Error` is, so that it's thrown where an `io::Result<()>` is an
// `Err` and told apart from a panic.
class $IoError extends $WriteError {
  constructor(kind, error, message, code) {
    super();
    this.kind = kind;
    this.error = error;
    this.message = message;
    this.code = code;
  }
}
// An `io::Result<()>`'s `Err`: its error, thrown.
function $ioThrow(error) {
  throw error;
}
// What each `ErrorKind` says it is, std's `as_str`.
const $ioKinds = {
  AddrInUse: "address in use",
  AddrNotAvailable: "address not available",
  AlreadyExists: "entity already exists",
  ArgumentListTooLong: "argument list too long",
  BrokenPipe: "broken pipe",
  ConnectionAborted: "connection aborted",
  ConnectionRefused: "connection refused",
  ConnectionReset: "connection reset",
  CrossesDevices: "cross-device link or rename",
  Deadlock: "deadlock",
  DirectoryNotEmpty: "directory not empty",
  ExecutableFileBusy: "executable file busy",
  FileTooLarge: "file too large",
  FilesystemLoop: "filesystem loop or indirection limit (e.g. symlink loop)",
  HostUnreachable: "host unreachable",
  InProgress: "in progress",
  InputOutputError: "input/output error",
  Interrupted: "operation interrupted",
  InvalidData: "invalid data",
  InvalidFilename: "invalid filename",
  InvalidInput: "invalid input parameter",
  IsADirectory: "is a directory",
  NetworkDown: "network down",
  NetworkUnreachable: "network unreachable",
  NotADirectory: "not a directory",
  NotConnected: "not connected",
  NotFound: "entity not found",
  NotSeekable: "seek on unseekable file",
  Other: "other error",
  OutOfMemory: "out of memory",
  PermissionDenied: "permission denied",
  QuotaExceeded: "quota exceeded",
  ReadOnlyFilesystem: "read-only filesystem or storage medium",
  ResourceBusy: "resource busy",
  StaleNetworkFileHandle: "stale network file handle",
  StorageFull: "no storage space",
  TimedOut: "timed out",
  TooManyLinks: "too many links",
  TooManyOpenFiles: "too many open files",
  Uncategorized: "uncategorized error",
  UnexpectedEof: "unexpected end of file",
  Unsupported: "unsupported",
  WouldBlock: "operation would block",
  WriteZero: "write zero",
};
// `{}` of an `ErrorKind`: its words.
function $ioKindText(kind) {
  return $ioKinds[kind];
}
// `{}` of an `io::Error`, given the `Formatter`'s options: the OS's message
// and number, `write!`'s; what its error shows, given them; its message,
// `pad` of it; or its kind's words, `write_str`'s (#1689).
function $ioErrorDisplay(e, options) {
  if (e.code !== undefined) return `${e.message} (os error ${e.code})`;
  if (e.error !== undefined) return e.error.impl.Display().fmt(e.error.value, options);
  if (e.message) return $formatted(e.message, options);
  return $ioKindText(e.kind);
}
// `{}` of one into a writer, as std's writes it: the OS's message and
// `write!`'s pieces of its number; its error's, given the options; its
// message's `pad`; or its kind's words, one `write_str` (#1652).
function $writeIoError(f, e, options) {
  if (e.code !== undefined) {
    f.write(e.message);
    f.write(" (os error ");
    $writeInteger(f, e.code);
    return f.write(")");
  }
  if (e.error !== undefined) return e.error.impl.Display().fmtInto(f, e.error.value, options);
  if (e.message) return $writePadded(f, e.message, options);
  return f.write($ioKindText(e.kind));
}
// `{:?}` of one, std's `Os { code, kind, message }`, `Custom { kind, error
// }` or `Kind(..)`, pretty where `options` say.
function $ioErrorDebug(e, options) {
  if (e.code !== undefined) {
    const fields = [`code: ${e.code}`, `kind: ${e.kind}`, `message: ${$debugStr(e.message)}`];
    return options?.alternate ? $pretty("Os {", fields, "}") : `Os { ${fields.join(", ")} }`;
  }
  if (e.message) {
    const fields = [`kind: ${e.kind}`, `message: ${$debugStr(e.message)}`];
    return options?.alternate ? $pretty("Error {", fields, "}") : `Error { ${fields.join(", ")} }`;
  }
  if (e.error === undefined) return options?.alternate ? $pretty("Kind(", [e.kind], ")") : `Kind(${e.kind})`;
  const error = e.error.impl.Debug().fmt(e.error.value, options);
  return options?.alternate
    ? $pretty("Custom {", [`kind: ${e.kind}`, `error: ${error}`], "}")
    : `Custom { kind: ${e.kind}, error: ${error} }`;
}
// `{:?}` of one into a writer, as std's writes it, by its builders (#2250):
// an OS error's `Os { code, kind, message }`, its message between two
// quotes, each a `write_str`, as std's `write!(fmt, "\"{}\"", ..)` writes
// it; a message's `Error { kind, message }`, its message a `str`'s
// `Debug`; one holding an error, `Custom { kind, error }`, the error by
// its own dictionary's `Debug`; a kind's `Kind(..)`. A kind is its
// variant's name, one `write_str`.
function $writeIoErrorDebug(f, e, options) {
  const pretty = options?.alternate;
  const kind = ["kind", e.kind, (g, kind) => g.write(kind)];
  if (e.code !== undefined) {
    const message = (g, message) => {
      g.write('"');
      g.write(message);
      g.write('"');
    };
    return $writeDebugStruct(f, "Os", [["code", e.code, (g, code) => $writeInteger(g, code)], kind, ["message", e.message, message]], pretty);
  }
  if (e.message) return $writeDebugStruct(f, "Error", [kind, ["message", e.message, $writeDebugStr]], pretty);
  if (e.error === undefined) return $writeDebugTuple(f, "Kind", [[e.kind, (g, kind) => g.write(kind)]], pretty);
  const error = (g, error) => $writeBy(g, error.impl.Debug(), error.value, options);
  $writeDebugStruct(f, "Custom", [kind, ["error", e.error, error]], pretty);
}
// `{:?}` of an `io::Result<()>`, `undefined` or its error.
function $ioResultDebug(result, options) {
  if (result === undefined) return "Ok(())";
  const error = $ioErrorDebug(result, options);
  return options?.alternate ? $pretty("Err(", [error], ")") : `Err(${error})`;
}
// An `io::Error`, of `TypeId` `type`, as a `dyn Error`'s dictionary (ADR
// 0141); its `type_id` `type` (ADR 0331), and its drop, what it holds.
function $ioErrorDyn(type) {
  return {
    Debug: () => ({ fmt: (e, options) => $ioErrorDebug(e, options), fmtInto: $writeIoErrorDebug }),
    Display: () => ({ fmt: (e, options) => $ioErrorDisplay(e, options), fmtInto: $writeIoError }),
    // Its error's own `source`, as std's is.
    source: (e) => e.error?.impl.source(e.error.value),
    typeId: () => type,
    $drop: $ioErrorDrop,
  };
}
// The drop of an `io::Error`: of the error it holds, a `dyn Error` pair, its
// dictionary's drop, where what it holds has one; of an `io::Result<()>`,
// `undefined` or its error, its error's (ADR 0462, amended).
function $ioErrorDrop(e) {
  e?.error?.impl.$drop?.(e.error.value);
}
