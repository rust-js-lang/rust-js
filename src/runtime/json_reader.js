
// serde_json's reader (ADR 0078): its `Deserializer`, ported step for step,
// with its methods' names. It reads the text's UTF-8 bytes, as serde_json
// does, so a mistake is found at the same byte, with the same message, and
// its column counts bytes.
class $JsonReader extends $JsonDecoder {
  constructor(text) {
    super();
    this.bytes = new TextEncoder().encode(text);
    this.index = 0;
    this.remainingDepth = 128;
  }

  // The next byte, or -1 at the end.
  peek() {
    return this.index < this.bytes.length ? this.bytes[this.index] : -1;
  }

  next() {
    return this.index < this.bytes.length ? this.bytes[this.index++] : -1;
  }

  // `position_of_index`: the line, and the bytes before `index` on it.
  errorAt(index, message) {
    const start = index === 0 ? 0 : this.bytes.lastIndexOf(10, index - 1) + 1;
    let line = 1;
    for (let i = 0; i < start; i++) if (this.bytes[i] === 10) line++;
    return new $JsonError(message, line, index - start);
  }

  error(message) {
    return this.errorAt(this.index, message);
  }

  peekError(message) {
    return this.errorAt(Math.min(this.bytes.length, this.index + 1), message);
  }

  // `fix_position`: a visitor's message has no place, until it gets where
  // the reader is.
  fixPosition(e) {
    return e instanceof $JsonError && e.line === 0 ? this.error(e.message) : e;
  }

  end() {
    if (this.parseWhitespace() !== -1) throw this.peekError("trailing characters");
  }

  parseWhitespace() {
    for (;;) {
      const c = this.peek();
      if (c !== 32 && c !== 10 && c !== 9 && c !== 13) return c;
      this.index++;
    }
  }

  parseIdent(rest) {
    for (let i = 0; i < rest.length; i++) {
      const c = this.next();
      if (c === -1) throw this.error("EOF while parsing a value");
      if (c !== rest.charCodeAt(i)) throw this.error("expected ident");
    }
  }

  // What's there instead of `expected`, read past, in serde's words.
  peekInvalidType(expected) {
    const c = this.peek();
    let unexpected;
    if (c === 110) {
      this.index++;
      this.parseIdent("ull");
      unexpected = "null";
    } else if (c === 116) {
      this.index++;
      this.parseIdent("rue");
      unexpected = "boolean `true`";
    } else if (c === 102) {
      this.index++;
      this.parseIdent("alse");
      unexpected = "boolean `false`";
    } else if (c === 45) {
      this.index++;
      unexpected = $jsonUnexpectedNumber(this.parseInteger(false));
    } else if (c >= 48 && c <= 57) {
      unexpected = $jsonUnexpectedNumber(this.parseInteger(true));
    } else if (c === 34) {
      this.index++;
      unexpected = `string ${$debugStr(this.parseStr())}`;
    } else if (c === 91) {
      unexpected = "sequence";
    } else if (c === 123) {
      unexpected = "map";
    } else {
      return this.peekError("expected value");
    }
    return this.fixPosition($jsonError(`invalid type: ${unexpected}, expected ${expected}`));
  }

  // A number: `{ kind, value }`, a `"u"`nsigned or `"i"`nteger (a BigInt
  // past 2^53) where it's an integer that fits in 64 bits, else an `"f"`.
  deserializeNumber(expected, visit) {
    const peek = this.parseWhitespace();
    if (peek === -1) throw this.peekError("EOF while parsing a value");
    try {
      if (peek === 45) {
        this.index++;
        return visit(this.parseInteger(false));
      }
      if (peek >= 48 && peek <= 57) return visit(this.parseInteger(true));
      throw this.peekInvalidType(expected);
    } catch (e) {
      throw this.fixPosition(e);
    }
  }

