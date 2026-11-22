
// What stdout holds of a line left unended, every copy of the runtime's
// one (`$stdoutKey`): `held`, what's written to a stream and held
// (`$stdoutWrite`); `printed`, where there's no stream, each stream's
// text since its last newline; `exits`, whether the process's exit writes
// what's held; and while any is held, `log`, the host's `console.log`,
// which `$heldLog` wraps.
// Found when first asked, as marks.js, which declares its key, may come
// after this in the runtime.
function $stdout() {
  return (globalThis[$stdoutKey] ??= { held: [], printed: ["", ""], exits: false, log: undefined, wrapper: undefined });
}
function $print(text) {
  $write(text, 0);
}
function $eprint(text) {
  $write(text, 1);
}
function $write(text, error) {
  if (error) $flushAhead();
  const stream = globalThis.process?.[error ? "stderr" : "stdout"];
  if (stream?.write) {
    if (error) $hostWrite(2, text);
    else $stdoutWrite(text);
    return;
  }
  const log = error ? console.error : ($stdout().log ?? console.log);
  const lines = ($stdout().printed[error] + text).split("\n");
  const rest = lines.pop();
  for (const line of lines) log(line);
  // What's left unended is written when this task ends, so none is lost.
  if (rest !== "" && $stdout().printed[error] === "") {
    queueMicrotask(() => {
      if ($stdout().printed[error] !== "") (error ? console.error : ($stdout().log ?? console.log))($stdout().printed[error]);
      $stdout().printed[error] = "";
      if (!error) $unwrapLog();
    });
  }
  $stdout().printed[error] = rest;
  if (error) return;
  if (rest === "") $unwrapLog();
  else $wrapLog();
}
// While stdout holds a line left unended, the host's `console.log` is
// wrapped, so a whole line any crate prints by it, `println!`'s
// (ADR 0087), comes after what's held, as one `LineWriter` writes every
// crate's natively (`$heldLog`, #2320). Unwrapped once nothing's held.
function $wrapLog() {
  if ($stdout().wrapper !== undefined) return;
  const log = globalThis.console?.log;
  if (typeof log !== "function") return;
  const wrapper = function (...args) {
    return $heldLog(log, this, args);
  };
  try {
    console.log = wrapper;
  } catch {}
  if (console.log !== wrapper) throw $holdRefused("its console.log can't be wrapped");
  $stdout().log = log;
  $stdout().wrapper = wrapper;
}
function $unwrapLog() {
  if ($stdout().wrapper === undefined) return;
  // A spy put on it meanwhile, a test's, is left as it is.
  if (console.log === $stdout().wrapper) console.log = $stdout().log;
  $stdout().log = undefined;
  $stdout().wrapper = undefined;
}
// A host that can't keep what's held before a line refuses, a run-time
// error, as which host runs the JS is decided only when it runs.
function $holdRefused(why) {
  return new Error(`rust-js: stdout holds a line left unended, and on this host ${why}, so a line printed now would come out before it`);
}
// A `console.log` while stdout holds text: what's held written first, by
// the host's own write, which its `console.log` comes after (`$hostWrite`,
// #2347), then the line. Where there's no stream, a browser's console,
// whose every `console.log` is a line, the held text joined to a line of one
// string, every `println!` rust-js writes; values can't follow it on its
// line, and are refused.
function $heldLog(log, self, args) {
  if (globalThis.process?.stdout?.write) {
    $flushStdout();
    return log.apply(self, args);
  }
  if (args.length !== 1 || typeof args[0] !== "string") throw $holdRefused("a console.log of values can't follow it but apart");
  const held = $stdout().printed[0];
  $stdout().printed[0] = "";
  $unwrapLog();
  return log.call(self, held + args[0]);
}
// What std's stdout `LineWriter` holds of a line left unended, a
// `print!`'s or a `write_all`'s, text or bytes, as each was written, till
// the line's newline, a `flush()` or the process's exit, std's cleanup at
// its end, where it's written; standard error, written at once, comes
// before it (#2319). Where there's no stream, `$stdout().printed` holds it so.
// `chunk`, text or bytes, to stdout: what's held and the chunk up to its
// last newline written, what's after held.
function $stdoutWrite(chunk) {
  const end = typeof chunk === "string" ? chunk.lastIndexOf("\n") : chunk.lastIndexOf(10);
  if (end < 0) {
    if (chunk.length === 0) return;
    $stdout().held.push(chunk);
    $wrapLog();
    if (!$stdout().exits) {
      $stdout().exits = true;
      globalThis.process.once("exit", () => {
        globalThis.process.stdout._handle?.setBlocking?.(true);
        $flushStdout();
      });
    }
    return;
  }
  for (const held of $stdout().held) $hostWrite(1, held);
  $stdout().held = [];
  $unwrapLog();
  $hostWrite(1, chunk.slice(0, end + 1));
  if (end + 1 < chunk.length) $stdoutWrite(chunk.slice(end + 1));
}
// `io::stdout().flush()`: what's held, written.
function $flushStdout() {
  const stream = globalThis.process?.stdout;
  if (!stream?.write) return;
  for (const held of $stdout().held) $hostWrite(1, held);
  $stdout().held = [];
  $unwrapLog();
}
// The host's write of `chunk`, text or bytes, to standard stream `fd`,
// 1 or 2, by measurement each host's own (#2347). Bun's `console.log` and
// `console.error` write the descriptor themselves, at once, and its
// streams buffer, so through a pipe a stream's write could come out after a
// `console.log` made after it: Bun's is the descriptor's, `fs.writeSync`,
// as its console's are. Node's `console.log` writes through the stream, so
// a descriptor's write could overtake what it's queued there: Node's is the
// stream's. A browser has neither, and never comes here.
function $hostWrite(fd, chunk) {
  const fs = globalThis.Bun === undefined ? undefined : globalThis.process.getBuiltinModule("node:fs");
  if (fs === undefined) {
    globalThis.process[fd === 1 ? "stdout" : "stderr"].write(chunk);
    return;
  }
  $writeAll((bytes) => fs.writeSync(fd, bytes), chunk);
}
// `chunk` written whole by `write`, which writes some of the bytes it's
// given and says how many, or throws: a descriptor that would block,
// `EAGAIN`, tried again after `pause`'s wait, not at once, up to
// `limit` times in a row, past which it's thrown.
function $writeAll(write, chunk, pause = $pause, limit = 60_000) {
  let bytes = typeof chunk === "string" ? new TextEncoder().encode(chunk) : chunk;
  let waits = 0;
  while (bytes.length > 0) {
    try {
      bytes = bytes.subarray(write(bytes));
      waits = 0;
    } catch (error) {
      if (error?.code !== "EAGAIN" || ++waits > limit) throw error;
      pause(1);
    }
  }
}
// A wait of `ms` milliseconds that doesn't spin: `Atomics.wait` on a
// slot no one wakes.
function $pause(ms) {
  Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, ms);
}
// std's stdout `LineWriter` (#1490, ADR 1130): what's written while a line
// is formatted held till its newline, each whole line then written at once,
// so what a writer prints as it formats lands inside the line formatted so
// far, `a inner 1`, as Rust's does, and standard error, written at once,
// comes before the line it interrupts. Where there's no stream, each line is
// `console.log`'s, and what's left unended at a task's end is written then.
let $lineHeld = "";
// Of a line formatted, the bytes written ahead of the text that holds
// them, at a write to standard error (`$flushAhead`), which that text,
// reaching the line, leaves out.
let $lineAhead = "";
// Of each `{:#?}` builder's step formatting now, the depth of the captures
// whose text it writes through std's `PadAdapter`, and the adapter.
const $padAt = new Map();
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
// writer writes the line, doesn't go through the adapter (#1521). Of a part
// written in pieces, a builder's field's name, then its value (#1533), the
// adapter's state, whether it's at a line's start, kept between them.
function $padded(text, adapter = { onNewline: true }) {
  let padded = "";
  for (const [at, part] of text.split($markOf().split).entries()) {
    if (at % 2 === 1) {
      padded += part;
      continue;
    }
    for (const line of part.split(/(?<=\n)/)) {
      if (line === "") continue;
      if (adapter.onNewline) padded += "    ";
      padded += line;
      adapter.onNewline = line.endsWith("\n");
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
// The text of the writers above `depth` a panic left unfinished, closed:
// what they wrote, in the order they wrote it, for the writer at `depth` to
// write as its own, a builder's through `{:#?}`'s adapter (#1533).
function $unfinishedAbove(depth) {
  if ($captures.length <= depth) return "";
  $closeAbove(depth + 1);
  return $captures.pop().text;
}
// Text for the line: printed where no line is formatted, as `print!`
// prints, so a `format!`'s writer's print comes before what's printed
// next (#1521); else held till its newline.
function $toLine(text) {
  text = $unmarked(`${text}`);
  // What's written ahead is of the open writers' text, which reaches the
  // line whole, the outermost's, so it starts with it.
  if ($lineAhead !== "") {
    if (!text.startsWith($lineAhead)) throw new Error("rust-js: stdout's line was written ahead of other text");
    text = text.slice($lineAhead.length);
    $lineAhead = "";
  }
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
// `text`'s prints put back in their places, as `$unmarked` puts them, each
// kept for when the text reaches the line.
function $unmarkedSoFar(text) {
  if ($markForm === undefined || !text.includes($markForm.open)) return text;
  return text.replace($markForm.pattern, (_, mark) => $unmarkedSoFar($marks.get(Number(mark))));
}
// Before a write to standard error, what std's stdout `LineWriter` holds
// no longer, as Rust's would have written it by now: each line ended so far
// of the line formatted, its text held and each open writer's so far, with
// what it printed in its place and a `{:#?}` part through its adapter
// (#1490). Written now, and left out when that text reaches the line.
function $flushAhead() {
  if ($printFrames.length === 0 || ($lineHeld === "" && $captures.length === 0)) return;
  let open = "";
  for (let depth = $captures.length - 1; depth >= 0; depth--) {
    open = $captures[depth].text + open;
    const adapter = $padAt.get(depth);
    if (adapter !== undefined) open = $padded(open, { ...adapter });
  }
  let pending = $lineHeld + $unmarkedSoFar(open);
  if ($lineAhead !== "") {
    if (!pending.startsWith($lineAhead)) throw new Error("rust-js: stdout's line was written ahead of other text");
    pending = pending.slice($lineAhead.length);
  }
  const end = pending.lastIndexOf("\n");
  if (end < 0) return;
  const ended = pending.slice(0, end + 1);
  $lineAhead += ended.slice($lineHeld.length);
  $lineHeld = "";
  $print(ended);
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
    // What was written ahead of a line a panic stopped, which never reaches
    // it (#1761).
    if ($printFrames.length === 0) $lineAhead = "";
  }
}
// What a writer that may print writes its text to: in a line, a capture of
// its own, its text given back by `done()`, with what it printed marked in
// its place; else a string of its own, which it gives back. And a handle on
// its text as a `String`'s `&mut` is, which a struct that holds the
// writer's `Formatter` and generic `fmt::Write` code append to,
// `h.value += s`: each `value` set is written, and what's read is nothing,
// so an append writes what it adds (#1637).
function $sink() {
  // Of a writer shown into a writer by `write!`, that writer's sink, whose
  // calls are its `write_str` and `write_char` (#1652, `$writingInto`).
  if ($writerArmed !== undefined) {
    const into = $writerArmed;
    $writerArmed = undefined;
    return $sinkOf(into.write, () => "", into.writeChar, into.armed);
  }
  if ($printFrames.at(-1) !== $out) {
    let text = "";
    return $sinkOf(
      (more) => {
        text += more;
      },
      () => text,
    );
  }
  const capture = { text: "" };
  const depth = $captures.push(capture);
  return $sinkOf(
    (more) => {
      $closeAbove(depth);
      capture.text += more;
    },
    () => {
      $closeAbove(depth);
      if ($captures.at(-1) === capture) $captures.pop();
      return capture.text;
    },
  );
}
// `armed`: whether it's a writer's, each piece a call of the writer's
// (`$writeBy`), not text being made; a sink armed with another's is what
// that one is.
function $sinkOf(write, done, writeChar = write, armed = false) {
  return {
    write,
    writeChar,
    done,
    armed,
    get value() {
      return "";
    },
    set value(more) {
      write(more);
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
  if (error) $flushAhead();
  const stream = globalThis.process?.[error ? "stderr" : "stdout"];
  if (stream?.write && error) $hostWrite(2, Uint8Array.from(bytes));
  else if (stream?.write) $stdoutWrite(Uint8Array.from(bytes));
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
  flush: () => $flushStdout(),
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
