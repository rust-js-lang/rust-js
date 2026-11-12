// Mutations of src/runtime/readers.js (#1156).
import type { Mutation } from "../../mutations";

const slice = ["test/corpus.test.ts", "-t", "^io_read_slice.rs$"];
const cursor = ["test/corpus.test.ts", "-t", "^io_cursor.rs$"];
const bufRead = ["test/corpus.test.ts", "-t", "^io_buf_read.rs$"];
const generic = ["test/corpus.test.ts", "-t", "^io_read_generic.rs$"];
const deque = ["test/corpus.test.ts", "-t", "^io_read_deque.rs$"];
const emptyRepeat = ["test/corpus.test.ts", "-t", "^io_read_empty_and_repeat.rs$"];
const chainTake = ["test/corpus.test.ts", "-t", "^io_read_chain_take_bytes.rs$"];
const readerDefaults = ["test/lowering-readers.test.ts"];
const toEndCapacity =["test/corpus.test.ts", "-t", "^io_read_to_end_capacity.rs$"];
const bufReader = ["test/corpus.test.ts", "-t", "^io_buf_reader.rs$"];

export const mutations: Mutation[] = [
  {
    name: "deque-read-exact-keeps-deque",
    breaks: "a `VecDeque<u8>`'s `read_exact` that can't fill its buffer leaves the deque as it was, where std's empties it (#1156)",
    file: "src/runtime/readers.js",
    find: "      d.length = 0;\n      throw $readExactEof();\n",
    replace: "      throw $readExactEof();\n",
    tests: deque,
  },
  {
    name: "deque-read-to-string-keeps-bad-text",
    breaks: "a `VecDeque<u8>`'s `read_to_string` of what isn't UTF-8 leaves it in the deque, where std's `read_to_end` took it first (#1156)",
    file: "src/runtime/readers.js",
    find: "    const all = d.slice();\n    d.length = 0;\n    if ($utf8Check(all) !== undefined) return { TAG: \"Err\", _0: $invalidUtf8() };\n",
    replace: "    const all = d.slice();\n    if ($utf8Check(all) !== undefined) return { TAG: \"Err\", _0: $invalidUtf8() };\n    d.length = 0;\n",
    tests: deque,
  },
  {
    name: "deque-consume-unchecked",
    breaks: "a `VecDeque<u8>`'s `consume` past its end takes what there is, where std's `drain(..amt)` panics (#1156)",
    file: "src/runtime/readers.js",
    find: "    $checkRange(0, amt, r.value.length);\n",
    replace: "",
    tests: deque,
  },
  {
    name: "slice-read-exact-keeps-slice",
    breaks: "a `&[u8]`'s `read_exact` that can't fill its buffer leaves the slice as it was, where std's empties it",
    file: "src/runtime/readers.js",
    find: "      r.value = $part(r.value, r.value.length);\n      throw $readExactEof();\n",
    replace: "      throw $readExactEof();\n",
    tests: slice,
  },
  {
    name: "slice-read-to-string-takes-bad-text",
    breaks: "a `&[u8]`'s `read_to_string` of what isn't UTF-8 takes it anyway",
    file: "src/runtime/readers.js",
    find: "    const all = r.value;\n    if ($utf8Check(all) !== undefined) return { TAG: \"Err\", _0: $invalidUtf8() };\n",
    replace: "    const all = r.value;\n    if ($utf8Check(all) !== undefined) {\n      r.value = $part(all, all.length);\n      return { TAG: \"Err\", _0: $invalidUtf8() };\n    }\n",
    tests: slice,
  },
  {
    name: "read-exact-eof-message",
    breaks: "a `read_exact` past the end says other words than std's",
    file: "src/runtime/readers.js",
    find: "\"failed to fill whole buffer\"",
    replace: "\"unexpected end of file\"",
    tests: slice,
  },
  {
    name: "slice-consume-past-end-quiet",
    breaks: "a `&[u8]`'s `consume` past its end takes what's there, where std's slicing panics",
    file: "src/runtime/readers.js",
    find: "    r.value = $slice(r.value, amt);\n",
    replace: "    r.value = $part(r.value, amt);\n",
    tests: slice,
  },
  {
    name: "cursor-read-exact-eof-stays",
    breaks: "a `Cursor`'s `read_exact` that can't fill its buffer leaves its position, where std's puts it at its end",
    file: "src/runtime/readers.js",
    find: "        r.value.pos = BigInt(bytes(r.value).length);\n        throw $readExactEof();\n",
    replace: "        throw $readExactEof();\n",
    tests: cursor,
  },
  {
    name: "cursor-position-never-wraps",
    breaks: "a `Cursor`'s position moved past `u64::MAX` is past it, where std's build wraps",
    file: "src/runtime/readers.js",
    find: "    c.pos = BigInt.asUintN(64, c.pos + BigInt(n));\n",
    replace: "    c.pos = c.pos + BigInt(n);\n",
    tests: cursor,
  },
  {
    name: "cursor-seek-below-zero",
    breaks: "a `Cursor`'s `seek` to before its start moves it there, where std's is an error",
    file: "src/runtime/readers.js",
    find: "      if (n < 0n || n > 0xffffffffffffffffn) {\n",
    replace: "      if (n > 0xffffffffffffffffn) {\n",
    tests: cursor,
  },
  {
    name: "cursor-past-end-reads-its-end",
    breaks: "a `Cursor` past its end reads from its last byte, not nothing",
    file: "src/runtime/readers.js",
    find: "    return $part(all, c.pos < BigInt(all.length) ? Number(c.pos) : all.length);\n",
    replace: "    return $part(all, c.pos < BigInt(all.length) ? Number(c.pos) : all.length - 1);\n",
    tests: cursor,
  },
  {
    name: "append-to-string-keeps-bad-line",
    breaks: "`read_line` of what isn't UTF-8 adds it to the `String`, lossily",
    file: "src/runtime/readers.js",
    find: "  if ($utf8Check(bytes) !== undefined) {\n    return read.TAG === \"Err\" ? read : { TAG: \"Err\", _0: $invalidUtf8() };\n  }\n",
    replace: "  if ($utf8Check(bytes) !== undefined) {\n    buf.value += new TextDecoder().decode(Uint8Array.from(bytes));\n    return read.TAG === \"Err\" ? read : { TAG: \"Err\", _0: $invalidUtf8() };\n  }\n",
    tests: bufRead,
  },
  {
    name: "mut-reader-reads-the-box",
    breaks: "a `&mut` to a reader, `R = &mut c`, is read as the reader itself, not through the box it's given in",
    file: "src/runtime/readers.js",
    find: "    fillBuf: (r) => reader.fillBuf(r.value),\n",
    replace: "    fillBuf: (r) => reader.fillBuf(r),\n",
    tests: bufRead,
  },
  {
    name: "dyn-reader-reads-the-box",
    breaks: "a `dyn BufRead`'s dictionary reads the box it's given as the pair",
    file: "src/runtime/readers.js",
    find: "  fillBuf: (r) => r.value.impl.fillBuf(r.value),\n",
    replace: "  fillBuf: (r) => r.impl.fillBuf(r),\n",
    tests: bufRead,
  },
  {
    name: "dyn-reader-read-exact-of-the-box",
    breaks: "a `Box<dyn Read>` given to generic code's `read_exact` is read as the pair, not through the box it's in",
    file: "src/runtime/readers.js",
    find: "  readExact: (r, buf) => r.value.impl.readExact(r.value, buf),\n",
    replace: "  readExact: (r, buf) => r.impl.readExact(r, buf),\n",
    tests: generic,
  },
  {
    name: "empty-read-exact-of-nothing-fails",
    breaks: "an `io::empty()`'s `read_exact` of an empty buffer is `UnexpectedEof`, where std's is `Ok(())` (#1156)",
    file: "src/runtime/readers.js",
    find: "    if (buf.length !== 0) throw $readExactEof();\n",
    replace: "    throw $readExactEof();\n",
    tests: emptyRepeat,
  },
  {
    name: "repeat-read-fills-nothing",
    breaks: "an `io::repeat(b)`'s `read` says it read the buffer's length but leaves it as it was (#1156)",
    file: "src/runtime/readers.js",
    find: "  read(r, buf) {\n    buf.fill(r.value.byte);\n",
    replace: "  read(r, buf) {\n",
    tests: emptyRepeat,
  },
  {
    name: "repeat-read-to-end-ends",
    breaks: "an `io::repeat(b)`'s `read_to_end` reads nothing and is `Ok(0)`, where std's is out of memory (#1156)",
    file: "src/runtime/readers.js",
    find: '  readToEnd: () => ({ TAG: "Err", _0: new $IoError("OutOfMemory") }),\n',
    replace: '  readToEnd: () => ({ TAG: "Ok", _0: 0 }),\n',
    tests: emptyRepeat,
  },
  {
    name: "slice-read-to-end-reserves-nothing",
    breaks: "a `&[u8]`'s `read_to_end` grows the `Vec` by each push, its capacity its length, where std's `try_reserve(len)` first gives 8 of 2 (#1338)",
    file: "src/runtime/readers.js",
    find: "    const all = r.value;\n    $reserve(v, all.length, 1);\n",
    replace: "    const all = r.value;\n",
    tests: toEndCapacity,
  },
  {
    name: "deque-read-to-end-reserves-nothing",
    breaks: "a `VecDeque<u8>`'s `read_to_end` grows the `Vec` by each push, where std's `try_reserve(len)` first gives 8 of 2 (#1338)",
    file: "src/runtime/readers.js",
    find: "    $reserve(v, n, 1);\n",
    replace: "",
    tests: toEndCapacity,
  },
  {
    name: "cursor-read-to-end-reserves-nothing",
    breaks: "a `Cursor`'s `read_to_end` grows the `Vec` by each push, where std's `try_reserve(len)` first gives 8 of 3 (#1338)",
    file: "src/runtime/readers.js",
    find: "      $reserve(v, from.length, 1);\n",
    replace: "",
    tests: toEndCapacity,
  },
  {
    name: "read-byte-reads-two",
    breaks: "`Bytes::next`'s `$readByte` reads a two-byte buffer, taking two bytes of each step, where std reads one (#1156)",
    file: "src/runtime/readers.js",
    find: "  const byte = [0];\n",
    replace: "  const byte = [0, 0];\n",
    tests: chainTake,
  },
  {
    name: "reader-defaults-read-exact-short-quiet",
    breaks: "a `Chain`'s or a `Take`'s `read_exact` that can't fill its buffer is `Ok`, where std's `default_read_exact` is `UnexpectedEof` (#1156)",
    file: "src/runtime/readers.js",
    find: "  if (at < buf.length) throw $readExactEof();\n",
    replace: "",
    tests: chainTake,
  },
  {
    name: "reader-defaults-over-its-own-read-to-end",
    breaks: "std's default `read_to_end` replaces what a `Chain`'s impl writes itself, each source's own, which reserves first (#1156, #1338)",
    file: "src/runtime/readers.js",
    find: "    ...withReadBuf,\n  };\n  $readerDefaulted.set(reader, made);\n",
    replace: "    ...withReadBuf,\n  };\n  Object.assign(made, { readToEnd });\n  $readerDefaulted.set(reader, made);\n",
    tests: readerDefaults,
  },
  {
    name: "buf-read-never-bypasses",
    breaks: "a `BufReader`'s `read` as large as its buffer, where nothing's buffered, reads through its buffer, where std's goes past it to its inner (#1156)",
    file: "src/runtime/readers.js",
    find: "      if (b.pos === b.filled && buf.length >= b.buf.length) {\n",
    replace: "      if (false) {\n",
    tests: bufReader,
  },
  {
    name: "buf-read-bypasses-a-short-read",
    breaks: "a `BufReader`'s `read` shorter than its buffer by one goes past it, where std's reads through it: its capacity is the least a bypass takes (#1156)",
    file: "src/runtime/readers.js",
    find: "buf.length >= b.buf.length) {\n",
    replace: "buf.length + 1 >= b.buf.length) {\n",
    tests: bufReader,
  },
  {
    name: "buf-fill-buf-always-refills",
    breaks: "a `BufReader`'s `fill_buf` reads its inner again where it has bytes buffered, where std's gives them (#1156)",
    file: "src/runtime/readers.js",
    find: "      if (b.pos >= b.filled) {\n        const read = R.read(inner(b), b.buf);",
    replace: "      if (true) {\n        const read = R.read(inner(b), b.buf);",
    tests: bufReader,
  },
  {
    name: "buf-fill-buf-reads-half",
    breaks: "a `BufReader`'s `fill_buf` asks its inner for half its buffer, where std's `read_buf` asks for all of it (#1156)",
    file: "src/runtime/readers.js",
    find: "        const read = R.read(inner(b), b.buf);\n",
    replace: "        const read = R.read(inner(b), $view(b.buf, 0, Math.ceil(b.buf.length / 2)));\n",
    tests: bufReader,
  },
  {
    name: "buf-consume-unbounded",
    breaks: "a `BufReader`'s `consume` past what's buffered goes past it, where std's stops at its end (#1156)",
    file: "src/runtime/readers.js",
    find: "      b.pos = Math.min((b.pos + amt) >>> 0, b.filled);\n",
    replace: "      b.pos = (b.pos + amt) >>> 0;\n",
    tests: bufReader,
  },
  {
    name: "buf-read-exact-leaves-pos",
    breaks: "a `BufReader`'s `read_exact` of what's buffered takes it without moving past it (#1156)",
    file: "src/runtime/readers.js",
    find: "        b.pos += buf.length;\n        return;\n",
    replace: "        return;\n",
    tests: bufReader,
  },
  {
    name: "buf-read-to-end-drops-buffered",
    breaks: "a `BufReader`'s `read_to_end` leaves out what's buffered, where std's adds it first (#1156)",
    file: "src/runtime/readers.js",
    find: "      for (let i = 0; i < held.length; i++) v.push(held[i]);\n",
    replace: "",
    tests: bufReader,
  },
  {
    name: "buf-read-to-end-keeps-buffered",
    breaks: "a `BufReader`'s `read_to_end` leaves what it added buffered, where std's discards it (#1156)",
    file: "src/runtime/readers.js",
    find: "      discard(b);\n      const read = R.readToEnd(inner(b), v);",
    replace: "      const read = R.readToEnd(inner(b), v);",
    tests: bufReader,
  },
  {
    name: "buf-read-to-end-reserves-nothing",
    breaks: "a `BufReader`'s `read_to_end` doesn't reserve room for what's buffered first, as std's `try_reserve` does (#1338)",
    file: "src/runtime/readers.js",
    find: "      $reserve(v, held.length, 1);\n      for (let i = 0; i < held.length; i++) v.push(held[i]);\n",
    replace: "      for (let i = 0; i < held.length; i++) v.push(held[i]);\n",
    tests: bufReader,
  },
  {
    name: "buf-read-to-string-into-text-unchecked",
    breaks: "a `BufReader`'s `read_to_string` into a `String` that has text takes what isn't UTF-8, where std's is `InvalidData` (#1156)",
    file: "src/runtime/readers.js",
    find: "      if ($utf8Check(bytes) !== undefined) return { TAG: \"Err\", _0: $invalidUtf8() };\n      s.value += $utf8Decode(bytes);\n      return { TAG: \"Ok\", _0: bytes.length };",
    replace: "      s.value += $utf8Decode(bytes);\n      return { TAG: \"Ok\", _0: bytes.length };",
    tests: bufReader,
  },
  {
    name: "buf-seek-current-ignores-buffer",
    breaks: "a `BufReader`'s `seek(Current(n))` moves its inner `n`, where std's counts from where the caller reads, less what's buffered (#1156)",
    file: "src/runtime/readers.js",
    find: "        const offset = style._0 - remainder;\n",
    replace: "        const offset = style._0;\n",
    tests: bufReader,
  },
  {
    name: "buf-seek-keeps-buffer",
    breaks: "a `BufReader`'s `seek` keeps what's buffered, where std's discards it (#1156)",
    file: "src/runtime/readers.js",
    find: "      discard(b);\n      return { TAG: \"Ok\", _0: result };\n",
    replace: "      return { TAG: \"Ok\", _0: result };\n",
    tests: bufReader,
  },
  {
    name: "buf-seek-overflow-moves-inner-once",
    breaks: "a `BufReader`'s `seek(Current(n))` of an `n` so far back its offset overflows seeks once, where std's moves back what's buffered first (#1156)",
    file: "src/runtime/readers.js",
    find: "        if (offset >= -(2n ** 63n)) {\n",
    replace: "        if (true) {\n",
    tests: bufReader,
  },
  {
    name: "buf-stream-position-ignores-buffer",
    breaks: "a `BufReader`'s `stream_position` is its inner's, where std's is less what's buffered (#1156)",
    file: "src/runtime/readers.js",
    find: "      return { TAG: \"Ok\", _0: position._0 - remainder };\n",
    replace: "      return { TAG: \"Ok\", _0: position._0 };\n",
    tests: bufReader,
  },
  {
    name: "buf-stream-position-underflow-quiet",
    breaks: "a `BufReader`'s `stream_position` of an inner behind what's buffered is a number, where std's `expect` panics (#1156)",
    file: "src/runtime/readers.js",
    find: "      if (position._0 < remainder) {\n",
    replace: "      if (false) {\n",
    tests: bufReader,
  },
  {
    name: "buf-seek-relative-always-seeks",
    breaks: "a `BufReader`'s `seek_relative` always seeks its inner, where std's stays within what's buffered where it can (#1156)",
    file: "src/runtime/readers.js",
    find: "      const b = r.value;\n      const pos = BigInt(b.pos);\n",
    replace: "      const b = r.value;\n      const pos = BigInt(b.pos);\n      if (true) return $defaultSeekRelative(reader, r, offset);\n",
    tests: bufReader,
  },
  {
    name: "buf-seek-relative-forward-past-filled",
    breaks: "a `BufReader`'s `seek_relative` forward stays within what's buffered up to a byte past it (#1156)",
    file: "src/runtime/readers.js",
    find: "      } else if (pos + offset <= BigInt(b.filled)) {\n",
    replace: "      } else if (pos + offset <= BigInt(b.filled) + 1n) {\n",
    tests: bufReader,
  },
  {
    name: "buf-seek-relative-back-past-start",
    breaks: "a `BufReader`'s `seek_relative` back past its buffer's start stays in it, where std's seeks its inner (#1156)",
    file: "src/runtime/readers.js",
    find: "        if (pos >= back) {\n",
    replace: "        if (pos + 1n >= back) {\n",
    tests: bufReader,
  },
  {
    name: "read-to-end-unreserved",
    breaks: "a `Take`'s `read_to_end` reads into fewer than 32 spare bytes, never `try_reserve(32)` first, so its capacity isn't std's (#1340)",
    file: "src/runtime/readers.js",
    find: "    if (spare() < $PROBE_SIZE) {\n      $reserve(buf, $PROBE_SIZE, 1);\n",
    replace: "    if (false) {\n      $reserve(buf, $PROBE_SIZE, 1);\n",
    tests: ["test/corpus.test.ts", "-t", "^read_to_end_capacity_of_std_readers.rs$"],
  },
  {
    name: "chain-read-buf-second-at-once",
    breaks: "a `Chain`'s `read_buf` reads its second at once, never its first, so what it reads isn't std's (#1340)",
    file: "src/runtime/readers.js",
    find: "    if (!chain.doneFirst) {\n      const oldLen = buf.filled;\n",
    replace: "    if (false) {\n      const oldLen = buf.filled;\n",
    tests: ["test/corpus.test.ts", "-t", "^read_to_end_capacity_of_std_readers.rs$"],
  },
  {
    name: "read-to-end-exact-fit-unprobed",
    breaks: "a `Vec` whose room a crate's reader filled exactly isn't probed again before it's grown, so the reader sees a read native Rust doesn't make (#1297)",
    file: "src/runtime/readers.js",
    find: "    if (spare() < $PROBE_SIZE && $capacity(buf) === startCap) {\n",
    replace: "    if (false) {\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^read_of_a_crates_own_reader.rs$"
    ]
  },
  {
    name: "read-to-end-max-read-size-undoubled",
    breaks: "a crate's reader that fills each read of `read_to_end` is asked no more than 8192 bytes a read, where std's `max_read_size` doubles (#1297)",
    file: "src/runtime/readers.js",
    find: "      else if (bytesRead === maxReadSize) maxReadSize *= 2;\n",
    replace: "",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^read_of_a_crates_own_reader.rs$"
    ]
  },
  {
    name: "read-to-end-reads-unseen",
    breaks: "a crate's reader read into a `Vec` whose capacity rust-js didn't keep is read in 8192-byte reads, unlike std's, where it's refused (#1297)",
    file: "src/runtime/readers.js",
    find: "    if (reader.readsSeen) {\n",
    replace: "    if (false) {\n",
    tests: [
      "test/lowering-readers.test.ts"
    ]
  },
  {
    name: "append-to-string-fresh-ignored",
    breaks: "a `String` proved fresh-empty is read into as one of a capacity not known, so a crate's reader read into it is refused (#1297)",
    file: "src/runtime/readers.js",
    find: "  const bytes = fresh ? [] : $untracked([]);\n",
    replace: "  const bytes = $untracked([]);\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^read_of_a_crates_own_reader.rs$"
    ]
  },
  {
    name: "append-to-string-fresh-assumed",
    breaks: "a `String` of a capacity no JS string carries is read into as one of none, so a crate's reader sees reads unlike std's, where it's refused (#1261, #1297)",
    file: "src/runtime/readers.js",
    find: "  const bytes = fresh ? [] : $untracked([]);\n",
    replace: "  const bytes = [];\n",
    tests: [
      "test/lowering-readers.test.ts"
    ]
  },
  {
    name: "reads-seen-of-sources-none",
    breaks: "a `Take` of a crate's reader read into a `Vec` whose capacity rust-js didn't keep reads it in 8192-byte reads, where it's refused (#1297)",
    file: "src/runtime/readers.js",
    find: "  return Object.assign(readBuf, { readsSeen: sources.some((source) => source.readsSeen === true) });\n",
    replace: "  return readBuf;\n",
    tests: [
      "test/lowering-readers.test.ts"
    ]
  },
  {
    name: "reader-defaults-reads-unseen",
    breaks: "a crate's reader's dictionary says its reads are no one's, so read into a capacity not known it's read in 8192-byte reads, where it's refused (#1297)",
    file: "src/runtime/readers.js",
    find: "  const readsSeen = readBuf === undefined || readBuf.readsSeen === true;\n",
    replace: "  const readsSeen = readBuf !== undefined && readBuf.readsSeen === true;\n",
    tests: [
      "test/lowering-readers.test.ts"
    ]
  },
  {
    name: "seek-defaults-position-moves",
    breaks: "a crate's own `Seek`'s `stream_position` seeks one past where it is, as `seek(Current(1))` (#1489)",
    file: "src/runtime/readers.js",
    find: "    streamPosition: (r) => seeker.seek(r, { TAG: \"Current\", _0: 0n }),\n",
    replace: "    streamPosition: (r) => seeker.seek(r, { TAG: \"Current\", _0: 1n }),\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^readers_of_the_crates_own_and_boxed.rs$"
    ]
  },
];