  parseInteger(positive) {
    const c = this.next();
    if (c === -1) throw this.error("EOF while parsing a value");
    if (c === 48) {
      const p = this.peek();
      if (p >= 48 && p <= 57) throw this.peekError("invalid number");
      return this.parseNumber(positive, 0);
    }
    if (c < 49 || c > 57) throw this.error("invalid number");
    let significand = c - 48;
    for (;;) {
      const p = this.peek();
      if (p < 48 || p > 57) return this.parseNumber(positive, significand);
      const grown = $jsonGrow(significand, p - 48);
      if (grown > 18446744073709551615n) {
        return { kind: "f", value: this.parseLongInteger(positive, significand) };
      }
      this.index++;
      significand = grown;
    }
  }

  parseNumber(positive, significand) {
    const p = this.peek();
    if (p === 46) return { kind: "f", value: this.parseDecimal(positive, significand, 0) };
    if (p === 101 || p === 69) return { kind: "f", value: this.parseExponent(positive, significand, 0) };
    if (positive) return { kind: "u", value: significand };
    // `-0`, and below `i64::MIN`, are floats.
    if (significand === 0 || significand > 9223372036854775808n) {
      return { kind: "f", value: -Number(significand) };
    }
    return { kind: "i", value: -significand };
  }

  parseDecimal(positive, significand, exponentBeforeDecimalPoint) {
    this.index++;
    let exponentAfterDecimalPoint = 0;
    for (let p = this.peek(); p >= 48 && p <= 57; p = this.peek()) {
      const grown = $jsonGrow(significand, p - 48);
      if (grown > 18446744073709551615n) {
        const exponent = exponentBeforeDecimalPoint + exponentAfterDecimalPoint;
        return this.parseDecimalOverflow(positive, significand, exponent);
      }
      this.index++;
      significand = grown;
      exponentAfterDecimalPoint--;
    }
    if (exponentAfterDecimalPoint === 0) {
      throw this.peekError(this.peek() === -1 ? "EOF while parsing a value" : "invalid number");
    }
    const exponent = exponentBeforeDecimalPoint + exponentAfterDecimalPoint;
    const p = this.peek();
    return p === 101 || p === 69
      ? this.parseExponent(positive, significand, exponent)
      : this.f64FromParts(positive, significand, exponent);
  }

  parseExponent(positive, significand, startingExp) {
    this.index++;
    let positiveExp = true;
    const sign = this.peek();
    if (sign === 43) {
      this.index++;
    } else if (sign === 45) {
      this.index++;
      positiveExp = false;
    }
    const c = this.next();
    if (c === -1) throw this.error("EOF while parsing a value");
    if (c < 48 || c > 57) throw this.error("invalid number");
    let exp = c - 48;
    for (let p = this.peek(); p >= 48 && p <= 57; p = this.peek()) {
      this.index++;
      if (exp * 10 + (p - 48) > 2147483647) {
        return this.parseExponentOverflow(positive, significand == 0, positiveExp);
      }
      exp = exp * 10 + (p - 48);
    }
    // i32's saturating add and subtract.
    const finalExp = Math.max(-2147483648, Math.min(2147483647, positiveExp ? startingExp + exp : startingExp - exp));
    return this.f64FromParts(positive, significand, finalExp);
  }

  // Not correctly rounded, as serde_json's isn't without its
  // `float_roundtrip` feature: the significand times or over a power of ten.
  f64FromParts(positive, significand, exponent) {
    let f = Number(significand);
    for (;;) {
      const pow = $JSON_POW10[Math.abs(exponent)];
      if (pow !== undefined) {
        if (exponent >= 0) {
          f *= pow;
          if (!Number.isFinite(f)) throw this.error("number out of range");
        } else {
          f /= pow;
        }
        break;
      }
      if (f === 0) break;
      if (exponent >= 0) throw this.error("number out of range");
      f /= 1e308;
      exponent += 308;
    }
    return positive ? f : -f;
  }

