// std's `Hasher`'s provided methods, each over `write` or another of them, as
// std's are (ADR 0463): an integer's bytes, little-endian as `wasm32`'s, a
// `usize`'s four; a signed one's as its unsigned one's; text's bytes and 0xff.
class $Hasher {
  write_u8(n) {
    this.write([n]);
  }
  write_u16(n) {
    this.write($toBytes(n, 2, true));
  }
  write_u32(n) {
    this.write($toBytes(n, 4, true));
  }
  write_u64(n) {
    this.write($toBytes(n, 8, true));
  }
  write_u128(n) {
    this.write($toBytes(n, 16, true));
  }
  write_usize(n) {
    this.write($toBytes(n, 4, true));
  }
  write_i8(n) {
    this.write_u8(n & 0xff);
  }
  write_i16(n) {
    this.write_u16(n & 0xffff);
  }
  write_i32(n) {
    this.write_u32(n >>> 0);
  }
  write_i64(n) {
    this.write_u64(BigInt.asUintN(64, n));
  }
  write_i128(n) {
    this.write_u128(BigInt.asUintN(128, n));
  }
  write_isize(n) {
    this.write_usize(n >>> 0);
  }
  write_length_prefix(n) {
    this.write_usize(n);
  }
  write_str(s) {
    this.write(Array.from(new TextEncoder().encode(s)));
    this.write_u8(0xff);
  }
}
const $hasherProvided = Object.getOwnPropertyNames($Hasher.prototype).filter((name) => name !== "constructor");

// A crate's `Hasher`'s dictionary, the provided methods its impl doesn't
// write filled in, each `$Hasher`'s over the dictionary's own entries.
function $hasher(dictionary) {
  for (const name of $hasherProvided) {
    dictionary[name] ??= (state, ...args) => {
      const self = Object.create($Hasher.prototype);
      for (const other of ["write", ...$hasherProvided]) {
        if (other !== name) self[other] = (...given) => dictionary[other](state, ...given);
      }
      self[name](...args);
    };
  }
  return dictionary;
}

// `Hash`'s provided `hash_slice`, of a dictionary's `hash`: each item's.
function $hashSlice(hash) {
  return (items, state, H) => {
    for (const item of items) hash(item, state, H);
  };
}

// SipHash, over 64-bit BigInts: `DefaultHasher`'s, SipHash-1-3 of zero keys,
// and the deprecated `SipHasher`'s, SipHash-2-4 (ADR 0463). The same bytes
// give the same `finish()` as std's, however they're split into writes.
class $SipHasher extends $Hasher {
  constructor(compressions, finalizations, k0 = 0n, k1 = 0n) {
    super();
    this.compressions = compressions;
    this.finalizations = finalizations;
    this.v0 = k0 ^ 0x736f6d6570736575n;
    this.v1 = k1 ^ 0x646f72616e646f6dn;
    this.v2 = k0 ^ 0x6c7967656e657261n;
    this.v3 = k1 ^ 0x7465646279746573n;
    this.length = 0;
    this.tail = 0n;
  }
  rounds(count) {
    const rotl = (x, b) => BigInt.asUintN(64, (x << b) | (x >> (64n - b)));
    for (let i = 0; i < count; i++) {
      this.v0 = BigInt.asUintN(64, this.v0 + this.v1);
      this.v1 = rotl(this.v1, 13n) ^ this.v0;
      this.v0 = rotl(this.v0, 32n);
      this.v2 = BigInt.asUintN(64, this.v2 + this.v3);
      this.v3 = rotl(this.v3, 16n) ^ this.v2;
      this.v0 = BigInt.asUintN(64, this.v0 + this.v3);
      this.v3 = rotl(this.v3, 21n) ^ this.v0;
      this.v2 = BigInt.asUintN(64, this.v2 + this.v1);
      this.v1 = rotl(this.v1, 17n) ^ this.v2;
      this.v2 = rotl(this.v2, 32n);
    }
  }
  write(bytes) {
    for (const byte of bytes) {
      this.tail |= BigInt(byte) << BigInt(8 * (this.length % 8));
      this.length++;
      if (this.length % 8 === 0) {
        this.v3 ^= this.tail;
        this.rounds(this.compressions);
        this.v0 ^= this.tail;
        this.tail = 0n;
      }
    }
  }
  finish() {
    const last = (BigInt(this.length & 0xff) << 56n) | this.tail;
    const { v0, v1, v2, v3 } = this;
    this.v3 ^= last;
    this.rounds(this.compressions);
    this.v0 ^= last;
    this.v2 ^= 0xffn;
    this.rounds(this.finalizations);
    const hash = this.v0 ^ this.v1 ^ this.v2 ^ this.v3;
    Object.assign(this, { v0, v1, v2, v3 });
    return hash;
  }
  clone() {
    return Object.assign(Object.create($SipHasher.prototype), this);
  }
}
// A std hasher's dictionary, of its `&mut`, `{ value }`: its own methods.
const $sipHasher = Object.fromEntries(
  ["write", "finish", ...$hasherProvided].map((name) => [name, (hasher, ...args) => hasher.value[name](...args)]),
);
