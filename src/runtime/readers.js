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

// core's `BorrowedBuf` and its `BorrowedCursor` (core/src/io/borrowed_buf.rs)
// as `default_read_to_end` reads into one, `{ buf, filled, init }`: `buf`
// its bytes, `filled` how many are read, `init` its one flag of whether all
// of it is initialized. A reader's `read_buf(cursor)` fills it from
// `filled` and may set `init` (#1340).
const $readBufDone = { TAG: "Ok", _0: undefined };
// `cursor.capacity()`: what's left to fill.
function $unfilled(cursor) {
  return cursor.buf.length - cursor.filled;
}
// `cursor.append(&from[..n])`: copied, filled, its `init` as it was.
function $cursorAppend(cursor, from, n) {
  for (let i = 0; i < n; i++) cursor.buf[cursor.filled + i] = from[i];
  cursor.filled += n;
}
// alloc's `default_read_buf(|b| self.read(b), cursor)`, a reader's whose own
// isn't written: `cursor.ensure_init()`, then its `read` of all of it.
function $defaultReadBuf(reader, r, cursor) {
  // Flags, not a state: core's `BorrowedBuf` has one `init` flag, which this mirrors.
  cursor.init = true;
  const read = reader.read(r, $view(cursor.buf, cursor.filled));
  if (read.TAG === "Err") return read;
  cursor.filled += read._0;
  return $readBufDone;
}
// A reader's `read_buf`: its own where std writes one, else the default.
function $readBufOf(reader) {
  return reader.readBuf ?? ((r, cursor) => $defaultReadBuf(reader, r, cursor));
}

