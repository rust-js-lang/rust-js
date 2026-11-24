
// What a JS caller gives a Rust parameter, checked against its Rust type
// where the function starts (#2403, ADR 1192): a value outside it throws,
// naming the function and the parameter, what was expected and what was
// given, as wasm-bindgen's glue does, so Rust never sees an impossible
// value. `where` is `double(n)`; `none`, where the parameter is an
// `Option`, the `None` its form allows: "null", "undefined" or "nullish".
function $checkNone(value, none) {
  switch (none) {
    case "nullish":
      return value == null;
    case "null":
      return value === null;
    case "undefined":
      return value === undefined;
    default:
      return false;
  }
}

// What was given, as an error names it: a string quoted, a bigint with
// its `n`, an object by its kind.
function $checkShown(value) {
  switch (typeof value) {
    case "string":
      return JSON.stringify(value);
    case "bigint":
      return `${value}n`;
    case "function":
      return "a function";
    case "object":
      return value === null ? "null" : Array.isArray(value) ? "an array" : "an object";
    default:
      return String(value);
  }
}

// A value's failure, where it is: of a parameter, `where`, the error
// itself; inside one, given no `where`, what failed, which each level
// it's in names its place in as it passes, `.items[1].qty`, so a value
// that checks builds no path (`$checkWhole`).
function $checkFailed(Kind, where, expected, value) {
  if (where === undefined) throw new $CheckFailure(Kind, expected, value);
  throw new Kind(`rust-js: ${where}: expected ${expected}, got ${$checkShown(value)}`);
}

class $CheckFailure {
  constructor(Kind, expected, value) {
    this.Kind = Kind;
    this.expected = expected;
    this.value = value;
    this.path = "";
  }
}

// A key's or an index's place in a path: `[1]`, `.qty`, or a symbol's,
// `[Symbol(id)]`, which no template literal can write.
function $checkPlace(at) {
  switch (typeof at) {
    case "number":
      return `[${at}]`;
    case "symbol":
      return `[${String(at)}]`;
    default:
      return `.${at}`;
  }
}

// A failure inside a value, at `at`, a field's key or an item's index,
// named there; anything else thrown passes as it is.
function $checkAt(failure, at) {
  if (failure instanceof $CheckFailure) failure.path = $checkPlace(at) + failure.path;
  throw failure;
}

// While a parameter's whole value is checked, each object checked so far,
// by each checker it's checked by: a value of `Rc`s that holds itself, a
// task's request whose tasks hold it, is checked once of each type, not
// again as each cycle comes round, which never ends.
let $checking = null;

// Whether `value` is an object `check` has checked, or is checking, in the
// whole value being checked; else marked so.
function $checkedBefore(value, check) {
  if ($checking === null || typeof value !== "object" || value === null) return false;
  let checks = $checking.get(value);
  if (checks === undefined) $checking.set(value, (checks = new Set()));
  if (checks.has(check)) return true;
  checks.add(check);
  return false;
}

// A parameter's whole value, checked by `check`, the checker of its type;
// a failure inside it named by where it is in it, `count(order.items[1])`.
function $checkWhole(check, value, where, none) {
  if ($checkNone(value, none)) return;
  const outer = $checking;
  $checking = new Map();
  try {
    check(value);
  } catch (failure) {
    if (!(failure instanceof $CheckFailure)) throw failure;
    const at = `${where.slice(0, -1)}${failure.path})`;
    throw new failure.Kind(`rust-js: ${at}: expected ${failure.expected}, got ${$checkShown(failure.value)}`);
  } finally {
    $checking = outer;
  }
}

// Each parameter of a function JS gives, checked in one call (#2403, the
// lead's (A)): `place`, the function and each checked parameter as JS
// names it, `count(order, n)`, written once; then each value and its
// check. A leaf's check, of a number or a string, is called as it is; a
// whole value's, a struct's or an enum's, as `$checkWhole` calls it, each
// object checked once of each type. A failure names its parameter by its
// place, `count(n)`, its path inside it too, `count(order.items[1])`,
// built only where one fails, as each check's own was.
function $checks(place) {
  for (let i = 1; i < arguments.length; i += 2) {
    const value = arguments[i];
    const check = arguments[i + 1];
    const leaf = $checkLeaves.has(check);
    const outer = $checking;
    if (!leaf) $checking = new Map();
    try {
      check(value);
    } catch (failure) {
      if (!(failure instanceof $CheckFailure)) throw failure;
      const open = place.indexOf("(");
      const param = place.slice(open + 1, -1).split(", ")[(i - 1) / 2];
      const at = `${place.slice(0, open)}(${param}${failure.path})`;
      throw new failure.Kind(`rust-js: ${at}: expected ${failure.expected}, got ${$checkShown(failure.value)}`);
    } finally {
      if (!leaf) $checking = outer;
    }
  }
}

