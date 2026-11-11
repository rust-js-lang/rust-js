// std's files (#1156), through Node's fs, `$nodeFs`, which in a browser
// panics, saying what needed it (ADR 0391). A `File` is `{ fd, pos }`, its
// descriptor and the position std's would keep, a `u64` passed to each read
// and write, so `Seek` moves it as `lseek` would. It's closed once it's
// collected: rustc checks a program for a target whose `File` drops nothing,
// so its MIR has no drop of one (ADR 0425, amended). An `io::Result<()>`'s
// `Err` is thrown (ADR 0462), another's a value.

// The descriptors opened to append, `O_APPEND`, whose writes go to the end,
// where the position is then.
const $appending = new Set();

// Each file's descriptor, closed once the file is collected, its error
// unseen, as std's drop's is (rd803; exact close at Rust's drop, #1391):
// made where a file is first opened.
let $openFiles;
// A file of a descriptor just opened, `{ fd, pos }`, closed once it's collected.
function $opened(fd) {
  $openFiles ??= new FinalizationRegistry((opened) => {
    $appending.delete(opened);
    try {
      $nodeFs("`std::fs::File`").closeSync(opened);
    } catch {}
  });
  const file = { fd, pos: 0n };
  $openFiles.register(file, fd);
  return file;
}

// std's `decode_error_kind` of each code it maps, and the code's message,
// `strerror`'s: glibc's, then macOS's where it differs. A code std doesn't
// map is `Uncategorized`, `EBADF`'s too.
const $osErrors = {
  E2BIG: ["ArgumentListTooLong", "Argument list too long"],
  EACCES: ["PermissionDenied", "Permission denied"],
  EADDRINUSE: ["AddrInUse", "Address already in use"],
  EADDRNOTAVAIL: ["AddrNotAvailable", "Cannot assign requested address", "Can't assign requested address"],
  EAGAIN: ["WouldBlock", "Resource temporarily unavailable"],
  EBADF: ["Uncategorized", "Bad file descriptor"],
  EBUSY: ["ResourceBusy", "Device or resource busy", "Resource busy"],
  ECONNABORTED: ["ConnectionAborted", "Software caused connection abort"],
  ECONNREFUSED: ["ConnectionRefused", "Connection refused"],
  ECONNRESET: ["ConnectionReset", "Connection reset by peer"],
  EDEADLK: ["Deadlock", "Resource deadlock avoided"],
  EDQUOT: ["QuotaExceeded", "Disk quota exceeded"],
  EEXIST: ["AlreadyExists", "File exists"],
  EFBIG: ["FileTooLarge", "File too large"],
  EHOSTUNREACH: ["HostUnreachable", "No route to host"],
  EINPROGRESS: ["InProgress", "Operation now in progress"],
  EINTR: ["Interrupted", "Interrupted system call"],
  EINVAL: ["InvalidInput", "Invalid argument"],
  EIO: ["InputOutputError", "Input/output error"],
  EISDIR: ["IsADirectory", "Is a directory"],
  ELOOP: ["FilesystemLoop", "Too many levels of symbolic links"],
  EMFILE: ["TooManyOpenFiles", "Too many open files"],
  EMLINK: ["TooManyLinks", "Too many links"],
  ENAMETOOLONG: ["InvalidFilename", "File name too long"],
  ENETDOWN: ["NetworkDown", "Network is down"],
  ENETUNREACH: ["NetworkUnreachable", "Network is unreachable"],
  ENFILE: ["TooManyOpenFiles", "Too many open files in system"],
  ENOENT: ["NotFound", "No such file or directory"],
  ENOMEM: ["OutOfMemory", "Cannot allocate memory"],
  ENOSPC: ["StorageFull", "No space left on device"],
  ENOSYS: ["Unsupported", "Function not implemented"],
  ENOTCONN: ["NotConnected", "Transport endpoint is not connected", "Socket is not connected"],
  ENOTDIR: ["NotADirectory", "Not a directory"],
  ENOTEMPTY: ["DirectoryNotEmpty", "Directory not empty"],
  EOPNOTSUPP: ["Unsupported", "Operation not supported", "Operation not supported on socket"],
  EPERM: ["PermissionDenied", "Operation not permitted"],
  EPIPE: ["BrokenPipe", "Broken pipe"],
  EROFS: ["ReadOnlyFilesystem", "Read-only file system"],
  ESPIPE: ["NotSeekable", "Illegal seek"],
  ESTALE: ["StaleNetworkFileHandle", "Stale file handle", "Stale NFS file handle"],
  ETIMEDOUT: ["TimedOut", "Connection timed out", "Operation timed out"],
  ETXTBSY: ["ExecutableFileBusy", "Text file busy"],
  EXDEV: ["CrossesDevices", "Invalid cross-device link", "Cross-device link"],
};

