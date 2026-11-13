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
    Debug: () => ({ fmt: (e, options) => $ioErrorDebug(e, options) }),
    Display: () => ({ fmt: (e, options) => $ioErrorDisplay(e, options) }),
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