// An object of a struct's: not `null`, not an array.
function $checkObject(value, where, none) {
  if ($checkNone(value, none)) return;
  if (typeof value !== "object" || value === null || Array.isArray(value)) $checkFailed(TypeError, where, "an object", value);
}

// What a check reads of `object`'s `key`: its value by its descriptor, its
// own or its prototypes', never a getter run nor a proxy's `get`, so JS's
// value is read as often as Rust reads it (ADRs 1022, 1087); `$checkAccessor`
// where an accessor gives it, which a check leaves to the body, read once
// there, unchecked (#2403, the limit the guide says).
// One copy's own, never a mark another copy reads: its `$checkRead` gives
// it to its own checker alone, so no registry symbol (ADR 1184).
const $checkAccessor = Object.freeze({});
function $checkRead(object, key) {
  for (let o = object; o !== null; o = Object.getPrototypeOf(o)) {
    const property = Object.getOwnPropertyDescriptor(o, key);
    if (property !== undefined) return "value" in property ? property.value : $checkAccessor;
  }
  return undefined;
}

// A field of `object`'s, by its key, checked by `check`, its `None`
// `none`, and of an array `inner`, its items' checker; an accessor's not.
function $checkField(object, key, check, none, inner) {
  const value = $checkRead(object, key);
  if (value === $checkAccessor || $checkNone(value, none) || $checkedBefore(value, check)) return;
  try {
    check(value, undefined, undefined, inner);
  } catch (failure) {
    $checkAt(failure, key);
  }
}

// A struct's checker made of its table (#2403, the lead's (B)): its
// fields' checks by key, each a check or `[check, none]`, read where it
// first checks, so a struct that holds itself, or whose fields' checkers
// are made after it, names them; and of a type no other crate can make, its
// brand's `key` and `name`, checked first. Made once, its module's, and
// marked pure where it's made, so a bundler drops one nothing reads.
function $checkShape(fields, key, name) {
  let table;
  let keys;
  return function check(value) {
    if (key !== undefined) $checkBrand(value, undefined, undefined, key, name);
    $checkObject(value);
    if (table === undefined) {
      table = fields();
      keys = Reflect.ownKeys(table);
    }
    for (let i = 0; i < keys.length; i++) {
      const field = table[keys[i]];
      if (Array.isArray(field)) $checkField(value, keys[i], field[0], field[1]);
      else $checkField(value, keys[i], field);
    }
  };
}

// An `Option` in what's checked, its `None` either JS's, else its value by
// `check`.
function $checkOption(value, where, check) {
  if (value != null) check(value, where);
}

// An array, each item checked by `inner`, of a `Vec`, a slice or an array
// of `length` items where it's one.
function $checkArray(value, where, none, inner, length) {
  if ($checkNone(value, none)) return;
  if (!Array.isArray(value)) $checkFailed(TypeError, where, "an array", value);
  if (length !== undefined && value.length !== length) $checkFailed(RangeError, where, `an array of ${length}`, value);
  // An integer kind's items, each tested in place, its bounds read once:
  // one that fails is checked by its helper, for its error.
  const range = $checkIntRanges.get(inner);
  if (range !== undefined) {
    const min = range[0];
    const max = range[1];
    for (let i = 0; i < value.length; i++) {
      const item = value[i];
      if (!(typeof item === "number" && Number.isInteger(item) && item >= min && item <= max)) $checkItem(inner, item, i);
    }
    return;
  }
  for (let i = 0; i < value.length; i++) $checkItem(inner, value[i], i);
}

// An array's item `i`, checked by `inner`, its failure named by its index.
function $checkItem(inner, item, i) {
  if ($checkedBefore(item, inner)) return;
  try {
    inner(item);
  } catch (failure) {
    $checkAt(failure, i);
  }
}

// A number of an integer type of 32 bits or fewer: whole, and in range.
function $checkInt(value, where, none, expected, min, max) {
  if ($checkNone(value, none)) return;
  if (typeof value !== "number") $checkFailed(TypeError, where, expected, value);
  if (!Number.isInteger(value) || value < min || value > max) $checkFailed(RangeError, where, expected, value);
}

function $checkU8(value, where, none) {
  $checkInt(value, where, none, "a u8", 0, 0xff);
}

function $checkU16(value, where, none) {
  $checkInt(value, where, none, "a u16", 0, 0xffff);
}

function $checkU32(value, where, none) {
  $checkInt(value, where, none, "a u32", 0, 0xffffffff);
}

function $checkI8(value, where, none) {
  $checkInt(value, where, none, "an i8", -0x80, 0x7f);
}

function $checkI16(value, where, none) {
  $checkInt(value, where, none, "an i16", -0x8000, 0x7fff);
}