// std's `io::Error` of Node's error of a call, `e`: its number the OS's,
// which Node's is the negation of, its kind and message std's of its code.
// Of a code the table hasn't, `Uncategorized` and Node's words (ADR 0262,
// a known difference).
function $osError(e) {
  const known = $osErrors[e.code];
  if (known === undefined) {
    const words = e.message.slice(e.code.length + 2).split(`, ${e.syscall}`)[0];
    return new $IoError("Uncategorized", undefined, words, -e.errno);
  }
  const [kind, message, darwin] = known;
  const platform = $process("`std::io::Error`").platform;
  return new $IoError(kind, undefined, platform === "darwin" && darwin !== undefined ? darwin : message, -e.errno);
}
// `io::Error::from_raw_os_error(code)`: the OS's error of its number, named
// as the platform names it, Node's `os.constants.errno`, the name the table
// knows first where two share it, `EAGAIN`'s and not `EWOULDBLOCK`'s; its
// kind and message std's, as `$osError` gives them. One the platform names
// that the table hasn't, `Uncategorized` and Node's words, the decided
// difference (ADR 0262); one it doesn't name, `strerror`'s own words of a
// number it doesn't know, glibc's or macOS's, and `Uncategorized`.
function $osErrorOf(code) {
  const process = $process("`std::io::Error::from_raw_os_error`");
  const names = Object.entries(process.getBuiltinModule("node:os").constants.errno)
    .filter(([, number]) => number === code)
    .map(([name]) => name);
  const name = names.find((name) => $osErrors[name] !== undefined) ?? names[0];
  if (name === undefined) {
    const darwin = process.platform === "darwin";
    const words = code === 0 ? (darwin ? "Undefined error: 0" : "Success") : darwin ? `Unknown error: ${code}` : `Unknown error ${code}`;
    return new $IoError("Uncategorized", undefined, words, code);
  }
  const words = process.getBuiltinModule("node:util").getSystemErrorMessage(-code);
  return $osError({ code: name, errno: -code, message: `${name}: ${words}`, syscall: "from_raw_os_error" });
}
// Node's form of an error of the OS's the runtime finds itself, `lseek`'s
// `EINVAL` or `ESPIPE`: its number, both platforms'.
function $osFailure(code, errno) {
  return { code, errno: -errno, message: `${code}: `, syscall: "lseek" };
}

// Whether `e` is Node's error of a call of the OS's, which `$osError` reads.
function $fromOs(e) {
  return typeof e?.errno === "number" && typeof e.code === "string";
}
// `f()`'s value, `Ok`, or its error of the OS's, `Err`.
function $fsTry(f) {
  try {
    return { TAG: "Ok", _0: f() };
  } catch (e) {
    if (!$fromOs(e)) throw e;
    return { TAG: "Err", _0: $osError(e) };
  }
}
// `f()`, an `io::Result<()>`'s: its error of the OS's, thrown.
function $fsThrow(f) {
  try {
    f();
  } catch (e) {
    throw $fromOs(e) ? $osError(e) : e;
  }
}

// `File::open(path)`: read only.
function $fileOpen(path) {
  const fs = $nodeFs("`std::fs::File::open`");
  return $fsTry(() => $opened(fs.openSync(path, fs.constants.O_RDONLY)));
}
// `File::create(path)`: written from its start, made where there's none.
function $fileCreate(path) {
  const fs = $nodeFs("`std::fs::File::create`");
  const { O_WRONLY, O_CREAT, O_TRUNC } = fs.constants;
  return $fsTry(() => $opened(fs.openSync(path, O_WRONLY | O_CREAT | O_TRUNC, 0o666)));
}

