
// What serde's impls and derives read, the same whatever they read from:
// the text (`$JsonReader`), or a value already read (`$JsonContent`), which
// is how serde reads a tagged or an untagged enum.
class $JsonDecoder {
  // A hint's `deserialize_*` of a hand-written visitor, given the methods
  // that hint takes: of a value already read, serde's `ContentDeserializer`
  // and serde_json's `Value`, what it is visited by its method, or the
  // type it is refused (#236). The text's peeks first, `$JsonReader`'s.
  deserializeHinted(expected, visitor) {
    return this.deserializeAny(expected, visitor);
  }

  // An integer type, named as serde names it, from `min` to `max`: a
  // number, or for an `i64` or `u64`, a BigInt (ADR 0086).
  int(name, min, max) {
    return this.deserializeNumber(name, (n) => {
      if (n.kind === "f") throw $jsonError(`invalid type: floating point \`${$jsonNumber(n.value)}\`, expected ${name}`);
      if (n.kind === "u" ? n.value > max : n.value < min) {
        throw $jsonError(`invalid value: integer \`${n.value}\`, expected ${name}`);
      }
      return typeof max === "bigint" ? BigInt(n.value) : Number(n.value);
    });
  }

  f64() {
    return this.deserializeNumber("f64", (n) => Number(n.value));
  }

  // serde's `f32` takes any number, `as f32`: an integer rounded once, by
  // `$bigToF32`, and a float, serde_json's `f64`, rounded (ADR 0122).
  f32() {
    return this.deserializeNumber("f32", (n) => (n.kind === "f" ? Math.fround(n.value) : $bigToF32(BigInt(n.value))));
  }

  string() {
    return this.deserializeStr("a string", (s) => s);
  }

  char() {
    return this.deserializeStr("a character", $jsonChar);
  }

  // A `&str`, which borrows from the text, so it can't have had an escape.
  borrowedStr() {
    return this.deserializeStr("a borrowed string", (s, borrowed) => {
      if (borrowed) return s;
      throw $jsonError(`invalid type: string ${$debugStr(s)}, expected a borrowed string`);
    });
  }

  vec(read) {
    return this.deserializeSeq("a sequence", (seq) => {
      const items = [];
      while (seq.next()) items.push(seq.value(read));
      return items;
    });
  }

  tuple(reads) {
    const expected = `a tuple of size ${reads.length}`;
    return this.deserializeSeq(expected, (seq) => reads.map((read, i) => seq.element(read, i, expected)));
  }

  array(length, read) {
    const expected = length === 0 ? "an empty array" : `an array of length ${length}`;
    return this.deserializeSeq(expected, (seq) =>
      Array.from({ length }, (_, i) => seq.element(read, i, expected)),
    );
  }

  map(readKey, read) {
    return this.deserializeMap("a map", (map) => {
      const entries = new Map();
      while (map.next()) {
        const key = readKey(map.key());
        entries.set(key, map.value(read));
      }
      return entries;
    });
  }

  // A struct's fields: `[name, read]`, or `[[name, ...aliases], read]`, and
  // `missing`, the value it has when it's not there, for `#[serde(default)]`.
  // `build` makes the struct of their values.
  struct(expected, fields, build, options = {}) {
    const visitMap = (map) => this.structMap(map, fields, build, options);
    // With a flattened field, serde reads a struct as a map.
    if (options.flatten) return this.deserializeMap(options.expecting ?? expected, visitMap);
    return this.deserializeStruct(
      options.expecting ?? expected,
      fields.flatMap(([names]) => names),
      visitMap,
      (seq) => this.structSeq(seq, expected, fields, build, options),
    );
  }

  // `deserialize_struct`: a map, of these `names`, or for most readers, any
  // map at all.
  deserializeStruct(expected, names, visitMap, visitSeq) {
    return this.deserializeMap(expected, visitMap, visitSeq);
  }

  // An untagged struct variant, which serde reads from an object only.
  untaggedStruct(expected, fields, build, options = {}) {
    return this.deserializeAny(options.expecting ?? expected, {
      map: (map) => this.structMap(map, fields, build, options),
    });
  }

  // `flatten`: the readers of the flattened fields, which read what the
  // struct's own fields don't, kept until they're all read.
  structMap(map, fields, build, { deny = false, container, flatten } = {}) {
    const values = new Array(fields.length);
    const seen = new Array(fields.length).fill(false);
    const kept = [];
    while (map.next()) {
      const keyContent = map.key().content();
      const key = keyContent.value;
      const i = $jsonField(key, fields, deny && !flatten);
      if (i < 0 && flatten) {
        kept.push([keyContent, map.value((json) => json.content())]);
        continue;
      }
      if (i < 0) {
        map.value((json) => json.ignoreValue());
        continue;
      }
      if (seen[i]) throw $jsonError(`duplicate field \`${$jsonName(fields[i][0])}\``);
      values[i] = map.value(fields[i][1]);
      seen[i] = true;
    }
    const defaults = container?.();
    fields.forEach(([names, read, missing], i) => {
      if (!seen[i]) values[i] = missing ? missing(defaults) : read(new $JsonMissing($jsonName(names)));
    });
    if (!flatten) return build(values, defaults);
    for (const read of flatten) values.push(read(new $JsonFlat(kept)));
    const left = deny && kept.find(Boolean);
    if (left) throw $jsonError(`unknown field \`${left[0].value}\``);
    return build(values, defaults);
  }