function $checkI32(value, where, none) {
  $checkInt(value, where, none, "an i32", -0x80000000, 0x7fffffff);
}

// A bigint of a 64- or 128-bit integer type, in range: what `BigInt.asUintN`
// or `asIntN` of its width leaves as it is.
function $checkBig(value, where, none, expected, bits, signed) {
  if ($checkNone(value, none)) return;
  if (typeof value !== "bigint") $checkFailed(TypeError, where, expected, value);
  if ((signed ? BigInt.asIntN(bits, value) : BigInt.asUintN(bits, value)) !== value) {
    $checkFailed(RangeError, where, expected, value);
  }
}

function $checkU64(value, where, none) {
  $checkBig(value, where, none, "a u64", 64, false);
}

function $checkI64(value, where, none) {
  $checkBig(value, where, none, "an i64", 64, true);
}

function $checkU128(value, where, none) {
  $checkBig(value, where, none, "a u128", 128, false);
}

function $checkI128(value, where, none) {
  $checkBig(value, where, none, "an i128", 128, true);
}

// Any number, NaN too, as Rust's `f64` has it.
function $checkF64(value, where, none) {
  if ($checkNone(value, none)) return;
  if (typeof value !== "number") $checkFailed(TypeError, where, "an f64", value);
}

// A number an `f32` is: rust-js keeps one rounded to it, `Math.fround`.
function $checkF32(value, where, none) {
  if ($checkNone(value, none)) return;
  if (typeof value !== "number") $checkFailed(TypeError, where, "an f32", value);
  if (Math.fround(value) !== value && value === value) $checkFailed(RangeError, where, "an f32", value);
}

// `true` or `false`, never a value JS takes as either.
function $checkBool(value, where, none) {
  if ($checkNone(value, none)) return;
  if (typeof value !== "boolean") $checkFailed(TypeError, where, "a bool", value);
}

// One code point, never a lone surrogate, as a Rust `char` is.
function $checkChar(value, where, none) {
  if ($checkNone(value, none)) return;
  if (typeof value !== "string" || !value.isWellFormed() || value.length === 0 || value.length > 2 || (value.length === 2 && value.codePointAt(0) <= 0xffff)) {
    $checkFailed(TypeError, where, "a char", value);
  }
}

// A string, well-formed, as a Rust `str` is valid UTF-8: one with a lone
// surrogate is no `String`.
function $checkStr(value, where, none) {
  if ($checkNone(value, none)) return;
  if (typeof value !== "string" || !value.isWellFormed()) $checkFailed(TypeError, where, "a String", value);
}

// `()`, which is `undefined`.
function $checkUnit(value, where) {
  if (value !== undefined) $checkFailed(TypeError, where, "()", value);
}

// One of a fieldless enum's variants, each its name.
function $checkOneOf(value, names, where, none) {
  if ($checkNone(value, none)) return;
  if (!names.includes(value)) $checkFailed(TypeError, where, names.map((name) => JSON.stringify(name)).join(" | "), value);
}

// A tuple, an array of exactly its items, each checked by its own of `items`.
function $checkTuple(value, where, none, items) {
  if ($checkNone(value, none)) return;
  if (!Array.isArray(value) || value.length !== items.length) $checkFailed(TypeError, where, `an array of ${items.length}`, value);
  for (let i = 0; i < items.length; i++) {
    try {
      items[i](value[i]);
    } catch (failure) {
      $checkAt(failure, i);
    }
  }
}

// The checks of a value that holds none, which `$checks` calls with no map
// of the objects checked so far, as none is checked inside it.
const $checkLeaves = new Set([
  $checkU8,
  $checkU16,
  $checkU32,
  $checkU64,
  $checkU128,
  $checkI8,
  $checkI16,
  $checkI32,
  $checkI64,
  $checkI128,
  $checkF32,
  $checkF64,
  $checkBool,
  $checkChar,
  $checkStr,
  $checkUnit,
]);

// Each integer kind's checker of 32 bits or fewer, and its bounds: what an
// array of them tests in place (`$checkArray`).
const $checkIntRanges = new Map([
  [$checkU8, [0, 0xff]],
  [$checkU16, [0, 0xffff]],
  [$checkU32, [0, 0xffffffff]],
  [$checkI8, [-0x80, 0x7f]],
  [$checkI16, [-0x8000, 0x7fff]],
  [$checkI32, [-0x80000000, 0x7fffffff]],
]);

// A tagged enum's value, its tag given back: one of `names`, a fieldless
// variant's string, or an object whose `key` is one of `objects`, a
// variant's of fields, or each variant's of an enum of a tag of its own.
function $checkTag(value, key, objects, names) {
  if (typeof value === "string" && names.includes(value)) return value;
  if (typeof value === "object" && value !== null) {
    const tag = $checkRead(value, key);
    // An accessor's, unchecked, its variant's fields with it: no variant's.
    if (tag === $checkAccessor) return tag;
    if (objects.includes(tag)) return tag;
  }
  const expected = [
    ...names.map((name) => JSON.stringify(name)),
    ...objects.map((tag) => `{ ${key}: ${JSON.stringify(tag)} }`),
  ].join(" | ");
  $checkFailed(TypeError, undefined, expected, value);
}

