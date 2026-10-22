# Use a JavaScript library from Rust

You'll install an npm package, bind what your app uses of it, and call it
from Rust. The package is [Sonner](https://sonner.emilkowal.ski), a React
component and functions that show toasts. A binding is a declaration: it
writes no JS of its own, and its call is the call you'd write in JS.

Start from the app [Getting started](vite.md) made. Every example here was
compiled by rust-js as it is today; the JS shown is what it writes, without
its header comment and source-map line.

## Install the package

Install it as you install any npm package:

```bash
bun add sonner         # or npm install sonner
```

Nothing changes in `Cargo.toml`: rust-js doesn't need to know about the
package. Your bindings name it, and Vite resolves the import as it would
from hand-written JS.

## Read its types

Open the package's `index.d.ts`. These are the parts this app uses:

```ts
type Position = 'top-left' | 'top-right' | 'bottom-left' | 'bottom-right' | 'top-center' | 'bottom-center';

interface ToasterProps {
    position?: Position;
    richColors?: boolean;
    // ...
}

type ExternalToast = {
    description?: (() => React.ReactNode) | React.ReactNode;
    duration?: number;
    // ...
};

declare const toast: ((message: titleT, data?: ExternalToast) => string | number) & {
    success: (message: titleT | React.ReactNode, data?: ExternalToast) => string | number;
    dismiss: (id?: number | string) => string | number;
    // ...
};

declare const Toaster: React.ForwardRefExoticComponent<ToasterProps & React.RefAttributes<HTMLElement>>;

export { Toaster, toast };
```

Each binding says, in Rust, what one of these is.

## Write the bindings

Put them in a module of their own, named for the package:

```rust
// src/sonner.rs
//! [Sonner](https://sonner.emilkowal.ski)'s toasts: what this app uses of
//! its `index.d.ts`.

use react::JSX;

/// Where the toasts show: TypeScript's `Position`.
pub enum Position {
    #[cfg_attr(rust_js, rust_js::name = "top-right")]
    TopRight,
    #[cfg_attr(rust_js, rust_js::name = "bottom-right")]
    BottomRight,
}

pub struct ToasterProps {
    pub position: Option<Position>,
    pub rich_colors: Option<bool>,
}

/// Where the toasts show; one per app.
#[cfg_attr(rust_js, rust_js::link_name = "sonner#Toaster")]
pub fn Toaster(props: ToasterProps) -> JSX::Element {
    unreachable!()
}

/// A toast's id: TypeScript's `string | number`.
#[cfg_attr(rust_js, rust_js::untagged)]
#[derive(Clone)]
pub enum ToastId {
    Text(String),
    Number(f64),
}

/// What a toast shows besides its message: TypeScript's `ExternalToast`.
#[derive(Default)]
pub struct ExternalToast<'a> {
    pub description: Option<&'a str>,
    pub duration: Option<f64>,
}

/// Shows a toast with `message`.
#[cfg_attr(rust_js, rust_js::link_name = "sonner#toast")]
pub fn toast(message: &str, data: Option<&ExternalToast>) -> ToastId {
    unreachable!()
}

/// Shows a toast of something that went well.
#[cfg_attr(rust_js, rust_js::link_name = "sonner#toast.success")]
pub fn success(message: &str, data: Option<&ExternalToast>) -> ToastId {
    unreachable!()
}

/// Closes the toast `id`, or every toast.
#[cfg_attr(rust_js, rust_js::link_name = "sonner#toast.dismiss")]
pub fn dismiss(id: Option<ToastId>) -> ToastId {
    unreachable!()
}
```

How a binding is written:

- **A JS function is an ordinary Rust function** with
  `#[cfg_attr(rust_js, rust_js::link_name = "...")]` and the body
  `unreachable!()`. rust-js writes a call to the JS function where you call
  it; the body never runs ([ADR 0373](../../decisions/0373-one-binding-form.md)).
- **`link_name` says where the function is**: `"sonner#toast"` is the
  export `toast` of the module `sonner`, `import { toast } from "sonner"`.
  `"sonner#toast.success"` is a path inside an export, `toast.success`.
- **`cfg_attr(rust_js, ..)` is how rust-js's attributes are written.** rust-js
  compiles with `--cfg rust_js`. A plain `cargo check`, your editor's,
  leaves the attribute out, and still type-checks the function ([ADR 0113](../../decisions/0113-plain-rustc.md)).
  The template's `Cargo.toml` lists `cfg(rust_js)` so Cargo knows it.
- **A component is a binding that returns `JSX::Element`**, with a
  capitalized name, and its props are a struct, as your own components'
  are. `jsx!` checks its props as it checks yours.
- **One Rust function for each JS function**, named as the JS one is, with
  TypeScript's parameters in TypeScript's order ([ADR 0443](../../decisions/0443-a-value-a-caller-may-leave-out-is-an-option.md)).
- **Bind what you use.** A binding is a claim about another project's API,
  and rustc can't check it against the library: read the `.d.ts`, and its
  documentation where the types don't say. Here `message` is bound as the
  `&str` the app gives, of TypeScript's `React.ReactNode`.

## Type its values

Each TypeScript type is the Rust type that says the same thing:

| TypeScript | Rust |
| --- | --- |
| `string`, given | `&str` |
| `string`, returned | `String` |
| `number` | `f64`, or an integer type where it's one |
| `boolean` | `bool` |
| `x?: T`, an optional parameter or field | `Option<T>`: `None` is left out |
| `{ description?: string; .. }` | a struct, its fields `Option`s |
| `'top-right' \| 'bottom-right'` | an enum of variants without fields, each `rust_js::name`d |
| `string \| number` | an enum of one-value variants, `#[cfg_attr(rust_js, rust_js::untagged)]` |
| a component, `React.FC<Props>` | a capitalized function that returns `JSX::Element` |

- **A variant without fields is a string**, its name, or the
  `rust_js::name` you give it: `Position::BottomRight` is `"bottom-right"`.
- **An untagged enum is its payload** ([ADR 0214](../../decisions/0214-untagged-enums.md)): a `ToastId` is the
  string or the number, as JS has it. A `match` on one tests
  `typeof id === "string"`.
- **A field's JS name is its camelCase**: `rich_colors` is `richColors`
  ([ADR 0573](../../decisions/0573-every-rust-name-is-camel-case-in-js.md)).

[JavaScript types](../interop/types.md) has every Rust type's JS.

## Call it

Use the bindings as you use any Rust function:

```rust
// src/App.rs
mod sonner;

use react::event::MouseEvent;
use react::{JSX, jsx, use_state};
use sonner::{ExternalToast, Position, Toaster, dismiss, success, toast};

pub fn App() -> JSX::Element {
    let (last, set_last) = use_state(None);

    let save = move |_: &MouseEvent<_>| {
        let details = ExternalToast {
            description: Some("Your changes are on the server."),
            ..Default::default()
        };
        set_last.set(Some(success("Saved", Some(&details))));
    };

    jsx! {
        <main>
            <Toaster position={Some(Position::BottomRight)} richColors={Some(true)} />
            <button type="button" onClick={save}>{"Save"}</button>
            <button type="button" onClick={move |_| set_last.set(Some(toast("Hello from Rust", None)))}>
                {"Say hello"}
            </button>
            <button type="button" onClick={move |_| {
                dismiss(last.clone());
            }}>
                {"Dismiss"}
            </button>
        </main>
    }
}
```

`bunx rust-js build` writes `src/App.jsx`:

```jsx
import { useState } from "react";
import { Toaster, toast } from "sonner";

export function App() {
  const [last, setLast] = useState(undefined);

  const save = () => {
    const details = { description: "Your changes are on the server." };
    setLast(toast.success("Saved", details));
  };
  return (
    <main>
      <Toaster position="bottom-right" richColors />
      <button type="button" onClick={save}>
        Save
      </button>
      <button type="button" onClick={() => setLast(toast("Hello from Rust"))}>
        Say hello
      </button>
      <button type="button" onClick={() => toast.dismiss(last)}>
        Dismiss
      </button>
    </main>
  );
}
```

- **Each binding is its import.** `toast` and `toast.success` are one
  import, `toast`, and `Toaster` another.
- **A module of bindings alone writes no file.** There's no `sonner.js`:
  `src/sonner.rs` declares, and `App.jsx` imports from `sonner` itself.
- **`None` is left out.** A trailing `None` argument is no argument,
  `toast("Hello from Rust")`, and a `None` field is no field,
  `{ description: ".." }`. A `Some(true)` prop is `richColors`.
- **A struct is an object**, written where it's made. `..Default::default()`
  fills in the fields you leave out, as Rust does; they're `None`, so
  they're not written.

## Other forms

The same attribute binds the rest of what a JS module has. Each of these
compiles, against a module `./timer.js` of your own:

| JS | Binding | Call |
| --- | --- | --- |
| a default export | `link_name = "./timer.js#default"` on `pub fn shout(text: &str) -> String` | `shout(..)`, `import shout from "./timer.js"` |
| a class | `pub struct Timer(PhantomData<js::JsObject>);` | |
| its constructor | `link_name = "new ./timer.js#Timer"` on `pub fn new(label: &str) -> &'static Timer`, in `pub mod timer` | `timer::new("tea")` is `new Timer("tea")` |
| a method | `link_name = "tick"` on `pub fn tick(&self, by: Option<u32>) -> u32`, in `impl Timer` | `timer.tick(None)` is `timer.tick()` |
| a getter | `link_name = "get name"` on `pub fn name(&self) -> String` | `timer.name()` is `timer.name` |
| a global | `link_name = "console.log"` on `pub fn log(text: &str)` | `console.log(..)` |
| an exported value | `#[link_name = "./timer.js#VERSION"] pub safe static VERSION: &'static str;` in `unsafe extern "Rust" { }` | `VERSION` |

- **A JS class is a struct of a `JsObject`**, and you hold it by
  reference, `&'static Timer`.
- **A constructor is `new`**, in a module named for the class, as the
  crates' are: `webapi::event::new("ping", None)` is `new Event("ping")`.
- **An `extern` block holds `static`s alone**: an exported value, or a
  file a bundler imports, `#[link_name = "./assets/logo.svg#default"]`.
  A function in an `extern` block is a compile error that says what to
  write:

```text
error: rust-js: a JS function is bound as an ordinary function, `#[cfg_attr(rust_js, rust_js::link_name = "console.log")] fn log(..) { unreachable!() }`, not in an `extern` block (ADR 0373)
```

Before you bind something, look in the crates rust-js ships: `js` has
JavaScript's own API, `webapi` the browser's, `react` React's
([The crates rust-js ships](../libraries/README.md)). Use theirs; don't
bind it again.

## Next

- [Write bindings for JavaScript APIs](../interop/bindings.md): every form
  `link_name` takes, `null`, callbacks, variadic functions, and more.
- [Access to the JavaScript global scope](../interop/global-scope.md).
- [Dependencies: crates and npm packages](../project/dependencies.md):
  bindings someone else keeps, as a package.
