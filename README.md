<p align="center">
  <img src="docs/logo/rust-js.png" alt="Rust.js logo" width="235" height="170">
</p>
<h1 align="center">Rust.js</h1>
<p align="center"><b>Rust in. <i>Readable</i> JavaScript out.</b></p>

Rust.js compiles idiomatic Rust to readable JavaScript, ES modules, and JSX.
Built on rustc, it keeps Rust's type system, traits, ownership, borrow
checking, and diagnostics.

[Guide](docs/guide/README.md) · [Getting started](docs/guide/tutorial/vite.md) ·
[Examples](examples/) · [Roadmap](ROADMAP.md)

## Goals and delivery promise

1. **Idiomatic Rust in. Readable, correct JavaScript out.** Preserve Rust's
   behavior and enable full-stack Rust apps with business logic and data
   models shared between native servers and JavaScript frontends.
2. **Express anything JavaScript can express.** Build any JavaScript library,
   framework, or application in Rust, preserving its JavaScript APIs and
   runtime behavior. The compiler adapts to the software developers need
   to write.

**Proving goal 1:** Our [test harness](TESTS.md) compares generated JavaScript
with native Rust using regression programs, rustc's tests, real crates, and
generated edge cases. Mutations test the tests; snapshots and readability
checks guard the output; integration tests verify shared logic and JSON models.

**Proving goal 2:** We will port all major JavaScript libraries and frameworks,
keep their existing tests passing, and continuously compile and test each port
with the latest Rust.js to catch downstream regressions. Compiler gaps are
fixed before a port continues.

## Current status

**Experimental; production readiness has not yet been established.**
A substantial subset of Rust works today, but language, library, and
feature-combination gaps remain.

- Structs, enums, pattern matching, traits, generics, closures, and iterators.
- Common standard-library types and operations, including collections,
  `Option`, `Result`, strings, formatting, and `async` / `await`.
- JavaScript imports and exports, browser and Node.js bindings, and
  generated TypeScript declarations.
- React and JSX, Vite and Next.js integration, Fast Refresh, and source maps.
- Shared Cargo crates, supported crates.io dependencies, and Serde JSON.

rustc checks every program before JavaScript is emitted. Unsupported
constructs are refused with diagnostics. Documented differences and remaining
gaps are in the [semantics](docs/guide/semantics.md) and
[limitations](docs/guide/limitations.md).

## Rust → JavaScript

A React counter:

```rust
pub fn App() -> JSX::Element {
    let (count, set_count) = use_state(|| 0);
    jsx! {
        <button onClick={move |_| set_count.update(|count| count + 1)}>
            {"Count is "}{count}
        </button>
    }
}
```

becomes:

```jsx
export function App() {
  const [count, setCount] = useState(0);
  return (
    <button onClick={() => setCount((count) => (count + 1) | 0)}>
      Count is {count}
    </button>
  );
}
```

Imports omitted; see the [React guide](react/README.md) for a complete example.
The `| 0` preserves signed 32-bit wrapping arithmetic.

## Get started

Install [Bun](https://bun.sh/) and [rustup](https://rustup.rs/), then create
a React + Vite app:

```bash
bun create @rust-js@latest my-app
cd my-app
# Run the rustup toolchain install command printed by the generator.
bun install
bun run dev
```

After editing Rust, run `bunx rust-js build` in another terminal in the app's
directory. It writes JavaScript beside the Rust; the dev server picks it up.
There is no watch mode. See the [setup tutorial](docs/guide/tutorial/vite.md)
for host and toolchain requirements, or use `--template next` to create a
Next.js app.

## Contribute

See [Contributing](CONTRIBUTING.md) for the correctness and review standards,
[Development](DEVELOPMENT.md) for setup and checks, and
[Design decisions](docs/README.md) for the reasoning behind the compiler.
