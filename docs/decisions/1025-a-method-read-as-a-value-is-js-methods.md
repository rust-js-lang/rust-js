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

## Amendment: a step of a class the value extends, and a path held before a guard's `try` (rd232, rd233)

React DOM's hideInstance asks `typeof style.setProperty === 'function'` of
an element's `style`, TypeScript's `CSSStyleProperties`, whose
`setProperty` is the `CSSStyleDeclaration` it extends (its `Deref`):
`js::is_function((style, CSSStyleDeclaration::set_property as fn(..)))`
was refused, the step no method of the value's own type. **A step may be a
method of a class the value's type extends, as its `Deref` says**
(`Recognition::superclasses`), for every path, `js::is_defined`'s,
`js::is_function`'s and `js::method`'s.

setFocusIfFocusable reads `(element.focus || HTMLElement.prototype.focus)`
while a guard that removes its focus listener is live: the call may unwind
past the guard, so the path's tuple was held before the guard's `try`,
`const _15 = [element, ..]`, and its value no longer written in the call:
refused. **A path's value held so is the variable the tuple was made of,
read again** (through the copies and reborrows MIR makes of it), nothing
having run between; one of another value is still refused. Its JS reads
`element.focus` before the `try`, `const focus = element.focus; try {
(focus || HTMLElement.prototype.focus).call(element, focusOptions); }
finally { .. }`, where upstream reads it inside: the read of a host
object's member is held before the class's read (temporaries' `admit`),
a readability difference left open.

Held by `test/lowering-is-function.test.ts`'s "js::is_function of a path
from a value asks its member" (`hide_element`) and "js::method of a path
reads its method as a value" (`guarded`, run: the guard dropped each
time), and mutations `path-step-superclass-refused`,
`path-value-held-refused` (`mir/calls.ts`).

## Amendment: a member by its key, called as its value's method (rd334)

Case ([0262](0262-when-rust-and-js-disagree.md)): none. JS's
`value.name(..args)` reads the member once, gives it the value as its
`this`, and throws a `TypeError` where it's no function, before any
argument runs: what `function::call(js::method((value, name)),
Some(value), args)` says, the member read, then called with that `this`.

### Context

react-art sets functions on art's nodes, expandos art doesn't declare,
and calls them as methods: `instance._applyProps(instance, props)`,
`instance._subscriptions[type]()`, `listener.handleEvent(event)` of an
`any` listener, `props.fill.applyFill(instance)`. A value of any shape
had no form for it. `js::method` read only a binding's method, and the
port cast what `js::get` read, `unsafe { js::cast::<ApplyProps>(js::get(instance,
"_applyProps"))(instance, props, None) }`, or wrote
`function::call(cast, Some(&listener), (event,))`, which printed
`listener.handleEvent.call(listener, event)`.

### Decision

**`js::method` of a value of any shape and a key, a `PropertyKey`, is the
member that key names**, `js::method((listener, "handleEvent"))`,
`listener.handleEvent`, as `js::get` reads it (a getter may answer it),
typed as a function of any shape, `Kind::Function`'s `&'static dyn Fn() ->
Option<&'static Unknown>` (the `js::Method` impl for `(T, &K)`, its marker
`sealed::Keyed`, no method's `fn` pointer). The existing form takes the
key, not a new function: a member's call is `function::call`'s, its
`this` given, as a method read by `js::method` already was.

**`function::call` of a `js::method` whose `this` is `Some` of the path's
value, the same variable, is the method's call**: `listener.handleEvent(event)`,
`subscriptions[type]()`, of a key; `element.focus(options)` of a binding's
method, which printed `element.focus.call(element, options)`. The body
query `method_calls` finds each, the `function::call` and its
`js::method` by their spans (`BodyFacts::method_calls`,
`called_methods`); the call's `f.call(value, ..)` is marked
`Known::OwnMethodCall`, and preparation writes it `value.f(..)` where
the method is read in place of the variable its `this` is
(`Expr::own_method_call`), which a call's `this` is then, as `call`
gives it, and read before its arguments either way. A read the lowering
held before an argument whose statements run first, `const handleEvent =
listener.handleEvent; .. handleEvent.call(listener, sum)`, stays so: a
variable's call gives no `this`. Of another `this`, or of a variable
changed between, `f.call(other, ..)`, as before.

**A key's `js::method` read anywhere else is refused**, naming the form:
the member may be no function, so a `&dyn Fn` of it held, given on or
tested would be a lie (`Some(f)` of `undefined` is `None`). `js::get`
reads a member as a value, and `js::classify`'s `Kind::Function` holds one
that's a function. Of a `this` that isn't the path's variable, a key's
read is refused too: JS's `f.call(other)` would call an object's own
`call` where the member is no function, which `function::call` doesn't
mean.

### Alternatives

- **A new function, `js::call_method(value, key, args)`.** It would read
  the value once, `props.fill.applyFill(instance)` with no variable, but
  it's a second way to call a function with a `this`, beside
  `function::call`: refused by the one-way rule.
- **Calling a key's `js::method` directly, `js::method((v, k))()`.** Its
  type takes no arguments, so it would be a second way for calls of none.
- **Two reads written as one.** `function::call(js::method((js::get(props,
  "fill"), "applyFill")), Some(&js::get(props, "fill")), ..)` reads
  `props.fill` twice in Rust; printing one read would drop a getter's
  second run. Left to the user (rd334's report).

### Consequences

- A value that isn't a variable is held in one to call its member as its
  method: `let subscriptions = js::get(instance, "_subscriptions");` is
  `const subscriptions = instance._subscriptions;` before
  `subscriptions[type]()`, where upstream reads it in the call.
- Held by `test/lowering-is-function.test.ts`'s "function::call of
  js::method whose this is its value is the method's call" (run: the
  member read once through a getter, its `this` and arguments, a
  `TypeError` of a non-function and of a missing key, an object's own
  `call` member not called, the read before an argument that runs code,
  and before one whose statements run first or that `?` returns early
  of, held; a binding's method of
  its value and of another, and of a variable changed in the `this`) and
  "js::method of a key read other than as its value's method is refused";
  the guide's types page. Mutations `method-call-other-variable`,
  `method-call-function-unrecognized` (`body_queries/method_calls.ts`),
  `method-call-unmarked` (`binding_calls.ts`), `own-method-call-of-variable`,
  `own-method-call-other-this` (`js/make.ts`), `keyed-method-anywhere`
  (`mir/calls/method_paths.ts`). A key's read is a getter's, as `js::get`'s, though
  only a call's callee, read before what it's given either way, has it.
