# 1025. A method read as a value is `js::method`'s, and JS's `||` is `js::or`

Status: Accepted (rd181, a port gap; P1; the user's decision to print
React's form exactly). Extends [0942](0942-whether-a-global-is-a-function-is-is-function.md)'s
path from a value, [0972](0972-a-function-that-reads-its-this-is-with-thiss.md)'s
JS function of `this` and [0740](0740-a-functions-members-are-read-of-any-function.md)'s
`function::call`.

Case ([0262](0262-when-rust-and-js-disagree.md)): none. `js::method` reads
the member JS reads, and `js::or` gives what JS's `||` gives; Rust
evaluates both of `or`'s operands, so one that may run code runs first, in
order, as `Option::unwrap_or`'s `??` does.

## Context

React DOM's setFocusIfFocusable calls

```js
(element.focus || HTMLElement.prototype.focus).call(element, focusOptions);
```

the prototype's method where an element has none of its own (IE11's SVG
elements). The port wrote `element.focus(None)`: no form read a method as
a value, none was JS's `||` of two values, and a binding's method alone,
`HTMLElement::focus`, as a value is an arrow of its Rust signature, `(self,
options) => self.focus(options)`, which `function::call` would give the
options as `self`. The user decided the port prints React's form exactly.

## Decision

**`js::method(path)` is the method a path from a value names, the JS
function itself:** `js::method((element, HTMLElement::focus))` is
`element.focus`, typed by the method's signature without its `this`,
`&'static dyn Fn(Option<FocusOptions>)`, as a `with_this` function is
(the trait `js::Method`, of each method of up to 12 parameters). It's the
path `js::is_function` asks of (ADR 0942), read: each step a member, the
value read as the path's first is. `function::call(f, Some(element),
args)` calls it with its `this`, as JS does. A getter may answer the read
where it may the method's call, of a value of any shape's type (ADR
0750).

**A path's value may be any value**, `js::Unknown`, its step a method of
any type, as TypeScript reads `any`'s members: `(js::class::<HTMLElement>().prototype(),
HTMLElement::focus)` is `HTMLElement.prototype.focus`. This holds for
`js::is_defined` and `js::is_function` too. A method of another type than
the value's is refused, as before.

**`js::or(a, b)` is `a || b`**, `T: Copy` of any two values, `a` where
it's truthy, else `b`: a binding of the operator, as `js::strict_equals`
is `===`. A `b` with effects is held first, `const fallback = ..; a ||
fallback`, and `a` before it where it has effects too.

**A getter binding that runs nothing, `#[rust_js::runs_nothing]`, is read
as a data property**, which no getter answers: `Class::prototype`, a
function's own `prototype`, so `HTMLElement.prototype.focus` isn't held
first as one that may run code.

```rust
function::call(
    js::or(
        js::method((element, HTMLElement::focus)),
        js::method((js::class::<HTMLElement>().prototype(), HTMLElement::focus)),
    ),
    Some(element),
    (focus_options,),
);
```

```js
(element.focus || HTMLElement.prototype.focus).call(element, focusOptions);
```

## Consequences

- One form reads a method as a value; `function::call` and `js::or` are
  the existing families' (a function's members, the operators). A
  binding's method alone as a value is still its Rust signature's arrow.
- Held by `test/lowering-is-function.test.ts`'s "js::method of a path
  reads its method as a value, js::or is JS's ||" (run: an element with
  its own `focus` and one without, called with its `this` and options;
  `js::or` of a call runs it either way) and "js::method of a method of
  another type is refused"; the guide's global-scope page. Mutations
  `or-other-unspilled`, `runs-nothing-read-a-getter` (`binding_calls.ts`),
  `method-path-any-value-refused`, `method-path-step-unread`
  (`mir/calls.ts`).