  parseLongInteger(positive, significand) {
    let exponent = 0;
    for (;;) {
      const p = this.peek();
      if (p >= 48 && p <= 57) {
        this.index++;
        exponent++;
      } else if (p === 46) {
        return this.parseDecimal(positive, significand, exponent);
      } else if (p === 101 || p === 69) {
        return this.parseExponent(positive, significand, exponent);
      } else {
        return this.f64FromParts(positive, significand, exponent);
      }
    }
  }

  parseDecimalOverflow(positive, significand, exponent) {
    for (let p = this.peek(); p >= 48 && p <= 57; p = this.peek()) this.index++;
    const p = this.peek();
    return p === 101 || p === 69
      ? this.parseExponent(positive, significand, exponent)
      : this.f64FromParts(positive, significand, exponent);
  }

  parseExponentOverflow(positive, zeroSignificand, positiveExp) {
    if (!zeroSignificand && positiveExp) throw this.error("number out of range");
    for (let p = this.peek(); p >= 48 && p <= 57; p = this.peek()) this.index++;
    return positive ? 0 : -0;
  }

  // A string, after its opening quote. `escaped` says whether it had an
  // escape, which a borrowed `&str` can't (serde_json's `Reference::Copied`).
  parseStr() {
    const bytes = this.bytes;
    let text = "";
    let start = this.index;
    this.escaped = false;
    for (;;) {
      while (this.index < bytes.length) {
        const c = bytes[this.index];
        if (c === 34 || c === 92 || c < 32) break;
        this.index++;
      }
      if (this.index === bytes.length) throw this.error("EOF while parsing a string");
      const c = bytes[this.index];
      if (c === 34) {
        text += $jsonUtf8(bytes.subarray(start, this.index));
        this.index++;
        return text;
      }
      if (c !== 92) {
        this.index++;
        throw this.error("control character (\\u0000-\\u001F) found while parsing a string");
      }
      text += $jsonUtf8(bytes.subarray(start, this.index));
      this.index++;
      text += this.parseEscape();
      this.escaped = true;
      start = this.index;
    }
  }

  parseEscape() {
    const c = this.next();
    if (c === -1) throw this.error("EOF while parsing a string");
    switch (c) {
      case 34:
        return '"';
      case 92:
        return "\\";
      case 47:
        return "/";
      case 98:
        return "\b";
      case 102:
        return "\f";
      case 110:
        return "\n";
      case 114:
        return "\r";
      case 116:
        return "\t";
      case 117:
        return this.parseUnicodeEscape();
    }
    throw this.error("invalid escape");
  }

  parseUnicodeEscape() {
    const n = this.decodeHexEscape();
    // A trailing surrogate, which serde_json calls a leading one.
    if (n >= 0xdc00 && n <= 0xdfff) throw this.error("lone leading surrogate in hex escape");
    if (n < 0xd800 || n > 0xdbff) return String.fromCharCode(n);
    for (const expected of [92, 117]) {
      if (this.peek() === -1) throw this.error("EOF while parsing a string");
      const c = this.bytes[this.index++];
      if (c !== expected) throw this.error("unexpected end of hex escape");
    }
    const n2 = this.decodeHexEscape();
    if (n2 < 0xdc00 || n2 > 0xdfff) throw this.error("lone leading surrogate in hex escape");
    return String.fromCharCode(n, n2);
  }

  decodeHexEscape() {
    if (this.index + 4 > this.bytes.length) {
      this.index = this.bytes.length;
      throw this.error("EOF while parsing a string");
    }
    let n = 0;
    let valid = true;
    for (let i = 0; i < 4; i++) {
      const digit = $jsonHexDigit(this.bytes[this.index + i]);
      valid &&= digit >= 0;
      n = n * 16 + digit;
    }
    this.index += 4;
    if (!valid) throw this.error("invalid escape");
    return n;
  }

