// `io::Read`'s, `io::BufRead`'s and `io::Seek`'s dictionaries of std's
// byte sources (#1156): each method one its impl in std writes itself,
// std's body over the source, given in a box as a generic `&mut` is,
// `{ value }`. What the traits provide, `read_line` or `rewind`, is std's
// own body, compiled into `@rust-js/std`'s `io.js` and given the dictionary.
// An `io::Result<()>`'s `Err` is thrown (ADR 0462), another's a value.

// std's `Error::READ_EXACT_EOF` and `Error::INVALID_UTF8`, word for word.
function $readExactEof() {
  return new $IoError("UnexpectedEof", undefined, "failed to fill whole buffer");
}
function $invalidUtf8() {
  return new $IoError("InvalidData", undefined, "stream did not contain valid UTF-8");
}

// `buf[..n].copy_from_slice(&from[..n])`.
function $readInto(buf, from, n) {
  for (let i = 0; i < n; i++) buf[i] = from[i];
}

// `impl Read for &[u8]` and `impl BufRead for &[u8]`: what's read is taken
// from the slice's front, `*self = rest`.
const $sliceReader = {
  // Its `BufRead`'s supertrait's, `Read`'s: itself.
  Read: () => $sliceReader,
  read(r, buf) {
    const amt = Math.min(buf.length, r.value.length);
    $readInto(buf, r.value, amt);
    r.value = $part(r.value, amt);
    return { TAG: "Ok", _0: amt };
  },
  // One that can't fill `buf` empties the slice first, and fills nothing.
  readExact(r, buf) {
    if (buf.length > r.value.length) {
      r.value = $part(r.value, r.value.length);
      throw $readExactEof();
    }
    $readInto(buf, r.value, buf.length);
    r.value = $part(r.value, buf.length);
  },
  readToEnd(r, v) {
    const all = r.value;
    for (let i = 0; i < all.length; i++) v.push(all[i]);
    r.value = $part(all, all.length);
    return { TAG: "Ok", _0: all.length };
  },
  // All of what's left is checked first: text that isn't UTF-8 takes nothing.
  readToString(r, s) {
    const all = r.value;
    if ($utf8Check(all) !== undefined) return { TAG: "Err", _0: $invalidUtf8() };
    s.value += $utf8Decode(all);
    r.value = $part(all, all.length);
    return { TAG: "Ok", _0: all.length };
  },
  fillBuf: (r) => ({ TAG: "Ok", _0: r.value }),
  // `*self = &self[amt..]`, which panics past its end.
  consume(r, amt) {
    r.value = $slice(r.value, amt);
  },
};

// `impl Read for VecDeque<u8>` and `impl BufRead`: what's read taken from
// its front, an array changed in place. Its `read` and `fill_buf`, which
// read its front slice alone, are refused where they're called (#1302).
const $dequeReader = {
  // One that can't fill `buf` empties the deque, and fills nothing.
  readExact(r, buf) {
    const d = r.value;
    if (buf.length > d.length) {
      d.length = 0;
      throw $readExactEof();
    }
    $readInto(buf, d, buf.length);
    d.splice(0, buf.length);
  },
  readToEnd(r, v) {
    const d = r.value;
    const n = d.length;
    for (let i = 0; i < n; i++) v.push(d[i]);
    d.length = 0;
    return { TAG: "Ok", _0: n };
  },
  // All of it is taken, as `read_to_end` takes it, then kept where it's
  // UTF-8: text that isn't adds nothing, the deque emptied all the same.
  readToString(r, s) {
    const d = r.value;
    const all = d.slice();
    d.length = 0;
    if ($utf8Check(all) !== undefined) return { TAG: "Err", _0: $invalidUtf8() };
    s.value += $utf8Decode(all);
    return { TAG: "Ok", _0: all.length };
  },
  // `self.drain(..amt)`, which panics past its end.
  consume(r, amt) {
    $checkRange(0, amt, r.value.length);
    r.value.splice(0, amt);
  },
};

// `impl Read for Empty`, `impl BufRead` and `impl Seek`: nothing is read,
// a `read_exact` of anything is past its end, and every position is 0.
const $emptyReader = {
  Read: () => $emptyReader,
  read: () => ({ TAG: "Ok", _0: 0 }),
  readExact(r, buf) {
    if (buf.length !== 0) throw $readExactEof();
  },
  readToEnd: () => ({ TAG: "Ok", _0: 0 }),
  readToString: () => ({ TAG: "Ok", _0: 0 }),
  fillBuf: () => ({ TAG: "Ok", _0: [] }),
  consume() {},
  readUntil: () => ({ TAG: "Ok", _0: 0 }),
  skipUntil: () => ({ TAG: "Ok", _0: 0 }),
  readLine: () => ({ TAG: "Ok", _0: 0 }),
  seek: () => ({ TAG: "Ok", _0: 0n }),
  streamPosition: () => ({ TAG: "Ok", _0: 0n }),
};

// `impl Read for Repeat`: every byte read is its `byte`, without end, so
// what reads to the end is out of memory.
const $repeatReader = {
  Read: () => $repeatReader,
  read(r, buf) {
    buf.fill(r.value.byte);
    return { TAG: "Ok", _0: buf.length };
  },
  readExact(r, buf) {
    buf.fill(r.value.byte);
  },
  readToEnd: () => ({ TAG: "Err", _0: new $IoError("OutOfMemory") }),
  readToString: () => ({ TAG: "Err", _0: new $IoError("OutOfMemory") }),
};

