// What a `char` is, as Rust's std answers it, of its own Unicode tables
// (#1888, ADR 1163), `unicode_tables.js`: a JS engine's `\p{..}` and
// `toUpperCase()` are its own Unicode version's, which may be another, so a
// `char` Unicode assigned since is unassigned to one and shown escaped.

// Each table's `char`s as a regular expression, `shape` its pattern with
// `#` the table's class, `[\u{41}-\u{5a}..]`: `^#$` of one `char`, made the
// first time it's asked, the boundaries each a step from the last.
const $unicodeRegexps = new Map();
function $unicodeRegexp(table, shape = "^#$") {
  let made = $unicodeRegexps.get(table);
  if (made === undefined) {
    const range = (start, end) =>
      end === start ? `\\u{${start.toString(16)}}` : `\\u{${start.toString(16)}}-\\u{${end.toString(16)}}`;
    let at = 0;
    let start = 0;
    let ranges = "";
    table.split(",").forEach((step, i) => {
      at += parseInt(step, 36);
      if (i % 2 === 0) start = at;
      else ranges += range(start, at - 1);
    });
    made = { class: `[${ranges}]`, shapes: new Map() };
    $unicodeRegexps.set(table, made);
  }
  let regexp = made.shapes.get(shape);
  if (regexp === undefined) {
    regexp = new RegExp(shape.replace("#", made.class), "u");
    made.shapes.set(shape, regexp);
  }
  return regexp;
}

function $isAlphabetic(c) {
  return $unicodeRegexp($unicodeAlphabetic).test(c);
}

function $isNumeric(c) {
  return $unicodeRegexp($unicodeNumeric).test(c);
}

function $isAlphanumeric(c) {
  return $isAlphabetic(c) || $isNumeric(c);
}

function $isUppercase(c) {
  return $unicodeRegexp($unicodeUppercase).test(c);
}

function $isLowercase(c) {
  return $unicodeRegexp($unicodeLowercase).test(c);
}

function $isWhitespace(c) {
  return $unicodeRegexp($unicodeWhitespace).test(c);
}

// `s.split_whitespace()`: its words, between runs of `char::is_whitespace`'s.
function $splitWhitespace(s) {
  return s.split($unicodeRegexp($unicodeWhitespace, "#+")).filter((word) => word !== "");
}

// A case map, `char::to_uppercase`'s or `to_lowercase`'s, made the first time
// it's asked: each `char` it changes, by its code point, and what it becomes.
const $unicodeMaps = new Map();
function $unicodeMap(table) {
  let made = $unicodeMaps.get(table);
  if (made === undefined) {
    made = new Map();
    let at = 0;
    for (const token of table.split(",")) {
      const many = token.split("=");
      if (many.length === 2) {
        at += parseInt(many[0], 36);
        made.set(at, String.fromCodePoint(...many[1].split("+").map((c) => parseInt(c, 36))));
        continue;
      }
      const [step, offset, count = 1, stride = 0] = token.split(":").map((n) => parseInt(n, 36));
      at += step;
      for (let i = 0; i < count; i++, at += stride) made.set(at, String.fromCodePoint(at + offset));
      at -= stride;
    }
    $unicodeMaps.set(table, made);
  }
  return made;
}

// `s.to_uppercase()`, and a `char`'s: each `char`'s capital, `ß` `SS`. ASCII
// is JS's own, as Rust's is ASCII's alone.
function $toUppercase(s) {
  if (/^[\0-\x7f]*$/.test(s)) return s.toUpperCase();
  const upper = $unicodeMap($unicodeUpper);
  let out = "";
  for (const c of s) out += upper.get(c.codePointAt(0)) ?? c;
  return out;
}

// `s.to_lowercase()`: each `char`'s, and `Σ` `ς` where it ends a word, as
// Rust's Final_Sigma says: a cased `char` before it, and none after, past
// those case-ignorable.
function $toLowercase(s) {
  if (/^[\0-\x7f]*$/.test(s)) return s.toLowerCase();
  const lower = $unicodeMap($unicodeLower);
  const chars = Array.from(s);
  const ignorable = $unicodeRegexp($unicodeCaseIgnorable);
  const cased = $unicodeRegexp($unicodeCased);
  const thenCased = (from, step) => {
    for (let i = from; i >= 0 && i < chars.length; i += step) if (!ignorable.test(chars[i])) return cased.test(chars[i]);
    return false;
  };
  let out = "";
  chars.forEach((c, i) => {
    if (c === "Σ") out += thenCased(i - 1, -1) && !thenCased(i + 1, 1) ? "ς" : "σ";
    else out += lower.get(c.codePointAt(0)) ?? c;
  });
  return out;
}