  ignoreStr() {
    const bytes = this.bytes;
    for (;;) {
      while (this.index < bytes.length) {
        const c = bytes[this.index];
        if (c === 34 || c === 92 || c < 32) break;
        this.index++;
      }
      if (this.index === bytes.length) throw this.error("EOF while parsing a string");
      const c = bytes[this.index];
      if (c === 34) {
        this.index++;
        return;
      }
      if (c !== 92) throw this.error("control character (\\u0000-\\u001F) found while parsing a string");
      this.index++;
      const escape = this.next();
      if (escape === -1) throw this.error("EOF while parsing a string");
      if (escape === 117) this.decodeHexEscape();
      else if (!'"\\/bfnrt'.includes(String.fromCharCode(escape))) throw this.error("invalid escape");
    }
  }

  parseObjectColon() {
    const c = this.parseWhitespace();
    if (c === 58) {
      this.index++;
      return;
    }
    throw this.peekError(c === -1 ? "EOF while parsing an object" : "expected `:`");
  }

  endSeq() {
    const c = this.parseWhitespace();
    if (c === 93) {
      this.index++;
      return;
    }
    if (c === 44) {
      this.index++;
      throw this.peekError(this.parseWhitespace() === 93 ? "trailing comma" : "trailing characters");
    }
    throw this.peekError(c === -1 ? "EOF while parsing a list" : "trailing characters");
  }

  endMap() {
    const c = this.parseWhitespace();
    if (c === 125) {
      this.index++;
      return;
    }
    throw this.peekError(c === 44 ? "trailing comma" : c === -1 ? "EOF while parsing an object" : "trailing characters");
  }

  ignoreValue() {
    const frames = [];
    let enclosing;
    for (;;) {
      const peek = this.parseWhitespace();
      if (peek === -1) throw this.peekError("EOF while parsing a value");
      let frame;
      if (peek === 110) {
        this.index++;
        this.parseIdent("ull");
      } else if (peek === 116) {
        this.index++;
        this.parseIdent("rue");
      } else if (peek === 102) {
        this.index++;
        this.parseIdent("alse");
      } else if (peek === 45) {
        this.index++;
        this.ignoreInteger();
      } else if (peek >= 48 && peek <= 57) {
        this.ignoreInteger();
      } else if (peek === 34) {
        this.index++;
        this.ignoreStr();
      } else if (peek === 91 || peek === 123) {
        if (enclosing !== undefined) frames.push(enclosing);
        enclosing = undefined;
        this.index++;
        frame = peek;
      } else {
        throw this.peekError("expected value");
      }
      let acceptComma = true;
      if (frame !== undefined) {
        acceptComma = false;
      } else if (enclosing !== undefined) {
        frame = enclosing;
        enclosing = undefined;
      } else if (frames.length > 0) {
        frame = frames.pop();
      } else {
        return;
      }
      for (;;) {
        const c = this.parseWhitespace();
        if (c === 44 && acceptComma) {
          this.index++;
          break;
        }
        if (c === -1) throw this.peekError(frame === 91 ? "EOF while parsing a list" : "EOF while parsing an object");
        if (!((c === 93 && frame === 91) || (c === 125 && frame === 123))) {
          if (acceptComma) throw this.peekError(frame === 91 ? "expected `,` or `]`" : "expected `,` or `}`");
          break;
        }
        this.index++;
        if (frames.length === 0) return;
        frame = frames.pop();
        acceptComma = true;
      }
      if (frame === 123) {
        const quote = this.parseWhitespace();
        if (quote !== 34) throw this.peekError(quote === -1 ? "EOF while parsing an object" : "key must be a string");
        this.index++;
        this.ignoreStr();
        const colon = this.parseWhitespace();
        if (colon !== 58) throw this.peekError(colon === -1 ? "EOF while parsing an object" : "expected `:`");
        this.index++;
      }
      enclosing = frame;
    }
  }

  ignoreInteger() {
    const c = this.next();
    if (c === 48) {
      const p = this.peek();
      if (p >= 48 && p <= 57) throw this.peekError("invalid number");
    } else if (c >= 49 && c <= 57) {
      for (let p = this.peek(); p >= 48 && p <= 57; p = this.peek()) this.index++;
    } else {
      throw this.error("invalid number");
    }
    const p = this.peek();
    if (p === 46) this.ignoreDecimal();
    else if (p === 101 || p === 69) this.ignoreExponent();
  }