// `impl Read for Cursor<T>`, `impl BufRead` and `impl Seek`, of `T`'s
// `AsRef<[u8]>`: read at its position, `pos`, a `u64`, of its inner value's
// bytes, `TAsRef.asRef(inner)`; what's past its end is nothing. Its
// position moves as std's does, wrapping as std's build does.
function $cursorReader(TAsRef) {
  const bytes = (c) => TAsRef.asRef(c.inner);
  // `Cursor::split(self).1`: what's from its position, none past its end.
  const rest = (c) => {
    const all = bytes(c);
    return $part(all, c.pos < BigInt(all.length) ? Number(c.pos) : all.length);
  };
  const moved = (c, n) => {
    c.pos = BigInt.asUintN(64, c.pos + BigInt(n));
  };
  const reader = {
    Read: () => reader,
    read(r, buf) {
      const from = rest(r.value);
      const n = Math.min(buf.length, from.length);
      $readInto(buf, from, n);
      moved(r.value, n);
      return { TAG: "Ok", _0: n };
    },
    // One that can't fill `buf` puts the cursor at its end.
    readExact(r, buf) {
      const from = rest(r.value);
      if (buf.length > from.length) {
        r.value.pos = BigInt(bytes(r.value).length);
        throw $readExactEof();
      }
      $readInto(buf, from, buf.length);
      moved(r.value, buf.length);
    },
    readToEnd(r, v) {
      const from = rest(r.value);
      for (let i = 0; i < from.length; i++) v.push(from[i]);
      moved(r.value, from.length);
      return { TAG: "Ok", _0: from.length };
    },
    readToString(r, s) {
      const from = rest(r.value);
      if ($utf8Check(from) !== undefined) return { TAG: "Err", _0: $invalidUtf8() };
      s.value += $utf8Decode(from);
      moved(r.value, from.length);
      return { TAG: "Ok", _0: from.length };
    },
    fillBuf: (r) => ({ TAG: "Ok", _0: rest(r.value) }),
    consume(r, amt) {
      moved(r.value, amt);
    },
    // `SeekFrom::Start(n)` is `n`; `End(n)` and `Current(n)` are from its
    // length or its position, an error where that's below 0 or past `u64`.
    seek(r, style) {
      if (style.TAG === "Start") {
        r.value.pos = style._0;
        return { TAG: "Ok", _0: style._0 };
      }
      const base = style.TAG === "End" ? BigInt(bytes(r.value).length) : r.value.pos;
      const n = base + style._0;
      if (n < 0n || n > 0xffffffffffffffffn) {
        return {
          TAG: "Err",
          _0: new $IoError("InvalidInput", undefined, "invalid seek to a negative or overflowing position"),
        };
      }
      r.value.pos = n;
      return { TAG: "Ok", _0: n };
    },
    streamPosition: (r) => ({ TAG: "Ok", _0: r.value.pos }),
  };
  return reader;
}

// A reader's dictionary through the box generic code is given a `&mut`
// to it in, `R = &mut c`, which std's `impl Read for &mut R` reads through:
// the reader's own, of what's in the box, as `$mutWriter`'s.
function $mutReader(reader) {
  const through = {
    Read: () => through,
    read: (r, buf) => reader.read(r.value, buf),
    readExact: (r, buf) => reader.readExact(r.value, buf),
    readToEnd: (r, v) => reader.readToEnd(r.value, v),
    readToString: (r, s) => reader.readToString(r.value, s),
    fillBuf: (r) => reader.fillBuf(r.value),
    consume: (r, amt) => reader.consume(r.value, amt),
    seek: (r, style) => reader.seek(r.value, style),
    streamPosition: (r) => reader.streamPosition(r.value),
  };
  return through;
}

// A `dyn Read`'s, `dyn BufRead`'s or `dyn Seek`'s, and a `Box` of one's,
// given the pair, `{ value, impl }`, in the box generic code is given:
// its impl's own, given the pair, whose `value` a box's is, as `read_line`
// of a `dyn BufRead` asks it (#1156).
const $dynReader = {
  Read: () => $dynReader,
  read: (r, buf) => r.value.impl.read(r.value, buf),
  readExact: (r, buf) => r.value.impl.readExact(r.value, buf),
  readToEnd: (r, v) => r.value.impl.readToEnd(r.value, v),
  readToString: (r, s) => r.value.impl.readToString(r.value, s),
  fillBuf: (r) => r.value.impl.fillBuf(r.value),
  consume: (r, amt) => r.value.impl.consume(r.value, amt),
  seek: (r, style) => r.value.impl.seek(r.value, style),
  streamPosition: (r) => r.value.impl.streamPosition(r.value),
};

// alloc's `append_to_string(buf, f)`, what `read_line` adds to a `String`
// by: `f` given an array to add bytes to, and what it added kept where
// it's UTF-8; else nothing is, and `Err(INVALID_UTF8)` where `f` gave `Ok`.
function $appendToString(buf, f) {
  const bytes = [];
  const read = f(bytes);
  if ($utf8Check(bytes) !== undefined) {
    return read.TAG === "Err" ? read : { TAG: "Err", _0: $invalidUtf8() };
  }
  buf.value += $utf8Decode(bytes);
  return read;
}
