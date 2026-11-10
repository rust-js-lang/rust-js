// Mutations of src/runtime/readers.js (#1156).
import type { Mutation } from "../../mutations";

const slice = ["test/corpus.test.ts", "-t", "^io_read_slice.rs$"];
const cursor = ["test/corpus.test.ts", "-t", "^io_cursor.rs$"];
const bufRead = ["test/corpus.test.ts", "-t", "^io_buf_read.rs$"];
const generic = ["test/corpus.test.ts", "-t", "^io_read_generic.rs$"];
const deque = ["test/corpus.test.ts", "-t", "^io_read_deque.rs$"];

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
];