  ignoreDecimal() {
    this.index++;
    let atLeastOneDigit = false;
    for (let p = this.peek(); p >= 48 && p <= 57; p = this.peek()) {
      this.index++;
      atLeastOneDigit = true;
    }
    if (!atLeastOneDigit) throw this.peekError("invalid number");
    const p = this.peek();
    if (p === 101 || p === 69) this.ignoreExponent();
  }

  ignoreExponent() {
    this.index++;
    const sign = this.peek();
    if (sign === 43 || sign === 45) this.index++;
    const c = this.next();
    if (c < 48 || c > 57) throw this.error("invalid number");
    for (let p = this.peek(); p >= 48 && p <= 57; p = this.peek()) this.index++;
  }

  // An array or an object: `visit` reads what's inside, and the closing
  // bracket is looked for even when it fails, as serde_json does.
  nested(visit, end) {
    if (--this.remainingDepth === 0) throw this.peekError("recursion limit exceeded");
    this.index++;
    let value;
    let failure;
    try {
      value = visit();
    } catch (e) {
      if (!(e instanceof $JsonError)) throw e;
      failure = e;
    }
    this.remainingDepth++;
    try {
      end();
    } catch (e) {
      if (!(e instanceof $JsonError)) throw e;
      failure ??= e;
    }
    if (failure) throw failure;
    return value;
  }

  // `deserialize_seq`: `[ .. ]`, which `visit` reads with a `$JsonSeq`.
  deserializeSeq(expected, visit) {
    const peek = this.parseWhitespace();
    if (peek === -1) throw this.peekError("EOF while parsing a value");
    try {
      if (peek !== 91) throw this.peekInvalidType(expected);
      return this.nested(() => visit(new $JsonSeq(this)), () => this.endSeq());
    } catch (e) {
      throw this.fixPosition(e);
    }
  }

  // `deserialize_map` and `deserialize_struct`: `{ .. }`, or for a struct,
  // `[ .. ]` too.
  deserializeMap(expected, visitMap, visitSeq) {
    const peek = this.parseWhitespace();
    if (peek === -1) throw this.peekError("EOF while parsing a value");
    try {
      if (peek === 91 && visitSeq) return this.nested(() => visitSeq(new $JsonSeq(this)), () => this.endSeq());
      if (peek !== 123) throw this.peekInvalidType(expected);
      return this.nested(() => visitMap(new $JsonMap(this)), () => this.endMap());
    } catch (e) {
      throw this.fixPosition(e);
    }
  }

  deserializeStr(expected, visit) {
    const peek = this.parseWhitespace();
    if (peek === -1) throw this.peekError("EOF while parsing a value");
    try {
      if (peek !== 34) throw this.peekInvalidType(expected);
      this.index++;
      const s = this.parseStr();
      return visit(s, !this.escaped);
    } catch (e) {
      throw this.fixPosition(e);
    }
  }

  // What serde's own impls read, from the text.

  bool() {
    const peek = this.parseWhitespace();
    if (peek === -1) throw this.peekError("EOF while parsing a value");
    try {
      if (peek === 116) {
        this.index++;
        this.parseIdent("rue");
        return true;
      }
      if (peek === 102) {
        this.index++;
        this.parseIdent("alse");
        return false;
      }
      throw this.peekInvalidType("a boolean");
    } catch (e) {
      throw this.fixPosition(e);
    }
  }

  unit(expected = "unit") {
    const peek = this.parseWhitespace();
    if (peek === -1) throw this.peekError("EOF while parsing a value");
    try {
      if (peek !== 110) throw this.peekInvalidType(expected);
      this.index++;
      this.parseIdent("ull");
      return undefined;
    } catch (e) {
      throw this.fixPosition(e);
    }
  }

  unitStruct(expected) {
    return this.unit(expected);
  }

  option(read) {
    if (this.parseWhitespace() !== 110) return read(this);
    this.index++;
    this.parseIdent("ull");
    return undefined;
  }

