
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
  write: (w, bytes) => (w.value.push(...bytes), { TAG: "Ok", _0: bytes.length }),
  writeAll: (w, bytes) => void w.value.push(...bytes),
  writeFmt: (w, text) => $writeText(w.value, text),
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
  v.push(...new TextEncoder().encode(text));
}
