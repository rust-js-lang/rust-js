
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
// The text of each writer formatting in a line, made before it's written
// there (#1521): a `Vec`'s, an `Option`'s or a tuple's `Debug` is one
// expression, `items.map(fmt).join(", ")`, made whole before the line's
// next part, so a writer can't write to the line as it formats. What's
// printed meanwhile goes into the innermost one, at that point, as a mark
// (`$marked`), and is put back in its place as the text reaches the line
// (`$unmarked`): the line's bytes are the ones Rust's `LineWriter` holds,
// each print where it ran. A mark opens with a lone surrogate, which no
// Rust string holds, and this run's own random key, so a JS string given
// through interop, which may hold lone surrogates, isn't taken for one.
const $captures = [];
const $marks = new Map();
let $markCount = 0;
// The run's mark, `open`, and its patterns, made once a mark is first made.
let $markForm;
function $markOf() {
  if ($markForm === undefined) {
    const key = Array.from({ length: 8 }, () => String.fromCharCode(0xe000 + Math.floor(Math.random() * 0x1900)));
    const open = `\uDC00${key.join("")}`;
    $markForm = { open, pattern: new RegExp(`${open}(\\d+)\uDC01`, "g"), split: new RegExp(`(${open}\\d+\uDC01)`) };
  }
  return $markForm;
}
function $marked(text) {
  const mark = $markCount++;
  $marks.set(mark, `${text}`);
  return `${$markOf().open}${mark}\uDC01`;
}
function $unmarked(text) {
  if ($markForm === undefined || !text.includes($markForm.open)) return text;
  return text.replace($markForm.pattern, (_, mark) => {
    const printed = $marks.get(Number(mark));
    $marks.delete(Number(mark));
    return $unmarked(printed);
  });
}
// `text` as std's `PadAdapter` writes it, `{:#?}`'s parts': four spaces
// before each of its lines' first character, a blank line's too; a print's
// mark at a line's start stays before them, as the print, made before its
// writer writes the line, doesn't go through the adapter (#1521).
function $padded(text) {
  let padded = "";
  let lineStart = true;
  for (const [at, part] of text.split($markOf().split).entries()) {
    if (at % 2 === 1) {
      padded += part;
      continue;
    }
    for (const line of part.split(/(?<=\n)/)) {
      if (line === "") continue;
      if (lineStart) padded += "    ";
      padded += line;
      lineStart = line.endsWith("\n");
    }
  }
  return padded;
}
// The captures above `depth`, of writers a panic or an error left
// unfinished, each's text after what it was opened in: what they wrote,
// in the order they wrote it.
function $closeAbove(depth) {
  while ($captures.length > depth) {
    const left = $captures.pop();
    if ($captures.length > 0) $captures.at(-1).text += left.text;
    else $toLine(left.text);
  }
}
// Text for the line: printed where no line is formatted, as `print!`
// prints, so a `format!`'s writer's print comes before what's printed
// next (#1521); else held till its newline.
function $toLine(text) {
  text = $unmarked(`${text}`);
  if ($printFrames.length === 0) {
    $print(text);
    return;
  }
  $lineHeld += text;
  const end = $lineHeld.lastIndexOf("\n");
  if (end < 0) return;
  const lines = $lineHeld.slice(0, end + 1);
  $lineHeld = $lineHeld.slice(end + 1);
  $print(lines);
}
// A part of a line, or what a writer prints as it formats: into the
// innermost writer's text, as a mark, where one is formatting; else the
// line's.
function $out(text) {
  if ($captures.length > 0) $captures.at(-1).text += $marked(text);
  else $toLine(text);
}
// A `println!` whose formatting may print, `write` its pieces written
// through the line. What's left unended, a `print!`'s or a panic's
// part-way through, is written as `print!` writes it, once no line is
// formatted, so what's printed after comes after it; and what a writer a
// panic stopped wrote before it, in the line before it.
function $printing(write) {
  const depth = $captures.length;
  $printFrames.push($out);
  try {
    write();
  } finally {
    $closeAbove(depth);
    $printFrames.pop();
    if ($printFrames.length === 0 && $lineHeld !== "") {
      const rest = $lineHeld;
      $lineHeld = "";
      $print(rest);
    }
  }
}
// What a writer that may print writes its text to: in a line, a capture of
// its own, its text given back by `done()`, with what it printed marked in
// its place; else a string of its own, which it gives back.
function $sink() {
  if ($printFrames.at(-1) !== $out) {
    let text = "";
    return {
      write: (more) => {
        text += more;
      },
      done: () => text,
    };
  }
  const capture = { text: "" };
  const depth = $captures.push(capture);
  return {
    write: (more) => {
      $closeAbove(depth);
      capture.text += more;
    },
    done: () => {
      $closeAbove(depth);
      if ($captures.at(-1) === capture) $captures.pop();
      return capture.text;
    },
  };
}
// A writer's text made as a line is formatted, for anything but that line,
// `format!`'s or `to_string()`'s: made apart from it. What its writers
// printed, held while it's made, is written as `print!` writes it once no
// line is formatted, as `$printing` writes what it leaves unended, before
// what's printed next (#1534).
function $formatting(make) {
  $printFrames.push(undefined);
  try {
    return make();
  } finally {
    $printFrames.pop();
    if ($printFrames.length === 0 && $lineHeld !== "") {
      const rest = $lineHeld;
      $lineHeld = "";
      $print(rest);
    }
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
