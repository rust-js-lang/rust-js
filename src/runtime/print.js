
const $printed = ["", ""];
function $print(text) {
  $write(text, 0);
}
function $eprint(text) {
  $write(text, 1);
}
function $write(text, error) {
  const stream = globalThis.process?.[error ? "stderr" : "stdout"];
  if (stream?.write) {
    stream.write(text);
    return;
  }
  const log = error ? console.error : console.log;
  const lines = ($printed[error] + text).split("\n");
  const rest = lines.pop();
  for (const line of lines) log(line);
  // What's left unended is written when this task ends, so none is lost.
  if (rest !== "" && $printed[error] === "") {
    queueMicrotask(() => {
      if ($printed[error] !== "") log($printed[error]);
      $printed[error] = "";
    });
  }
  $printed[error] = rest;
}
// std's stdout `LineWriter` (#1490, ADR 1130): what's written while a line
// is formatted held till its newline, each whole line then written at once,
// so what a writer prints as it formats lands inside the line formatted so
// far, `a inner 1`, as Rust's does, and standard error, written at once,
// comes before the line it interrupts. Where there's no stream, each line is
// `console.log`'s, and what's left unended at a task's end is written then.
let $lineHeld = "";
// What each writer that may print, formatting now, writes its text to: the
// line, `$out`, of a `println!` (`$printing`), or apart from it, `undefined`,
// of a `format!` made as one formats (`$formatting`).
const $printFrames = [];
function $out(text) {
  $lineHeld += text;
  const end = $lineHeld.lastIndexOf("\n");
  if (end < 0) return;
  const lines = $lineHeld.slice(0, end + 1);
  $lineHeld = $lineHeld.slice(end + 1);
  $print(lines);
}
// A `println!` whose formatting may print, `write` its pieces written
// through the line. What's left unended, a `print!`'s or a panic's
// part-way through, is written as `print!` writes it, once no line is
// formatted, so what's printed after comes after it.
function $printing(write) {
  $printFrames.push($out);
  try {
    write();
  } finally {
    $printFrames.pop();
    if ($printFrames.length === 0 && $lineHeld !== "") {
      const rest = $lineHeld;
      $lineHeld = "";
      $print(rest);
    }
  }
}
// What a writer that may print writes its text to: the line, as it formats
// into it, so its text comes before what it prints after, and it gives back
// nothing more; else a string of its own, which it gives back.
function $sink() {
  if ($printFrames.at(-1) === $out) return { write: $out, done: () => "" };
  let text = "";
  return {
    write: (more) => {
      text += more;
    },
    done: () => text,
  };
}
// A writer's text made as a line is formatted, for anything but that line,
// `format!`'s or `to_string()`'s: made apart from it.
function $formatting(make) {
  $printFrames.push(undefined);
  try {
    return make();
  } finally {
    $printFrames.pop();
  }
}
// Bytes written to a standard stream (ADR 0392), and how many: Node's, as
// they are; where there's none, the text they are, a character split
// between two writes kept till its end comes.
const $decoders = [];
function $printBytes(bytes, error) {
  const stream = globalThis.process?.[error ? "stderr" : "stdout"];
  if (stream?.write) stream.write(Uint8Array.from(bytes));
  else $write(($decoders[error] ??= new TextDecoder()).decode(Uint8Array.from(bytes), { stream: true }), error);
  return bytes.length;
}
// `io::Write`'s dictionaries (ADR 0392): of a standard stream, and of a
// `Vec<u8>`, given in a box as a generic `&mut` is, whose bytes are added to
// it. A write never fails: `write` is `Ok` of all it's given, and the others
// nothing (ADR 0132).
const $stdoutWriter = {
  write: (_, bytes) => ({ TAG: "Ok", _0: $printBytes(bytes, 0) }),
  writeAll: (_, bytes) => void $printBytes(bytes, 0),
  writeFmt: (_, text) => $print(text),
  flush: () => {},
};
const $stderrWriter = {
  write: (_, bytes) => ({ TAG: "Ok", _0: $printBytes(bytes, 1) }),
  writeAll: (_, bytes) => void $printBytes(bytes, 1),
  writeFmt: (_, text) => $eprint(text),
  flush: () => {},
};
const $vecWriter = {
  write: (w, bytes) => ($extend(w.value, bytes), { TAG: "Ok", _0: bytes.length }),
  writeAll: (w, bytes) => $extend(w.value, bytes),
  writeFmt: (w, text) => $writeText(w.value, text),
  flush: () => {},
};
// std's `Write for &mut [u8]`, given in a box, `w.value` the slice: its bytes
// written at its front, as many as it has room for, and the slice re-pointed
// past them, as std's takes the rest, `*self = &mut take(self)[n..]`; its
// `write_all` std's, a write of what fits and `WriteZero` past it (#1282).
const $sliceWrite = (w, bytes) => {
  const room = w.value;
  const n = Math.min(room.length, bytes.length);
  for (let i = 0; i < n; i++) room[i] = bytes[i];
  w.value = $view(room, n);
  return { TAG: "Ok", _0: n };
};
const $sliceWriter = {
  write: $sliceWrite,
  writeAll: (w, bytes) => $ioWriteAll($sliceWrite)(w, bytes),
  writeFmt: (w, text) => $ioWriteFmt($sliceWrite)(w, text),
  flush: () => {},
};
// A `Box<dyn Write>`'s, given in a box: the pair it holds, written through.
const $dynWriter = {
  write: (w, bytes) => w.value.impl.write(w.value, bytes),
  writeAll: (w, bytes) => w.value.impl.writeAll(w.value, bytes),
  writeFmt: (w, text) => w.value.impl.writeFmt(w.value, text),
  flush: (w) => w.value.impl.flush(w.value),
};
// `io::Write`'s `write_all` of a writer of the crate's, std's: its `write`
// till all's written. One it writes none of, or fails, an `io::Error`,
// which JS has no value of, is rust-js's error, as no write of rust-js's
// fails (ADR 0392).
function $ioWriteAll(write) {
  return (w, bytes) => {
    for (let at = 0; at < bytes.length; ) {
      const written = write(w, at === 0 ? bytes : bytes.slice(at));
      // std's: an interrupted write again, another's error thrown (ADR 0462).
      if (written.TAG === "Err") {
        if (written._0.kind === "Interrupted") continue;
        throw written._0;
      }
      if (written._0 === 0) throw new $IoError("WriteZero", undefined, "failed to write whole buffer");
      at += written._0;
    }
  };
}
// Its `write_fmt`: the text's UTF-8 bytes, all written.
function $ioWriteFmt(write) {
  const all = $ioWriteAll(write);
  return (w, text) => all(w, Array.from(new TextEncoder().encode(text)));
}
// `write!(v, ..)` of a `Vec<u8>`: its text's UTF-8 bytes added.
function $writeText(v, text) {
  $extend(v, new TextEncoder().encode(text));
}
