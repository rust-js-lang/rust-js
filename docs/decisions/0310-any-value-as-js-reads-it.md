# 0310. Any value is read as JS reads it

Status: Accepted. Extends the js crate (ADR 0102) and its `Unknown` (ADR
0225). Amended by [0639](0639-concat-text-of-any-value-may-throw.md):
`concat_text` of a value of any shape gives a `Result`, as it may throw.
Extended by [0860](0860-the-iterator-protocol-and-a-loop-of-what-its-test-makes.md):
`number::to_string(x, radix)`, its radix TypeScript's `radix?`.

Case: A ([0262](0262-when-rust-and-js-disagree.md)).

## Context

react.dev's Console formats what a page logs, of any type, as the browser's
console does:

```ts
formatted = formatted.replace(REGEXP, (match, escaped, ptn, flag) => {
  let arg = args.shift();
  switch (flag) {
    case 's': arg += ''; break;
    case 'd': arg = parseInt(arg, 10).toString(); break;
  }
  ..
});
children = JSON.stringify(msg, null, 2);        // in a try
children = Object.prototype.toString.call(msg); // in a try
children = '[' + typeof msg + ']';
```

The js crate had none of these: `replace` took only text; `parseInt` and
`parseFloat` only a `&str`; no `value + ""`, `typeof`, number `toString`,
or `Object.prototype.toString.call`; and `stringify_with` only a value whose
JSON is exact.

## Decision

**The js crate reads any value as JS does: `reg_exp::replace` takes text
or a closure of the match and its groups; `parse_int` and `parse_float`
take any value, which JS makes text; `concat_text` is `value + ""`;
`type_of` is `typeof value`; `number::to_string` is a number's
`toString`; `object::to_string` is `Object.prototype.toString.call`; and
`json::stringify_with` of an `Unknown` gives what may throw or be
`undefined`.**

```rust
reg_exp::replace(text, pattern, |matched: &str, escaped: Option<&str>, _ptn: Option<&str>, flag: Option<&str>| {
    ..
})
number::to_string(js::parse_int(arg, 10))
json::stringify_with(msg, None, 2) // Result<Option<String>, &JsError>
```

```js
text.replace(pattern, (matched, escaped, _ptn, flag) => { .. });
parseInt(arg, 10).toString();
$try(() => JSON.stringify(msg, null, 2));
```

- **One `replace`**, as TypeScript's overloads: its `with` is text, or a
  closure of the match and up to 9 groups, each `None` where it matched
  nothing, giving text or any value. ReScript splits it by arity,
  `replaceRegExpBy0` .. `By5`; one function is one spell for the job.
- **`ToText`** is what JS takes as text where it wants text, which
  TypeScript's `string` is given: text, or any value, `None` too.
- **`value + ""`** is its own binding, `#[link_name = "+ \"\""]`, not
  `String(value)`: `+` asks an object's `valueOf` first and throws of a
  symbol. (Amended by ADR 0639: of a value of any shape it gives a
  `Result<String, &JsError>`, by `ToText`'s `Text`, a `String` of text
  and numbers; it gave a `String` of each.) **`typeof`** is a link form
  too, `#[link_name = "typeof"]`, as `instanceof` is.
- **Whether a binding may throw is of the call's types**: the instantiated
  signature's `Result`, so `stringify_with` of an `Unknown` runs in a `try`
  and of a `JsonText` doesn't.

## Why

- **It's the JavaScript a person writes**: react.dev's own calls, each as
  written.
- **It's the same program**: each is JS's own operation on the value.
- **It's tested**: a bindings test runs each of Console's, of text, numbers,
  `undefined`, objects, a function, a cycle and a `BigInt`; mutations make
  `typeof` and `+ ""` calls, and read a binding's throwing from its generic
  signature.

## Amendment (2026-11-01): `js::to_number`, JS's unary `+` (rd331c)

react-art's Surface makes its surface of `Mode.Surface(+width, +height,
this._tagRef)`, `width` a prop of any shape. No binding was JS's unary
`+`, and `Number(width)` isn't it: `Number(1n)` is `1`, where `+1n`
throws a `TypeError`, as it does of a symbol. Case C
([0262](0262-when-rust-and-js-disagree.md)): results would change, so JS's
own operator is bound.

**`js::to_number(value)` is `+value`**, a link form of its own, `"+"`,
beside `concat_text`'s `"" +`, its result by `ToNumber`'s associated type,
as `concat_text`'s is by `ToText`'s (ADR 0639): of text, `&str` or a
`String`, an `f64`, `+"0x10"` 16 and `+""` 0, where `str::parse` is an
`Err`; of a value of any shape, `&Unknown`, `Option<&Unknown>` or
`&JsObject`, `Result<f64, &'static JsError>`, whose `unwrap()` is the call
(ADR 0643), `+width`, what JS threw going on as itself, and whose `match`
or `is_ok()` is JS's `try`. Not of a number or a `bool`, whose `+` Rust
writes already, `x` and `f64::from(u8::from(b))`. Of a reference to what it takes,
its value's, as a field taken apart from borrowed props is, `let
SurfaceProps { width, .. } = self.props();`'s `&&Unknown`.

Rejected: `js::number`, a name a reader takes for `Number(value)`; and
`Number(value)` itself, whose `BigInt` result differs. Held by
`test/js-any-values.test.ts`'s "to_number is JS's unary +, which throws of
a BigInt", run in Node of text, objects, `undefined`, a `BigInt` and a
symbol, and the mutations `to-number-of-number`
(`lower/binding_calls.rs`), `unary-plus-a-call` (`lower/bindings/forms.rs`)
and `unary-plus-negates` (`to_oxc/expressions.rs`).
