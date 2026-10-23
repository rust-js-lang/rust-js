# 0664. A function's last slice marked `#[rust_js::rest]` is JS's rest parameter

Status: Accepted. Decided with the user, 2026-10-23. The defining side of
[0640](0640-a-rest-parameter-is-a-tuple.md), whose `impl js::Rest<T>` is
a binding's, the calling side; beside [0428](0428-c-variadic-functions.md),
whose C-variadic `...` is rustc's own.

Case ([0262](0262-when-rust-and-js-disagree.md)): N, a function's shape
to JS: the function is given the slice it would be in Rust, and a JS
caller gives its items as JS does.

## Context

React's build rewrites each `throw Error(`..`)` into a call of
`formatProdErrorMessage(code, a, b, ..)`, as many arguments as the
message has, and `shared/formatProdErrorMessage.js` reads them from
`arguments`. A Rust function's slice was one argument, `function f(code,
args)`, an array a JS caller had to make; nothing could write a function
JS calls with any number of them.

## Decision

**A function's last parameter, a shared slice marked
`#[cfg_attr(rust_js, rust_js::rest)]`, is JS's rest parameter:
`...args`, the array of what it's given past the others.** The slice of
any element type a slice parameter takes.

```rust
js::export_default! {
    fn format_prod_error_message(
        code: f64,
        #[cfg_attr(rust_js, rust_js::rest)] args: &[Option<&Unknown>],
    ) -> String {
        let mut url = format!("https://react.dev/errors/{code}");
        for (i, arg) in args.iter().enumerate() {
            url += if i == 0 { "?args[]=" } else { "&args[]=" };
            url += &js::encode_uri_component(&js::string(*arg));
        }
        format!(
            "Minified React error #{code}; visit {url} for the full message or \
             use the non-minified dev environment for full errors and additional \
             helpful warnings."
        )
    }
}
```

```js
export default function formatProdErrorMessage(code, ...args) {
  let url = `https://react.dev/errors/${$displayF64(code)}`;
  for (const [i, arg] of args.entries()) {
    url += i === 0 ? "?args[]=" : "&args[]=";
    url += encodeURIComponent(String(arg));
  }
  return `Minified React error #${$displayF64(code)}; visit ${url} for the full message or use the non-minified dev environment for full errors and additional helpful warnings.`;
}
```

- **Where it's allowed**: a function's own, a `pub fn`'s, a
  `js::export_default!`'s, a private one's, an inherent method's, an
  `async fn`'s, one named by `#[rust_js::name]`, as any function
  is. Each refused elsewhere, by an error that says what it is:
  - not the last parameter: "a function's last parameter, as JS's rest
    parameter is";
  - not a shared slice, a `Vec<f64>`, a `&mut [f64]`, whose writes JS's
    fresh array would lose, or a `&[f64; 2]`: "a slice's, `xs: &[T]`";
  - a pattern, `&[..]`: "a parameter's name, `args: &[T]`";
  - a closure's: what calls it gives what its type says, a callback's
    binding;
  - a trait method's, its declaration's or an impl's: a call through the
    trait gives each impl alike;
  - a component's props, which React gives as one object;
  - a function given a dictionary, a type's fact, a drop or its caller's
    location, each written after its parameters, which a rest can't come
    before (as ADR 0428 refuses a C-variadic one's).
- **A call from Rust spreads the slice**, as a call of a binding's rest
  does (ADR 0640): `total(&v)` is `total(...v)`, `total(&v[1..])`
  `total(...$slice(v, 1))`; an array written out is its items,
  `total(&[a, b])` is `total(a, b)` and `total(&[])` `total()`. Here the
  array written out is the one way to give arguments written out: a
  slice parameter takes no tuple, so rustc has no other. One answer,
  `calls::spread_rest`, gives a binding's rest and a function's.
- **As a value it's the arrow of its call**, `(x) => total(...x)`, which
  a caller through a `fn` pointer or an `impl Fn` gives the slice;
  `js::import!` of one is refused, as what it gives is called with the
  slice whole.
- **A variable holding the function is called as the function**, `let
  named = total; named(&[x])` is `total(x)`, its rest spread: rustc's
  variable of a function item is the item, and its call is the item's.
- **Its `.d.ts` declares a rest parameter** of the slice's type,
  `...xs: number[]`, `...args: (unknown | null | undefined)[]`; the
  `Option`s before it are optional still.
- **Across crates, the library's manifest says so**, `rest: true`:
  rustc's metadata keeps no parameter's attributes. A library a compiler
  wrote before had none, as no function was one.
- **One place answers which functions take one**: `bindings::marked_rest`
  reads the attribute from the body's parameters, `bindings::takes_rest`
  asks it of the crate's function and `FnCx::takes_rest` of a library's
  too, which the definition (`gather_rest`), each call (`rest_given`),
  the function as a value, the `.d.ts` and the manifest ask.
  `analysis::validation::reject_rest_misuse` refuses it elsewhere before
  anything is lowered.

## Why

- **It's JS's signature**, `function formatProdErrorMessage(code,
  ...args)`, which React's build and any JS caller call with the
  arguments themselves.
- **It's Rust's**: the function is given a slice, as native Rust gives
  it, and the body reads it as one; a Rust caller writes the call it
  would anywhere, `f(code, &args)`.
- **The attribute is where the meaning is**, on the parameter, as
  `#[cfg_attr(rust_js, ..)]`, so a plain `cargo check` and rustfmt take
  the program as it is.

## Alternatives

- **The attribute on the function, `#[rust_js::rest(args)]`**, as a
  binding's `#[rust_js::nullable(..)]` is, because an `extern` block and
  another crate's metadata keep no parameter's attributes: a function of
  the crate's has its body's, and the manifest carries it across crates.
- **`impl js::Rest<T>`, a binding's form**: a tuple of each argument's
  own type, which a function's body couldn't read without generics of each
  count.
- **Every last slice of an exported function a rest**: a JS caller of
  `total(prices)` gives an array, which the guide promises.
- **Refusing `&[a, b]` written out, as ADR 0640 does for a binding**: a
  binding has the tuple for it; a slice parameter has no other form, so
  the function couldn't be called with values written out.

## Consequences

- A JS caller that gives an array as the last argument gives one item:
  the rest is what it's given, as JS's is.
- `encodeURIComponent` takes what TypeScript's does, `string | number |
  boolean` (ADR 0229), and React's JS gives it any value, which neither
  takes: the port's `unknown` is `js::string(*arg)`, `String(arg)`, where
  upstream has `arg`, as a TypeScript port's would be.
- An `f64`'s `{code}` is `$displayF64(code)`, Rust's text of it (ADR
  0312's amendment), where React's integers read the same as JS's
  `${code}`; `js::shown(code)` is how a program asks for JS's text.

## What holds it

- `test/rest-parameters.test.ts`: the port of `formatProdErrorMessage`,
  its JS and `.d.ts`, run against upstream's with no, one and three
  arguments past the code, and a function's rest called with an array
  written out, a slice held, a subslice and none, a method's, a `fn`
  pointer's, an `impl Fn`'s and a variable's, each run; a library's rest
  called by another crate; and each refusal, by its message.
- Mutations in `scripts/mutations/lower/calls.ts` (a call given the slice
  whole, a library's not known), `analysis/validation.ts` (each refusal
  allowed), `mir.ts` (not gathered, a dictionary after it), `lower.ts`
  (the function as a value), `mir/dispatch.ts` (a variable's call),
  `binding_calls.ts` (an import), `declarations.ts` (declared an array),
  `library.ts` (the manifest's) and `bindings.ts` (the attribute read).