  // `deserialize_any`: whatever's there, for the `visitor` method of its
  // kind (`unit`, `bool`, `number`, `string`, `seq` or `map`), which is
  // `expected` if the visitor has none.
  deserializeAny(expected, visitor) {
    const peek = this.parseWhitespace();
    if (peek === -1) throw this.peekError("EOF while parsing a value");
    const invalid = (unexpected) => $jsonError(`invalid type: ${unexpected}, expected ${expected}`);
    try {
      if (peek === 110) {
        this.index++;
        this.parseIdent("ull");
        if (visitor.unit) return visitor.unit();
        throw invalid("null");
      }
      if (peek === 116 || peek === 102) {
        this.index++;
        this.parseIdent(peek === 116 ? "rue" : "alse");
        if (visitor.bool) return visitor.bool(peek === 116);
        throw invalid(`boolean \`${peek === 116}\``);
      }
      if (peek === 45 || (peek >= 48 && peek <= 57)) {
        if (peek === 45) this.index++;
        const n = this.parseInteger(peek !== 45);
        if (visitor.number) return visitor.number(n);
        throw invalid($jsonUnexpectedNumber(n));
      }
      if (peek === 34) {
        this.index++;
        const s = this.parseStr();
        if (visitor.string) return visitor.string(s, !this.escaped);
        throw invalid(`string ${$debugStr(s)}`);
      }
      if (peek === 91) {
        return this.nested(() => {
          if (!visitor.seq) throw invalid("sequence");
          return visitor.seq(new $JsonSeq(this));
        }, () => this.endSeq());
      }
      if (peek === 123) {
        return this.nested(() => {
          if (!visitor.map) throw invalid("map");
          return visitor.map(new $JsonMap(this));
        }, () => this.endMap());
      }
      throw this.peekError("expected value");
    } catch (e) {
      throw this.fixPosition(e);
    }
  }

  // serde's `ContentVisitor`: the value, read into a `{ type, value }`.
  content() {
    return this.deserializeAny("any value", {
      unit: () => ({ type: "unit" }),
      bool: (value) => ({ type: "bool", value }),
      number: (value) => ({ type: "num", value }),
      string: (value, borrowed) => ({ type: "str", value, borrowed }),
      seq: (seq) => {
        const items = [];
        while (seq.next()) items.push(seq.value((json) => json.content()));
        return { type: "seq", value: items };
      },
      map: (map) => {
        const entries = [];
        while (map.next()) {
          const key = map.key().content();
          entries.push([key, map.value((json) => json.content())]);
        }
        return { type: "map", value: entries };
      },
    });
  }

  // `deserialize_enum`: `"Name"`, or `{"Name": ..}`. `visit` gets the
  // variant's name and a `$JsonVariant` to read what it holds. `enumName` is
  // the enum's, for a flattened one.
  enum(enumName, variants, visit, other) {
    const peek = this.parseWhitespace();
    if (peek === 34) return visit(this.identifier(variants, other), new $JsonVariant(this, true));
    if (peek !== 123) throw this.peekError(peek === -1 ? "EOF while parsing a value" : "expected value");
    if (--this.remainingDepth === 0) throw this.peekError("recursion limit exceeded");
    this.index++;
    let value;
    try {
      const c = this.parseWhitespace();
      if (c !== 34) {
        throw this.peekError(
          c === 125 ? "expected value" : c === -1 ? "EOF while parsing an object" : "key must be a string",
        );
      }
      const name = this.identifier(variants, other);
      this.parseObjectColon();
      value = visit(name, new $JsonVariant(this, false));
    } finally {
      this.remainingDepth++;
    }
    const c = this.parseWhitespace();
    if (c === 125) {
      this.index++;
      return value;
    }
    throw this.error(c === -1 ? "EOF while parsing an object" : "expected value");
  }

  // A variant's name, from a string.
  identifier(variants, other) {
    return this.deserializeStr("variant identifier", (name) => $jsonVariantNamed(name, variants, other));
  }
}