// An `OpenOptions`: its flags, each set by its method, which gives back the
// options, std's `&mut Self`, so they chain as Rust's do; `open(path)` opens
// a file of them.
class $OpenOptions {
  flags = { read: false, write: false, append: false, truncate: false, create: false, createNew: false };
  read(on) {
    this.flags.read = on;
    return this;
  }
  write(on) {
    this.flags.write = on;
    return this;
  }
  append(on) {
    this.flags.append = on;
    return this;
  }
  truncate(on) {
    this.flags.truncate = on;
    return this;
  }
  create(on) {
    this.flags.create = on;
    return this;
  }
  createNew(on) {
    this.flags.createNew = on;
    return this;
  }
  open(path) {
    return $openWith(this.flags, path);
  }
}
// `OpenOptions::new()` and `File::options()`: every flag off.
function $openOptions() {
  return new $OpenOptions();
}
// std's `io::Error::new(InvalidInput, message)` of options that open nothing.
function $openInvalid(message) {
  return { TAG: "Err", _0: new $IoError("InvalidInput", { value: message, impl: $stringError() }) };
}
// `options.open(path)`, of its flags: std's `get_access_mode`, then
// `get_creation_mode`, word for word, and their flags opened.
function $openWith(o, path) {
  const fs = $nodeFs("`std::fs::OpenOptions::open`");
  const { O_RDONLY, O_WRONLY, O_RDWR, O_APPEND, O_CREAT, O_TRUNC, O_EXCL } = fs.constants;
  const creating = o.create || o.createNew || o.truncate;
  const asked = "creating or truncating a file requires write or append access";
  if (!o.read && !o.write && !o.append) {
    return $openInvalid(creating ? asked : "must specify at least one of read, write, or append access");
  }
  const access = o.append ? (o.read ? O_RDWR : O_WRONLY) | O_APPEND : o.read && o.write ? O_RDWR : o.write ? O_WRONLY : O_RDONLY;
  if ((!o.write && !o.append && creating) || (o.append && o.truncate && !o.createNew)) return $openInvalid(asked);
  const creation = o.createNew ? O_CREAT | O_EXCL : (o.create ? O_CREAT : 0) | (o.truncate ? O_TRUNC : 0);
  return $fsTry(() => {
    const fd = fs.openSync(path, access | creation, 0o666);
    if (o.append) $appending.add(fd);
    return $opened(fd);
  });
}

// `n` bytes read into `buf` from `at`, at the file's position, which moves
// past them. One that can't seek, a pipe's, reads where it is.
function $fileReadInto(f, buf, at, n) {
  const fs = $nodeFs("`std::fs::File`");
  const bytes = new Uint8Array(n);
  let read;
  try {
    read = fs.readSync(f.fd, bytes, 0, n, f.pos);
  } catch (e) {
    if (e?.code !== "ESPIPE") throw e;
    read = fs.readSync(f.fd, bytes, 0, n, null);
  }
  for (let i = 0; i < read; i++) buf[at + i] = bytes[i];
  f.pos += BigInt(read);
  return read;
}
// What std's `buffer_capacity_required` says is left: its length past its
// position, or none where it can't say.
function $fileLeft(f) {
  try {
    const size = BigInt($nodeFs("`std::fs::File`").fstatSync(f.fd).size);
    return size > f.pos ? Number(size - f.pos) : 0;
  } catch {
    return 0;
  }
}
// Whether the file can't seek, a pipe or a socket, whose `lseek` is `ESPIPE`.
function $unseekable(f) {
  const stat = $nodeFs("`std::fs::File`").fstatSync(f.fd);
  return stat.isFIFO() || stat.isSocket();
}