  structSeq(seq, expected, fields, build, { container, expecting } = {}) {
    const length = expecting ?? `${expected} with ${fields.length} element${fields.length === 1 ? "" : "s"}`;
    const defaults = container?.();
    const values = fields.map(([, read, missing], i) => seq.element(read, i, length, missing, defaults));
    return build(values, defaults);
  }

  // A tuple struct's fields: `read`, or `[read, missing]`.
  tupleStruct(expected, fields, build = (values) => values, { container, expecting } = {}) {
    const length = expecting ?? `${expected} with ${fields.length} element${fields.length === 1 ? "" : "s"}`;
    return this.deserializeSeq(expecting ?? expected, (seq) => {
      const defaults = container?.();
      const values = fields.map((field, i) => {
        const [read, missing] = typeof field === "function" ? [field] : field;
        return seq.element(read, i, length, missing, defaults);
      });
      return build(values, defaults);
    });
  }

  // An internally tagged enum's unit variant, whose other fields are
  // ignored (serde's `InternallyTaggedUnitVisitor`).
  taggedUnit(expected) {
    return this.deserializeAny(expected, {
      seq: () => undefined,
      map: (map) => {
        while (map.next()) {
          map.key().ignore();
          map.value((json) => json.ignoreValue());
        }
        return undefined;
      },
    });
  }

  // An untagged unit variant: `null` (serde's `UntaggedUnitVisitor`).
  untaggedUnit(expected) {
    return this.deserializeAny(expected, { unit: () => undefined });
  }

  // `#[serde(tag = "type")]`: the tag, found among the other fields, which
  // are kept for the variant to read (serde's `TaggedContentVisitor`).
  internallyTagged(tag, expected, variants, visit, other) {
    const [variant, rest] = this.deserializeAny(expected, {
      seq: (seq) => {
        if (!seq.next()) throw $jsonError(`missing field \`${tag}\``);
        const variant = seq.value((json) => json.identifier(variants, other));
        const items = [];
        while (seq.next()) items.push(seq.value((json) => json.content()));
        return [variant, { type: "seq", value: items }];
      },
      map: (map) => {
        let variant;
        const entries = [];
        while (map.next()) {
          const key = map.key().content();
          if (key.value !== tag) {
            entries.push([key, map.value((json) => json.content())]);
            continue;
          }
          if (variant !== undefined) throw $jsonError(`duplicate field \`${tag}\``);
          variant = map.value((json) => json.identifier(variants, other));
        }
        if (variant === undefined) throw $jsonError(`missing field \`${tag}\``);
        return [variant, { type: "map", value: entries }];
      },
    });
    return visit(variant, new $JsonContent(rest, true));
  }

  // `#[serde(tag = "t", content = "c")]`, as serde's derive reads one: the
  // tag, then the content, or the content first, kept until the tag says
  // how to read it.
  adjacentlyTagged(tag, content, expected, variants, visit, { deny = false, other } = {}) {
    const variantOf = (json) =>
      json.enum(
        undefined,
        variants,
        (name, access) => {
          access.unit();
          return name;
        },
        other,
      );
    // The next key that's the tag or the content, skipping any other.
    const relevant = (map) => {
      while (map.next()) {
        const key = map.key().string();
        if (key === tag) return "tag";
        if (key === content) return "content";
        if (deny) {
          throw $jsonError(`invalid value: string ${$debugStr(key)}, expected ${$debugStr(tag)} or ${$debugStr(content)}`);
        }
        map.value((json) => json.ignoreValue());
      }
      return undefined;
    };
    const finish = (map, value) => {
      const key = relevant(map);
      if (key !== undefined) throw $jsonError(`duplicate field \`${key === "tag" ? tag : content}\``);
      return value;
    };
    return this.deserializeStruct(
      expected,
      [tag, content],
      (map) => {
        const first = relevant(map);
        if (first === "tag") {
          const variant = map.value(variantOf);
          const second = relevant(map);
          if (second === "tag") throw $jsonError(`duplicate field \`${tag}\``);
          if (second === "content") return finish(map, map.value((json) => visit(variant, json)));
          return visit(variant, new $JsonMissing(content));
        }
        if (first === "content") {
          const buffered = map.value((json) => json.content());
          const second = relevant(map);
          if (second === "tag") {
            const variant = map.value(variantOf);
            return finish(map, visit(variant, new $JsonContent(buffered, true)));
          }
          if (second === "content") throw $jsonError(`duplicate field \`${content}\``);
        }
        throw $jsonError(`missing field \`${tag}\``);
      },
      (seq) => {
        if (!seq.next()) throw $jsonError(`invalid length 0, expected ${expected}`);
        const variant = seq.value((json) => json.identifier(variants, other));
        if (!seq.next()) throw $jsonError(`invalid length 1, expected ${expected}`);
        return seq.value((json) => visit(variant, json));
      },
    );
  }

  // `#[serde(untagged)]`: the value, read once and kept, then each way of
  // reading it in turn, the first that works.
  untagged(message, attempts) {
    const content = this.content();
    for (const attempt of attempts) {
      try {
        return attempt(new $JsonContent(content, false));
      } catch (e) {
        if (!(e instanceof $JsonError)) throw e;
      }
    }
    throw $jsonError(message);
  }
}
