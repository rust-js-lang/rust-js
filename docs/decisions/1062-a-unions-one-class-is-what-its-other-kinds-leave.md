# 1062. A union's one class is what its other kinds leave

Status: Accepted (rd506's audit, P1, 2026-11-02; the coordinator's
decision on the user's behalf). Amends
[0214](0214-untagged-enums.md) (an untagged enum's variant is told
apart by its value's kind). Amended (#555, P0): two classes no longer
keep `instanceof`; a DOM node is told by its `nodeType`, and any other
class `instanceof` would tell is refused, below.

Case ([0262](0262-when-rust-and-js-disagree.md)): results change, so
Rust's behavior is kept. A test of `instanceof File` where the host has
no `File` global throws a `ReferenceError`, where Rust's `match` runs on.

## Context

A `match` of an untagged enum tests its arms in the order the program
wrote them, the last what's left. A form's entry, webapi's
`FormDataEntryValue`, is a `File` or a string:

```rust
match data.get(name) {
    Some(FormDataEntryValue::File(_)) => 1,
    Some(FormDataEntryValue::Str(_)) => 2,
    None => 0,
}
```

printed `if (get instanceof File) { return 1; } return 2;`. A host
without the `File` global, Node 18, a worker of an older engine or a
test's environment, throws `ReferenceError: File is not defined` there,
though a `typeof get === "string"` tells the two apart with no global.
React's Flight reply client met it of getReader's union of readers,
`reader instanceof ReadableStreamDefaultReader`, in Jest's environment,
which has `ReadableStream` but not its readers (rd506, P1).

## Decision

**Where an untagged enum's one variant is a class and each other is a
kind JS tells by the value alone, the class's branch is the last, what's
left, and each other's is tested**: `typeof`, `Array.isArray`, a tuple's
length, a literal, `undefined` and `null`
(`Recognition::lone_class_variant`, consumed by `mir/structure.rs`'s
switch).

```js
if (get !== null) {
  if (typeof get === "string") {
    return 2;
  }
  return 1;
}
return 0;
```

- **Two classes or more keep `instanceof`** (superseded, #555, below):
  one of them has to be
  tested by its class. webapi's unions of them are the DOM's, IDB's,
  CSSOM's and canvas's (`RenderingContext`, `HTMLCollectionOrElement`,
  an `IDBRequest`'s source), whose classes a host has wherever it has
  the API that gives them.
- **An object, a test function's or a member's kind keeps its test**:
  an object's leaves a class out by `instanceof` too.
- **The arms the others take stay in the order the program wrote them**;
  only the class's moves to the end, where it's what's left.

## Alternatives

- **`typeof File !== "undefined" && get instanceof File`**: a guard
  upstream never writes, before every class test.
- **The program's order, the class first**: the `ReferenceError`.

## Consequences

- A `match` whose first arm is the class reads its arms in another order
  in JS: the class's last.
- Held by `test/webapi-objects.test.ts` ("a union of one class is told
  apart by the others' tests, no instanceof of a global"), run with
  `File` deleted from the global object, and the mutations
  `lone-class-tested` (`untagged.rs`) and `lone-class-branch-dropped`
  (`mir/structure.rs`).

## Amendment: a class is told in any realm, or refused (#555, P0, 2026-11-04)

Case ([0262](0262-when-rust-and-js-disagree.md)): results change, so
Rust's behavior is kept, or the program refused where it can't be (D).
This supersedes the decision above that **two classes or more keep
`instanceof`**.

An untagged enum of two binding classes whose arms differ was told
apart by `instanceof` of the global class:

```rust
match source {
    DocumentOrElement::Document(_) => "document".to_string(),
    DocumentOrElement::Element(element) => element.tag_name(),
}
```

```js
return source instanceof Document ? "document" : source.tagName;
```

That premise, "a host has the classes wherever it has the API that
gives them", fails twice. A document of another realm, a frame's
`contentDocument` or a JSDOM window's in a browser test, is no
`instanceof` the global `Document`, so its `tagName`, `undefined`, was
returned where Rust returns `"document"`: a wrong answer. And Jest's node
environment with a JSDOM window, Node or a worker has no `Document`, so
the test threw a `ReferenceError` where Rust runs on.

**Each category of class has its rule** (`untagged.rs`):

- **A DOM node class is told by its `nodeType`**, `source.nodeType ===
  9`, as React tells one: the same in any realm, and of no global. webapi's
  generator writes it from WebIDL, `#[rust_js::test(nodeType = 9)]`, of
  each interface below `Node` every interface under which, in any spec,
  has its own `Node` constant, `DOCUMENT_NODE` of `Document`, or its
  parent's: `Element` and each element 1, `Document` 9, `DocumentFragment`
  and `ShadowRoot` 11, `Comment` 8, `DocumentType` 10, `ProcessingInstruction`
  7, `CDATASection` 4, and `Text`, which a `CDATASection` is too, `3 or 4`,
  `node.nodeType === 3 || node.nodeType === 4`. `Node`, `CharacterData` and
  `Attr`, of no constant of their own name, have none. A JS object type's
  `test(key = n, ..)` is a member of those numbers, `untagged::Kind::Member`
  of `Kind::Constants`, as a struct's member tells it (ADR 1036): told from
  an object's, a primitive's and each other's.
- **Two classes of one `nodeType` are refused where they're told apart**,
  an `HTMLElement` and an `SVGElement`, each 1, which only `instanceof`
  told apart: "`Svg` and `Html` are each an object whose `nodeType` is the
  number 1, which JS can't tell apart in an untagged enum in every realm
  ..".
- **Any other binding class told by `instanceof`, its own test or one an
  object's leaves it out by, is refused where it's tested**: a web
  platform class, `Blob`, `ReadableStreamDefaultReader`, an observer;
  ECMAScript's own, `Date`, `RegExp`, `Promise`, buffers, errors, `Intl`,
  whose globals a host has, but whose `instanceof` is `false` of another
  realm's, a frame's date or Node's `Uint8Array` in Jest's JSDOM
  environment; and a program's binding of a global class. The error names
  the variant and its class and says to tell them apart in Rust, a tagged
  enum. Only a test that's made is refused: a `match` whose arms do the
  same JS, which tests nothing (ADR 0214, rd687), an `IntersectionObserver
  | ResizeObserver`'s `observer.observe(child)`, and a `match` that tests
  only a variant JS tells soundly, webapi's `BodyInit::Str(text)` beside
  `_`, `typeof body === "string"`, are made as before. The `match` refuses
  where it's written, once its arms are known to differ; a derived `==`,
  a clone and a drop where their function is.
- **The one class beside kinds JS tells by the value alone is what they
  aren't wherever it's tested**, a derived `==`'s too, `!Array.isArray(a)`,
  not only a `match`'s last branch, as above. A DOM node class is one
  here, so `Element | string` still tests `typeof` alone.
- **std's `Map` and `Set` keep `instanceof Map`**: values only the
  program makes, of ECMAScript's own globals, in its own realm.
- **A class the crate defines itself, `#[rust_js::class]`, is an
  object**, a struct of fields, told by a member it declares or refused
  beside another object, as before (ADR 1036): never `instanceof`.

### Alternatives

- **`typeof Document !== "undefined" && source instanceof Document`**:
  fixes the missing global, not the realm.
- **`Object.prototype.toString.call(v) === "[object Document]"`**: a
  realm-free brand, but exact, not of a subclass, `[object HTMLDocument]`
  of a page's document; and JS no one writes for a DOM node.
- **`instanceof` kept for ECMAScript's classes**: their global is there,
  but the realm's answer differs, which a JSDOM test meets of Node's
  buffers.

### Consequences

- webapi's 154 node classes carry `#[cfg_attr(rust_js,
  rust_js::test(nodeType = ..))]`; js's `WaitAsyncValue` is `Promise |
  string`, its `Ended` no `otherwise`, so the promise is what the string
  isn't.
- Tests: `test/webapi-objects.test.ts`, "an untagged enum's DOM nodes are
  told by their nodeType, in any realm and with no global", run with two
  happy-dom windows as two realms, the nodes one's, and no globals, then
  the other's; and its refusals of `HTMLElement | SVGElement` and `Blob |
  Element`; "a union's one class is what the others aren't in a derived
  ==, no instanceof"; `test/diagnostics.test.ts`, "an untagged enum of
  two classes told apart" (`Date | RegExp`) and "a JS object type's test
  of a member of no number"; `test/lowering-switches.test.ts`'s alike
  arms built and the differing ones refused; `test/compiler.test.ts`'s
  #391 classes, each lone, and `URL | Date` refused naming `URL`.
- No corpus case: what this tells is a JS object type's class, which
  native Rust can't make.
- Mutations: `node-type-untold`, `node-type-of-one-value`,
  `node-types-shared-apart`, `untold-class-unrefused`,
  `lone-class-tested-in-impls`, `untold-refused-in-arms-tests`,
  `test-member-misuse-allowed` (`untagged.ts`); `untold-refused-of-alike-arms`,
  `untold-of-untested-arms`, `untold-unrefused-in-match`
  (`mir/structure.ts`). A class's test that left out its subclasses,
  `instanceof Error && !(instanceof TypeError)`, is made of no enum now,
  so it's gone, and its mutations `untagged-class-takes-subclass` and
  `untagged-subclass-left-out-twice` with it; `deref-target-unknown` is
  caught by rd232's method of a class a `Deref` extends.