// `impl Read for File` and `impl Seek for File`: std's, each a call of the
// OS's at the file's position; what `Read` provides, `read_exact`, std's
// default over its `read`.
const $fileReader = {
  Read: () => $fileReader,
  read: (r, buf) => $fsTry(() => $fileReadInto(r.value, buf, 0, buf.length)),
  readExact: (r, buf) => $defaultReadExact($fileReader, r, buf),
  // Room for what's left first, `try_reserve`, as std's does, then all of it.
  readToEnd(r, v) {
    const f = r.value;
    const len = v.length;
    $reserve(v, $fileLeft(f), 1);
    const read = $fsTry(() => {
      const chunk = new Array(8192);
      let all = 0;
      while (true) {
        const n = $fileReadInto(f, chunk, 0, chunk.length);
        if (n === 0) return all;
        for (let i = 0; i < n; i++) v.push(chunk[i]);
        all += n;
      }
    });
    return $grewUnsized(v, len, read);
  },
  // What's read kept where it's UTF-8, as `append_to_string` keeps it.
  readToString: (r, s) => $appendToString(s, (bytes) => $fileReader.readToEnd(r, bytes)),
  // `lseek`'s: `Start(n)` is `n`, `End(n)` and `Current(n)` from its length or
  // its position; `EINVAL` where that's below 0 or past an `off_t`.
  seek(r, style) {
    const f = r.value;
    return $fsTry(() => {
      if ($unseekable(f)) throw $osFailure("ESPIPE", 29);
      const fs = $nodeFs("`std::fs::File`");
      const base = style.TAG === "Start" ? 0n : style.TAG === "End" ? BigInt(fs.fstatSync(f.fd).size) : f.pos;
      const n = base + style._0;
      if (n < 0n || n > 0x7fffffffffffffffn) throw $osFailure("EINVAL", 22);
      f.pos = n;
      return n;
    });
  },
  streamPosition: (r) => $fileReader.seek(r, { TAG: "Current", _0: 0n }),
  seekRelative: (r, offset) => $defaultSeekRelative($fileReader, r, offset),
};

// `impl Write for File`'s `write`: `bytes` written at its position, which
// moves past them; of one opened to append, at its end, where it's then.
// Node writes at a position below 2^53 alone, a `number`'s.
function $fileWrite(w, bytes) {
  const f = w.value;
  return $fsTry(() => {
    const fs = $nodeFs("`std::fs::File`");
    const data = Uint8Array.from(bytes);
    if ($appending.has(f.fd)) {
      const n = fs.writeSync(f.fd, data, 0, data.length, null);
      f.pos = BigInt(fs.fstatSync(f.fd).size);
      return n;
    }
    if (f.pos > BigInt(Number.MAX_SAFE_INTEGER)) {
      throw $unsupported("rust-js does not support a write to a file past 2^53 bytes, where Node writes none (#1156)");
    }
    let n;
    try {
      n = fs.writeSync(f.fd, data, 0, data.length, Number(f.pos));
    } catch (e) {
      if (e?.code !== "ESPIPE") throw e;
      n = fs.writeSync(f.fd, data, 0, data.length, null);
    }
    f.pos += BigInt(n);
    return n;
  });
}
// `impl Write for File`: its `write`, and what `Write` provides over it,
// std's; a file's writes aren't buffered, so `flush` does nothing.
const $fileWriter = {
  write: $fileWrite,
  writeAll: (w, bytes) => $ioWriteAll($fileWrite)(w, bytes),
  writeFmt: (w, text) => $ioWriteFmt($fileWrite)(w, text),
  flush: () => {},
};

// `fs::read(path)`: its bytes, in a `Vec` of its length's capacity, as std's
// `try_with_capacity` of its metadata's is.
function $fsRead(path) {
  const fs = $nodeFs("`std::fs::read`");
  return $fsTry(() => {
    const bytes = fs.readFileSync(path);
    const v = $withCapacity(bytes.length);
    for (const byte of bytes) v.push(byte);
    return v;
  });
}
// `fs::read_to_string(path)`: its text, `InvalidData` where it isn't UTF-8.
function $fsReadToString(path) {
  const fs = $nodeFs("`std::fs::read_to_string`");
  const read = $fsTry(() => Array.from(fs.readFileSync(path)));
  if (read.TAG === "Err") return read;
  if ($utf8Check(read._0) !== undefined) return { TAG: "Err", _0: $invalidUtf8() };
  return { TAG: "Ok", _0: $utf8Decode(read._0) };
}
// `fs::write(path, bytes)`: `File::create(path)`, then all of `bytes`.
function $fsWrite(path, bytes) {
  const fs = $nodeFs("`std::fs::write`");
  $fsThrow(() => fs.writeFileSync(path, Uint8Array.from(bytes)));
}
// `fs::remove_file(path)`.
function $fsRemoveFile(path) {
  const fs = $nodeFs("`std::fs::remove_file`");
  $fsThrow(() => fs.unlinkSync(path));
}
