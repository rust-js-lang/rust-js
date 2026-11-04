# 1075. A function type's parameters are named as the Rust names them, and its trailing `Option`s may be left out

Status: Accepted (#543, #544, 2026-11-03; the user's decision, option A, for
a `dyn Fn` field's names); amended (#630): a static's or a constant's
function type is named as the functions its value may be, a filler that
names nothing says nothing, and an alias in a `dyn Fn` or a private alias
is named. Extends [0196](0196-typescript-declarations.md),
whose function type named each parameter after its type, and
[0443](0443-a-value-a-caller-may-leave-out-is-an-option.md), whose
optional trailing `Option`s were a function's alone.

Case ([0262](0262-when-rust-and-js-disagree.md)): None. A parameter's name
in a declared function type means nothing at run time, and an optional
`?` says what a caller may already do (ADR 0313).

## Context

alien-signals' port declares its system's functions as `dyn Fn` fields,
filled once, in `create_reactive_system`:

```rust
pub struct ReactiveSystem<K> {
    pub link: Rc<dyn Fn(&Node<K>, &Node<K>, u32)>,
    pub unlink: Rc<dyn Fn(&Rc<Link<K>>, Option<&Node<K>>) -> Option<Rc<Link<K>>>>,
    pub propagate: Rc<dyn Fn(Rc<Link<K>>, bool)>,
    ..
}

ReactiveSystem {
    link: {
        let system = Rc::clone(&system);
        Rc::new(move |dep, sub, version| system.link(dep, sub, version))
    },
    ..
}
```

Its `.d.ts` named each parameter after its type, and wrote a trailing
`Option` as required:

```ts
link: (node: ReactiveNode<K>, node2: ReactiveNode<K>, value: number) => void;
unlink: (link: Link<K>, value: ReactiveNode<K> | null | undefined) => Link<K> | null | undefined;
propagate: (link: Link<K>, value: boolean) => void;
```

Upstream's is `(dep, sub, version)`, `(link, sub?)` and `(link,
innerWrite)`.

Rust's `Fn(A, B)` sugar can't name its parameters: `dyn Fn(dep: &Node)`
doesn't parse. A fn-pointer type can, `fn(dep: &Node, sub: &Node)`, and
HIR keeps those names, but they were dropped too. The port can't use one,
because its functions capture `system`. The names it gives are those of
the closures that fill the fields.

And the rule that a trailing `Option` may be left out (ADR 0443) was
`function`'s alone. A function type takes it the same way in each
direction:
- **A JS caller of a Rust closure may leave it out.** The closure reads
  the missing `undefined` `!= null`, as `None`.
- **rust-js leaves it out of its own calls.** A call of a function value
  leaves out a trailing `None`, which is `Absent` (ADR 0313, amended):
  the port's `this$.getter()` of `getter(None)`. So a callback JS gives
  must take it as missing.

## Decision

**A function type's parameter is named as the Rust names it.** Where the
Rust names none, it's named after its type, as before.

- **A fn pointer's names are those written in its type** (HIR's
  `FnPtrTy::param_idents`). `fn(count: u32, _sum: u32)` is `(count:
  number, sum: number) => number`. Each name is camelCase, without the
  leading `_` of one unused where what's left isn't a word JS keeps
  (`written_parameter_name`, beside `parameter_name`, in `parameter_names.rs`, which they moved to out of `lower.rs`, at its cap). The
  names are gathered as the type's aliases are (`as_written`), so a fn
  pointer anywhere in a written type has them: in an `Option`, an `Rc`,
  another fn pointer's parameters.
- **A field's `dyn Fn` is named by what fills it, where every literal of
  its struct or variant the crate writes agrees.** That's the closure or
  the function in the value each literal gives the field, before it's
  coerced: `Rc<{closure}>`, `Box<fn item>`, `Some(Box<..>)`, a block's
  last value. Each name is the one that function's own declaration gives
  it (`parameter_js_name`: `_sub` is `sub` as ADR 0678 has it, `self` is
  named after its type). At each position:
  - Where every literal names the parameter alike, that's its name.
  - Where one names it otherwise, names it `_` or by a pattern, or fills
    the field with what names nothing (a parameter's `Rc<dyn Fn>`, a
    `None`), it's named after its type.
  The port's `ReactiveSystem` is `link: (dep, sub, version)`, `unlink:
  (link, sub?)`, `propagate: (link, innerWrite)`, `checkDirty: (link,
  sub)`, as upstream's is. A fn pointer's written names win over its
  literals'.
- **A function type's trailing `Option`s are optional**, `sub?:
  ReactiveNode<K> | null | undefined`, as a function's are. The rule is
  one function, `optional_from` (`declarations/functions.rs`), which both
  `function` and `function_type` ask: trailing `Option`s, before a rest
  parameter too, but not one that's nullable. A JS class's constructor,
  `Class<fn(A) -> T>`, is the exception. rust-js gives it each `None` as
  `undefined` (ADR 0330), and the class is JS's, so its parameters stay
  as they are.

One owner holds the naming: `declarations/functions.rs`, beside
`function`'s own parameter naming. `filled_names` reads the crate's
literals once per module's declarations, when a function-typed field is
first declared, through `aggregates::each_struct_literal`, the one walk
of them, which `struct_literals` counts them by too. `function_names` is the stack of names for the type being
declared, pushed by `as_written` and `fill_names` and read by `ts`'s
fn-pointer and `dyn Fn` arms.

**A library's fields are named by its own literals.** A library's
declarations are written when the library is compiled, and its consumers'
literals are compiled after it, in their own crates. So names can't
travel from a consumer to the library's `.d.ts` through the manifest:
they'd go the wrong way. A struct the library declares and only its
consumers fill is named after its types.

## Why

- **It's the program's names.** Rust has two places that name a function
  type's parameters: a fn pointer's type, and the function a field is
  filled with. A person reading the port names the field's parameters as
  the closure does.
- **It's one rule, not one per type.** Optional parameters are
  `optional_from`'s, for a function and a function type alike. And a
  parameter is named as `function` names it, so a closure's `_x` and a
  method's `self` read the same in each.
- **It's what TypeScript would write**: the port's `system.d.ts` reads
  upstream's names and `sub?:`.

## Rejected

- **A rust-js attribute naming a `dyn Fn`'s parameters.** It would be
  dialect, and a second way to name a parameter where the program already
  names it.
- **The names of the arguments at a call**, `getter(old_value)`. A call
  gives values, not names, and two calls give two.
- **Names from a field's first literal alone.** A second literal that
  names them otherwise makes them no one's.

## Consequences

- A struct the crate fills in two ways is named by its types where they
  disagree, as `link: (dep, node, value)` is in the test.
- An `impl Fn` parameter is filled by its callers, which are JS's, so it's
  named after its types: alien-signals' `computed(getter: (value?: ..) =>
  ..)`, where upstream writes `(previousValue?: T)`. So is a struct that
  another module fills with names other than upstream's.
  `ReactiveSystemOptions`' `notify: (effect)`, `update: (node)`, where
  upstream's type says `(sub)`, is accepted in the port's README.
- A written type that holds two fn pointers of one signature with
  different names names both by the last's, as its aliases are found.

Held by `test/declarations-function-types.test.ts`, which has fn pointers,
`dyn Fn` fields filled alike and otherwise, a variant's field filled in a
block, an `impl Fn`'s trailing `Option`, and TypeScript calling each with
its `Option` left out, and a JS class's constructor of a fn pointer.
Mutations in `lower/declarations/functions.ts`, `lower/declarations.ts`,
`lower/declarations/aliases.ts` and `lower/parameter_names.ts`, where
`lower.rs`' parameter naming moved with its mutations.

## Amended (#630): a static's value, what names nothing, and aliases inside

Case ([0262](0262-when-rust-and-js-disagree.md)): None, as above.

React's SchedulerNative declares what it exports as thread-locals of a
private alias of a `dyn Fn`:

```rust
type ScheduleCallback = dyn Fn(PriorityLevel, Callback, Option<&Options>) -> Rc<Task>;