// `Seek`'s provided `seek_relative(offset)`: `seek(SeekFrom::Current(offset))`
// of `reader`, its `Err` thrown, an `io::Result<()>`'s (ADR 0462).
function $defaultSeekRelative(reader, r, offset) {
  const seek = reader.seek(r, { TAG: "Current", _0: offset });
  if (seek.TAG === "Err") throw seek._0;
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
  // Its `read_buf`: what fits appended, `cursor.append(a)`, which leaves the
  // cursor's initialized flag as it was.
  readBuf(r, cursor) {
    const amt = Math.min($unfilled(cursor), r.value.length);
    $cursorAppend(cursor, r.value, amt);
    r.value = $part(r.value, amt);
    return $readBufDone;
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
  // Room for all of it first, `try_reserve(len)`, as std's does, so its
  // capacity is native Rust's (#1338).
  readToEnd(r, v) {
    const all = r.value;
    $reserve(v, all.length, 1);
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
    $reserve(v, n, 1);
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
  // `fn read_buf(&mut self, _cursor) { Ok(()) }`.
  readBuf: () => $readBufDone,
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
  seekRelative: (r, offset) => $defaultSeekRelative($emptyReader, r, offset),
};

// `impl Read for Repeat`: every byte read is its `byte`, without end, so
// what reads to the end is out of memory.
const $repeatReader = {
  Read: () => $repeatReader,
  read(r, buf) {
    buf.fill(r.value.byte);
    return { TAG: "Ok", _0: buf.length };
  },
  // Its `read_buf`: all of what's left written, `write_filled`, and advanced
  // past, its `init` flag as it was.
  readBuf(r, cursor) {
    cursor.buf.fill(r.value.byte, cursor.filled);
    cursor.filled = cursor.buf.length;
    return $readBufDone;
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
    // Its `read_buf`: `Cursor::split(self).1`'s, a slice's, moved past.
    readBuf(r, cursor) {
      const from = rest(r.value);
      const n = Math.min($unfilled(cursor), from.length);
      $cursorAppend(cursor, from, n);
      moved(r.value, n);
      return $readBufDone;
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
      $reserve(v, from.length, 1);
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
    seekRelative: (r, offset) => $defaultSeekRelative(reader, r, offset),
  };
  return reader;
}

// `impl Read for BufReader<R>`, `impl BufRead` and `impl Seek` (#1156): each
// method std's body word for word, over `{ inner, buf, pos, filled }`, std's
// fields flattened: `buf` is its buffer, `capacity()` long, of what its
// inner `read` wrote, `buf[pos..filled]` what's buffered. `R` is its
// inner's `Read`, or `Seek`, dictionary, given the inner in a box as a
// `&mut` is, which writes it back. `fill_buf`'s read is `read_buf`'s
// default, `read` of the whole buffer, as each reader std gives it does.
function $bufReader(R) {
  const inner = (b) => ({
    get value() {
      return b.inner;
    },
    set value(value) {
      b.inner = value;
    },
  });
  // `self.buffer()`: what's buffered.
  const buffered = (b) => $part(b.buf, b.pos, b.filled);
  const discard = (b) => {
    b.pos = 0;
    b.filled = 0;
  };
  const reader = {
    Read: () => reader,
    // Its inner's reads are a crate reader's to see where they are (#1297).
    readsSeen: R.readsSeen === true,
    // Past its buffer, a read as large goes to its inner at once.
    read(r, buf) {
      const b = r.value;
      if (b.pos === b.filled && buf.length >= b.buf.length) {
        discard(b);
        return R.read(inner(b), buf);
      }
      const rem = reader.fillBuf(r);
      if (rem.TAG === "Err") return rem;
      const n = Math.min(buf.length, rem._0.length);
      $readInto(buf, rem._0, n);
      reader.consume(r, n);
      return { TAG: "Ok", _0: n };
    },
    // Its `read_buf`: of a reader as large as its buffer with none buffered,
    // its inner's at once; else what's buffered, read as a slice is.
    readBuf(r, cursor) {
      const b = r.value;
      if (b.pos === b.filled && $unfilled(cursor) >= b.buf.length) {
        discard(b);
        return $readBufOf(R)(inner(b), cursor);
      }
      const rem = reader.fillBuf(r);
      if (rem.TAG === "Err") return rem;
      const n = Math.min($unfilled(cursor), rem._0.length);
      $cursorAppend(cursor, rem._0, n);
      reader.consume(r, n);
      return $readBufDone;
    },
    // What's buffered fills `buf` or `default_read_exact` reads it.
    readExact(r, buf) {
      const b = r.value;
      if (buf.length <= b.filled - b.pos) {
        for (let i = 0; i < buf.length; i++) buf[i] = b.buf[b.pos + i];
        b.pos += buf.length;
        return;
      }
      $defaultReadExact(reader, r, buf);
    },
    // What's buffered, then its inner's.
    readToEnd(r, v) {
      const b = r.value;
      const held = buffered(b);
      $reserve(v, held.length, 1);
      for (let i = 0; i < held.length; i++) v.push(held[i]);
      discard(b);
      const read = R.readToEnd(inner(b), v);
      return read.TAG === "Err" ? read : { TAG: "Ok", _0: (held.length + read._0) >>> 0 };
    },
    // Into an empty `String` it's `read_to_end`'s, into the `String`'s own
    // bytes, kept where it's UTF-8; into another, all of it is read first,
    // into a new `Vec`, and checked.
    readToString(r, s, fresh) {
      if (s.value === "") return $appendToString(s, (v) => reader.readToEnd(r, v), fresh);
      const bytes = [];
      const read = reader.readToEnd(r, bytes);
      if (read.TAG === "Err") return read;
      if ($utf8Check(bytes) !== undefined) return { TAG: "Err", _0: $invalidUtf8() };
      s.value += $utf8Decode(bytes);
      return { TAG: "Ok", _0: bytes.length };
    },
    // Its buffer's what's read, again where it's all consumed.
    fillBuf(r) {
      const b = r.value;
      if (b.pos >= b.filled) {
        const read = R.read(inner(b), b.buf);
        b.pos = 0;
        b.filled = read.TAG === "Ok" ? read._0 : 0;
        if (read.TAG === "Err") return read;
      }
      return { TAG: "Ok", _0: buffered(b) };
    },
    consume(r, amt) {
      const b = r.value;
      b.pos = Math.min((b.pos + amt) >>> 0, b.filled);
    },
    // Its inner is moved from where it's read to: what's buffered is read, so
    // `Current(n)` is `n` less what's left of it.
    seek(r, style) {
      const b = r.value;
      let result;
      if (style.TAG === "Current") {
        const remainder = BigInt(b.filled - b.pos);
        const offset = style._0 - remainder;
        if (offset >= -(2n ** 63n)) {
          const seek = R.seek(inner(b), { TAG: "Current", _0: offset });
          if (seek.TAG === "Err") return seek;
          result = seek._0;
        } else {
          const back = R.seek(inner(b), { TAG: "Current", _0: -remainder });
          if (back.TAG === "Err") return back;
          discard(b);
          const seek = R.seek(inner(b), style);
          if (seek.TAG === "Err") return seek;
          result = seek._0;
        }
      } else {
        const seek = R.seek(inner(b), style);
        if (seek.TAG === "Err") return seek;
        result = seek._0;
      }
      discard(b);
      return { TAG: "Ok", _0: result };
    },
    // Its inner's less what's buffered of it.
    streamPosition(r) {
      const b = r.value;
      const remainder = BigInt(b.filled - b.pos);
      const position = R.streamPosition(inner(b));
      if (position.TAG === "Err") return position;
      if (position._0 < remainder) {
        throw new Error("overflow when subtracting remaining buffer size from inner stream position");
      }
      return { TAG: "Ok", _0: position._0 - remainder };
    },
    // Within what's buffered, nothing is asked of its inner.
    seekRelative(r, offset) {
      const b = r.value;
      const pos = BigInt(b.pos);
      if (offset < 0n) {
        const back = BigInt.asUintN(64, -offset);
        if (pos >= back) {
          b.pos -= Number(back);
          return;
        }
      } else if (pos + offset <= BigInt(b.filled)) {
        b.pos += Number(offset);
        return;
      }
      $defaultSeekRelative(reader, r, offset);
    },
  };
  return reader;
}

// A reader's dictionary through the box generic code is given a `&mut`
// to it in, `R = &mut c`, which std's `impl Read for &mut R` reads through:
// the reader's own, of what's in the box, as `$mutWriter`'s.
function $mutReader(reader) {
  const through = {
    Read: () => through,
    readsSeen: reader.readsSeen === true,
    read: (r, buf) => reader.read(r.value, buf),
    readBuf: (r, cursor) => $readBufOf(reader)(r.value, cursor),
    readExact: (r, buf) => reader.readExact(r.value, buf),
    readToEnd: (r, v) => reader.readToEnd(r.value, v),
    readToString: (r, s, fresh) => reader.readToString(r.value, s, fresh),
    fillBuf: (r) => reader.fillBuf(r.value),
    consume: (r, amt) => reader.consume(r.value, amt),
    seek: (r, style) => reader.seek(r.value, style),
    streamPosition: (r) => reader.streamPosition(r.value),
    seekRelative: (r, offset) => reader.seekRelative(r.value, offset),
  };
  return through;
}

// A `dyn Read`'s, `dyn BufRead`'s or `dyn Seek`'s, and a `Box` of one's,
// given the pair, `{ value, impl }`, in the box generic code is given:
// its impl's own, given the pair, whose `value` a box's is, as `read_line`
// of a `dyn BufRead` asks it (#1156).
const $dynReader = {
  Read: () => $dynReader,
  // Its value's reader may be the crate's own, whose reads a `Take` or a
  // `Chain` of it, which asks this of its dictionary, can't know (#1297).
  readsSeen: true,
  read: (r, buf) => r.value.impl.read(r.value, buf),
  readBuf: (r, cursor) => $readBufOf(r.value.impl)(r.value, cursor),
  readExact: (r, buf) => r.value.impl.readExact(r.value, buf),
  readToEnd: (r, v) => r.value.impl.readToEnd(r.value, v),
  readToString: (r, s, fresh) => r.value.impl.readToString(r.value, s, fresh),
  fillBuf: (r) => r.value.impl.fillBuf(r.value),
  consume: (r, amt) => r.value.impl.consume(r.value, amt),
  seek: (r, style) => r.value.impl.seek(r.value, style),
  streamPosition: (r) => r.value.impl.streamPosition(r.value),
  seekRelative: (r, offset) => r.value.impl.seekRelative(r.value, offset),
};

// alloc's `append_to_string(buf, f)`, what `read_line` adds to a `String`
// by: `f` given an array to add bytes to, and what it added kept where
// it's UTF-8; else nothing is, and `Err(INVALID_UTF8)` where `f` gave `Ok`.
// The array is the `String`'s bytes, whose capacity a JS string doesn't
// carry (#1261), so not known; but of one `fresh`, `String::new()` the
// analysis proved nothing was done to, none, as a `Vec::new()`'s (#1297).
function $appendToString(buf, f, fresh) {
  const bytes = fresh ? [] : $untracked([]);
  const read = f(bytes);
  if ($utf8Check(bytes) !== undefined) {
    return read.TAG === "Err" ? read : { TAG: "Err", _0: $invalidUtf8() };
  }
  buf.value += $utf8Decode(bytes);
  return read;
}

// alloc's `inlined_slow_read_byte(reader)`, `Bytes::next`'s: one byte read
// by the reader's `read` of a one-byte buffer, as std's body does, an
// interrupted read tried again: none at its end, else the byte, or the
// error. A reader of the crate's own sees std's own calls of it (#1297).
function $readByte(r, RRead) {
  const byte = [0];
  while (true) {
    const read = RRead.read(r, byte);
    if (read.TAG === "Err") {
      if (read._0.kind === "Interrupted") continue;
      return read;
    }
    return read._0 === 0 ? undefined : { TAG: "Ok", _0: byte[0] };
  }
}

// A field of a struct std's body reads through `&mut`, given as a box,
// `{ value }`, that writes it back.
function $fieldBox(of, field) {
  return {
    get value() {
      return of[field];
    },
    set value(value) {
      of[field] = value;
    },
  };
}

// `impl Read for Take<T>`'s `read_buf` (alloc/src/io/util.rs), over `{
// inner, limit }`, `T` its inner's `Read` dictionary: none at its limit;
// past it, a cursor of its limit read, initialized as the whole was, whose
// being initialized after makes the whole so; else its inner's of the whole.
function $takeReadBuf(TRead) {
  const innerReadBuf = $readBufOf(TRead);
  return $readsSeenOf([TRead], (r, buf) => {
    const take = r.value;
    if (take.limit === 0n) return $readBufDone;
    if (take.limit < BigInt($unfilled(buf))) {
      const limit = Number(take.limit);
      const isInit = buf.init;
      const slicedBuf = { buf: $view(buf.buf, buf.filled, buf.filled + limit), filled: 0, init: false };
      // Flags, not a state: core's `BorrowedBuf` has one `init` flag, which this mirrors.
      if (isInit) slicedBuf.init = true;
      const result = innerReadBuf($fieldBox(take, "inner"), slicedBuf);
      const didInitUpToLimit = slicedBuf.init;
      const filled = slicedBuf.filled;
      if (didInitUpToLimit && !isInit) {
        buf.buf.fill(0, buf.filled + limit);
        // Flags, not a state: core's `BorrowedBuf` has one `init` flag, which this mirrors.
        buf.init = true;
      }
      buf.filled += filled;
      take.limit -= BigInt(filled);
      return result;
    }
    const written = buf.filled;
    const result = innerReadBuf($fieldBox(take, "inner"), buf);
    take.limit -= BigInt(buf.filled - written);
    return result;
  });
}

// `impl Read for Chain<T, U>`'s `read_buf`, over `{ first, second,
// doneFirst }`: its first's till that writes nothing, then its second's.
function $followedReadBuf(TRead, URead) {
  const firstReadBuf = $readBufOf(TRead);
  const secondReadBuf = $readBufOf(URead);
  return $readsSeenOf([TRead, URead], (r, buf) => {
    const chain = r.value;
    if ($unfilled(buf) === 0) return $readBufDone;
    if (!chain.doneFirst) {
      const oldLen = buf.filled;
      const read = firstReadBuf($fieldBox(chain, "first"), buf);
      if (read.TAG === "Err") return read;
      if (buf.filled !== oldLen) return $readBufDone;
      // Flags, not a state: std's `Chain`'s `done_first` field, as its body sets it.
      chain.doneFirst = true;
    }
    return secondReadBuf($fieldBox(chain, "second"), buf);
  });
}

// `readBuf`, a `Take`'s or a `Chain`'s of `sources`' dictionaries, which
// reads them: whether a reader of the crate's own is among them, whose
// reads it then sees (#1297).
function $readsSeenOf(sources, readBuf) {
  return Object.assign(readBuf, { readsSeen: sources.some((source) => source.readsSeen === true) });
}

const $DEFAULT_BUF_SIZE = 8 * 1024;
const $PROBE_SIZE = 32;

// alloc's `default_read_to_end(r, buf, size_hint)` (alloc/src/io/read.rs),
// line by line, `size_hint` none, as `Take`'s and `Chain`'s provided
// methods give it: a 32-byte probe read where fewer are spare, `try_reserve`
// of 32 where the spare room is less, then reads of its spare room, at
// most `max_read_size`, which doubles where a read fills it and is
// uncapped where the reader's `read_buf` leaves its buffer uninitialized
// (`$readBufOf`'s). So the capacity it leaves `buf` is std's (#1340). Of a
// `buf` whose capacity isn't known, its reads, and its growth not kept.
// Its proof: corpus case read_to_end_capacity_of_std_readers.
function $defaultReadToEnd(reader, r, buf, sizeHint) {
  if ($capacityUnknown(buf)) {
    // Its reads, of a capacity not known, a reader of the crate's own would
    // see unlike std's (#1297).
    if (reader.readsSeen) {
      throw $unsupported(
        "rust-js does not support reading a reader of the crate's own to its end into a `Vec` whose capacity it didn't keep, or a `String` not provably fresh-empty, whose capacity a JS string doesn't carry: the reader would see reads of that capacity (#1261)",
      );
    }
    return $readToEndUnkept(reader, r, buf);
  }
  const readBuf = $readBufOf(reader);
  const startLen = buf.length;
  const startCap = $capacity(buf);
  // Optionally limit the maximum bytes read on each iteration.
  let maxReadSize =
    sizeHint === undefined ? $DEFAULT_BUF_SIZE : Math.ceil((sizeHint + 1024) / $DEFAULT_BUF_SIZE) * $DEFAULT_BUF_SIZE;
  // Tracks how many bytes are initialized in the buffer.
  let initUntil = buf.length;

  const smallProbeRead = () => {
    const probe = new Array($PROBE_SIZE).fill(0);
    while (true) {
      const read = reader.read(r, probe);
      if (read.TAG === "Ok") {
        // `buf.extend_from_slice(&probe[..n])`.
        $reserve(buf, read._0, 1);
        for (let i = 0; i < read._0; i++) buf.push(probe[i]);
        return read;
      }
      if (read._0.kind === "Interrupted") continue;
      return read;
    }
  };
  const spare = () => $capacity(buf) - buf.length;

  // Avoid inflating empty/small vecs before we have determined that
  // there's anything to read.
  if ((sizeHint === undefined || sizeHint === 0) && spare() < $PROBE_SIZE) {
    const read = smallProbeRead();
    if (read.TAG === "Err") return read;
    if (read._0 === 0) return { TAG: "Ok", _0: 0 };
  }

  while (true) {
    if (spare() < $PROBE_SIZE && $capacity(buf) === startCap) {
      // The buffer might be an exact fit.
      const read = smallProbeRead();
      if (read.TAG === "Err") return read;
      if (read._0 === 0) return { TAG: "Ok", _0: buf.length - startLen };
      initUntil = buf.length;
      continue;
    }

    // Avoid unnecessarily short reads by ensuring there's at least
    // PROBE_SIZE space available.
    if (spare() < $PROBE_SIZE) {
      $reserve(buf, $PROBE_SIZE, 1);
      initUntil = buf.length;
    }

    const bufLen = initUntil > buf.length + $PROBE_SIZE ? initUntil - buf.length : Math.min(maxReadSize, spare());
    const wasInit = initUntil >= buf.length + bufLen;
    const readBuf_ = { buf: new Array(bufLen).fill(0), filled: 0, init: wasInit };

    let result;
    while (true) {
      result = readBuf(r, readBuf_);
      if (result.TAG === "Err" && result._0.kind === "Interrupted") continue;
      break;
    }

    const bytesRead = readBuf_.filled;
    const isInit = readBuf_.init;
    if (isInit) initUntil = buf.length + bufLen;
    for (let i = 0; i < bytesRead; i++) buf.push(readBuf_.buf[i]);

    // Now that all data is pushed to the vector, we can fail without data loss.
    if (result.TAG === "Err") return result;

    if (bytesRead === 0) return { TAG: "Ok", _0: buf.length - startLen };

    // Use heuristics to determine the max read size if no initial size hint
    // was provided.
    if (sizeHint === undefined) {
      if (!isInit) maxReadSize = Infinity;
      else if (bytesRead === maxReadSize) maxReadSize *= 2;
    }
  }
}

// What `default_read_to_end` reads of a `buf` whose capacity isn't known:
// what's there, by its reader's `read`, its growth not kept.
function $readToEndUnkept(reader, r, v) {
  const len = v.length;
  const chunk = new Array($DEFAULT_BUF_SIZE).fill(0);
  let all = 0;
  while (true) {
    const read = reader.read(r, chunk);
    if (read.TAG === "Err") {
      if (read._0.kind === "Interrupted") continue;
      return $grewUnsized(v, len, read);
    }
    if (read._0 === 0) return $grewUnsized(v, len, { TAG: "Ok", _0: all });
    for (let i = 0; i < read._0; i++) v.push(chunk[i]);
    all += read._0;
  }
}

// std's provided `read_exact`, `read_to_end` and `read_to_string` (#1156)
// of a reader of std's own whose impl doesn't write them, a `Chain`'s or a
// `Take`'s, given its `read_buf`, `$takeReadBuf`'s or `$followedReadBuf`'s:
// `default_read_exact`, `default_read_to_end` (`$defaultReadToEnd`) and
// `default_read_to_string`, what it writes itself kept. One dictionary of
// each reader's.
const $readerDefaulted = new WeakMap();

// std's `default_read_exact(reader, buf)`: `reader`'s `read` called until
// `buf` is full, an interrupted read tried again. One that can't fill `buf`
// leaves what it read in it, and `UnexpectedEof` is thrown.
function $defaultReadExact(reader, r, buf) {
  let at = 0;
  while (at < buf.length) {
    const read = reader.read(r, $view(buf, at));
    if (read.TAG === "Err") {
      if (read._0.kind === "Interrupted") continue;
      throw read._0;
    }
    if (read._0 === 0) break;
    at += read._0;
  }
  if (at < buf.length) throw $readExactEof();
}

function $readerDefaults(reader, readBuf) {
  let made = $readerDefaulted.get(reader);
  if (made !== undefined) return made;
  // A reader of the crate's own has no `read_buf` but the default, and sees
  // each read; a `Take` or a `Chain` sees what its sources do (#1297).
  const readsSeen = readBuf === undefined || readBuf.readsSeen === true;
  const withReadBuf = { ...reader, readBuf, readsSeen };
  const readToEnd = (r, v) => $defaultReadToEnd(withReadBuf, r, v, undefined);
  made = {
    readExact: (r, buf) => $defaultReadExact(reader, r, buf),
    readToEnd,
    // All of it read by `default_read_to_end`, not the reader's own, then
    // kept where it's UTF-8.
    readToString: (r, s, fresh) => $appendToString(s, (b) => readToEnd(r, b), fresh),
    ...withReadBuf,
  };
  $readerDefaulted.set(reader, made);
  return made;
}