// A map's entries, each key by `key` and each value by `item`: a key's
// failure named `.keys()`, a value's by its key, `.get("tea")`.
function $checkMap(value, where, none, key, item) {
  if ($checkNone(value, none)) return;
  if (!(value instanceof Map)) $checkFailed(TypeError, where, "a Map", value);
  for (const [k, v] of value) {
    try {
      key(k);
    } catch (failure) {
      $checkAt(failure, "keys()");
    }
    try {
      item(v);
    } catch (failure) {
      $checkAt(failure, `get(${$checkShown(k)})`);
    }
  }
}

// A set's items, each by `item`, a failure named `.values()`.
function $checkSet(value, where, none, item) {
  if ($checkNone(value, none)) return;
  if (!(value instanceof Set)) $checkFailed(TypeError, where, "a Set", value);
  for (const v of value) {
    try {
      item(v);
    } catch (failure) {
      $checkAt(failure, "values()");
    }
  }
}

// A `&mut`'s box, `{ value }`, its `value` checked by `inner`.
function $checkCell(value, where, none, inner) {
  if ($checkNone(value, none)) return;
  if (typeof value !== "object" || value === null || !("value" in value)) $checkFailed(TypeError, where, "a { value }", value);
  const held = $checkRead(value, "value");
  if (held === $checkAccessor) return;
  try {
    inner(held);
  } catch (failure) {
    $checkAt(failure, "value");
  }
}

// A function JS gives Rust, `where` the parameter: a function, and where
// what it gives back is checked, `output`, the function wrapped, each call's
// result checked where it returns, `discounted(rule) returned -100,
// expected a u32`, its `name` and `length` the function's, as Rust may
// read them (`js::function::name`). One Rust ignores the result of, or
// that may give back anything, is given as it is.
function $checkCallback(f, where, output, none) {
  if ($checkNone(f, none)) return f;
  if (typeof f !== "function") $checkFailed(TypeError, where, "a function", f);
  if (output === undefined) return f;
  const checked = function (...args) {
    const result = f.apply(this, args);
    try {
      output(result);
    } catch (failure) {
      if (!(failure instanceof $CheckFailure)) throw failure;
      const at = failure.path === "" ? "" : ` (${failure.path.slice(failure.path.startsWith(".") ? 1 : 0)})`;
      throw new failure.Kind(`rust-js: ${where} returned ${$checkShown(failure.value)}${at}, expected ${failure.expected}`);
    }
    return result;
  };
  Object.defineProperty(checked, "name", { value: f.name });
  Object.defineProperty(checked, "length", { value: f.length });
  return checked;
}

// What a binding gives back, `result`, checked by `check`, its `None` the
// form's: the result, or the error of where it came from,
// `../ts/money.ts#format returned 42, expected a u32`.
function $checkResult(result, where, check, none) {
  if ($checkNone(result, none)) return result;
  try {
    check(result);
  } catch (failure) {
    if (!(failure instanceof $CheckFailure)) throw failure;
    throw new failure.Kind(`rust-js: ${where} returned ${$checkShown(failure.value)}, expected ${failure.expected}`);
  }
  return result;
}

// The brand set of the type `key` names, its crate's name, its
// disambiguator and its path, `app#1f2e::Email`: the objects Rust made of a
// type no other crate can make (ADR 1187), shared by every copy of the
// runtime, so a library's maker and its app's check read one set, and two
// versions of one crate, two types to Rust, two sets.
function $brandOf(key) {
  const brands = (globalThis[$brandsKey] ??= new Map());
  let set = brands.get(key);
  if (set === undefined) brands.set(key, (set = new WeakSet()));
  return set;
}

// `value`, an object Rust made of the type `key` names, branded so.
function $brand(key, value) {
  $brandOf(key).add(value);
  return value;
}

// A value of a type no other crate can make, `key`'s: one Rust made, of
// the type `name` names, "an Email"; JS's own object of its shape refused.
function $checkBrand(value, where, none, key, name) {
  if ($checkNone(value, none)) return;
  if (!$brandOf(key).has(value)) $checkFailed(TypeError, where, `${name} Rust made`, value);
}

// An `Option` whose `Some` is boxed, as its value might look like `None`,
// `Option<()>`, `Option<Option<u32>>` (ADR 0051): its `None` either JS's,
// else what its box holds, one level shallower, by `check`.
function $checkBoxedOption(value, where, check) {
  if (value != null) check($someValue(value), where);
}