thread_local! {
    pub static UNSTABLE_SCHEDULE_CALLBACK: &'static ScheduleCallback = if js::is_defined(native_runtime_scheduler) {
        unsafe { js::cast(..) }
    } else {
        &scheduler::unstable_schedule_callback
    };
}
```

Its `.d.ts` was `unstable_scheduleCallback: (value: number, callback:
Callback, value2?: ..) => Task`, where upstream's is `(priorityLevel:
PriorityLevel, callback: Callback, options?: ..) => Task`, and
`unstable_getCurrentPriorityLevel: () => number`, of `dyn Fn() ->
PriorityLevel`.

- **A static's or a constant's function type is named as the functions its
  value may be name them** (`fill_value_names`, `given_values`), as a
  field's is by its literals: its initializer's value, a thread-local's
  `init` function's (`analysis::init_of`, ADR 0037), through its blocks,
  its `if`s' and `match`es' branches and a `&` of one, each the closure or
  the function it is before it's coerced. `pub static SCALE: fn(f64, f64)
  -> f64 = scaled;` is `(value, factor)` of `scaled(value, factor)`.
- **What's given that names nothing says nothing of the names**, a field's
  literal's and a static's branch's alike (`agreed_names`, the one rule of
  both): a host's `js::cast`, a parameter's `Rc<dyn Fn>`, a `None`.
  Where the functions given name a parameter alike, that's its name; where
  two name it otherwise, or none is given, it's named after its type, as
  before. This changes the rule above, "or fills the field with what names
  nothing": #543's `given` field, filled with `system`'s parameter in one
  literal and `doubled(n)` in the other, is `(n: number) => number`, where
  it was `(value: number)`. A parameter's or a cast's function has names of
  its own no one wrote here, so it can't disagree with the ones written;
  leaving every name to the types for it named less than the program does.
- **An alias written inside a `dyn Fn`, its parameters and its result, and
  inside a private alias of the crate's that isn't declared, is named**
  (`aliases_in`, `aliases_in_args`), as one in a fn pointer or a generic
  argument is: `() => PriorityLevel`, `(priorityLevel: PriorityLevel, ..)`.
  A private alias is read as it's written where it has no type parameters
  of its own; one that has is expanded, as before, since its body is
  written of them.

Rejected: **naming a cast by what it's of**, the binding method a
`js::method` makes of, `unstable_schedule_callback(&self, priority_level,
callback)`: a cast says the value is another type, so what it's made of
needn't be that function, and here takes one argument fewer.

Held by `test/declarations-function-types.test.ts`'s "a static's function
type is named as the functions its value may be" (its program
`test/fixtures/value_function_names.rs`, SchedulerNative's form: a host
binding's `js::cast` or the crate's function, a private alias of a `dyn
Fn` of another module's `PriorityLevel`, a `dyn Fn() -> PriorityLevel`, a
plain static's fn pointer), `tsc` checking a caller and its run giving
what the crate's functions give, and its first test's `given`. Mutations
in `lower/declarations/aliases.ts` and `lower/declarations/functions.ts`.
