# Components and props

A React component in rust-js is a Rust function. Its props are a struct,
and its markup is [`jsx!`](jsx.md). rust-js compiles each component to the
function you'd write in a `.jsx` file.

## A component is a function

```rust
use react::{JSX, jsx};

pub fn Greeting() -> JSX::Element {
    jsx! { <h1>{"Hello, world"}</h1> }
}
```

```jsx
export function Greeting() {
  return <h1>Hello, world</h1>;
}
```

A function is a component when:

- its name starts with an uppercase letter, `Greeting`, as JSX and Fast
  Refresh need;
- it returns `JSX::Element`, or `Option<JSX::Element>` when it may render
  nothing;
- it takes no parameter, or one: its props struct.

Write the name as JSX does. rustc's `non_snake_case` lint doesn't fire for a
component, so you need no `#[allow]` ([ADR 0439](../../decisions/0439-a-component-is-named-as-jsx-names-it.md)). Any other function keeps
its lint and is camelCase in JS: `fn load_notes` is `loadNotes`
([ADR 0573](../../decisions/0573-every-rust-name-is-camel-case-in-js.md)).

A `pub` component is an `export`. Use it from another module by its Rust
path, `<ui::Card />`, or import it, `use ui::Card;` and `<Card />`.

## Props are a struct

Each field is a prop. A field's name is snake_case in Rust and camelCase in
JS, `image_url` as `imageUrl`, and you write it in `jsx!` as JS does. Take
the props apart in the parameter, as React does:

```rust
use react::{JSX, jsx};

pub struct AvatarProps<'a> {
    pub name: &'a str,
    pub image_url: &'a str,
    #[cfg_attr(rust_js, rust_js::default = 48)]
    pub size: u32,
    pub caption: Option<&'a str>,
}

pub fn Avatar(
    AvatarProps {
        name,
        image_url,
        size,
        caption,
    }: AvatarProps,
) -> JSX::Element {
    jsx! {
        <figure>
            <img src={image_url} alt={name} width={size} height={size} />
            {caption.map(|text| jsx! { <figcaption>{text}</figcaption> })}
        </figure>
    }
}

pub fn Team() -> JSX::Element {
    jsx! {
        <>
            <Avatar name="Ada" imageUrl="/ada.jpg" />
            <Avatar name="Alan" imageUrl="/alan.jpg" size={96} caption={Some("Alan Turing")} />
        </>
    }
}
```

```jsx
export function Avatar({ name, imageUrl, size = 48, caption }) {
  return (
    <figure>
      <img src={imageUrl} alt={name} width={size} height={size} />
      {caption != null && <figcaption>{caption}</figcaption>}
    </figure>
  );
}

export function Team() {
  return (
    <>
      <Avatar name="Ada" imageUrl="/ada.jpg" />
      <Avatar name="Alan" imageUrl="/alan.jpg" size={96} caption="Alan Turing" />
    </>
  );
}
```

The Rust spelling of a prop is an error that names the JS one:

```text
error: rust-js: `image_url` is `imageUrl` in JSX, as React names it
  |
  |             <Avatar name="Ada Lovelace" image_url="/ada.jpg" />
  |                                         ^^^^^^^^^
```

A missing prop, a prop the struct doesn't have, and a value of the wrong
type are compile errors too: ``missing prop `subtitle` of `CardProps` ``.

### Optional props

A prop JS callers may leave out is an `Option`. Leave it out in `jsx!`, or
give it as `Some(..)`. JS gets the value itself, and nothing where it's
`None`, as `caption` above.

### Defaults

A prop with a default is marked `#[cfg_attr(rust_js, rust_js::default)]`,
its type's `Default`, or `rust_js::default = <literal>`, that literal of
the field's type: `= 48`, `= true`, `= "_self"`. The field keeps its own
type, `u32`, not an `Option`, and the default goes where JS writes it, in
the destructuring: `{ size = 48 }` ([ADR 0212](../../decisions/0212-props-defaults.md)).

- Callers may leave the prop out, Rust's and JS's alike.
- The default is a literal. One that has to be made, a `HashMap`'s, is an
  error that says to write it.
- Props with a default are taken apart in the parameter, as above. Taken
  whole, `props: AvatarProps`, a JS caller's missing field would have no
  default, so that's an error.

### Props taken whole

A component may take its props whole, and pass them on whole with
`{...props}`:

```rust
use react::{JSX, jsx};

pub struct TitleProps<'a> {
    pub text: &'a str,
    pub level: u32,
}

pub fn Title(props: TitleProps) -> JSX::Element {
    jsx! { <h1 data-level={props.level}>{props.text}</h1> }
}

pub fn Page(props: TitleProps) -> JSX::Element {
    jsx! { <main><Title {...props} /></main> }
}
```

```jsx
export function Title(props) {
  return <h1 data-level={props.level}>{props.text}</h1>;
}

export function Page(props) {
  return (
    <main>
      <Title {...props} />
    </main>
  );
}
```

To give some props and take the rest from another struct, use Rust's
struct update, `{..base}`, last: `<Title text="New" {..props} />`. See
[spreads](jsx.md#spreads).

## Children

A `children` field holds what's written between the tags. Type it as
TypeScript's `children: ReactNode`, `&'a dyn ReactNode`, which takes any
node: an element, text, a number, several of them, a list, or `None`:

```rust
use react::{JSX, ReactNode, jsx};

pub struct CardProps<'a> {
    pub title: &'a str,
    pub children: &'a dyn ReactNode,
}

pub fn Card(CardProps { title, children }: CardProps) -> JSX::Element {
    jsx! {
        <section className="card">
            <h2>{title}</h2>
            {children}
        </section>
    }
}

pub fn Profile() -> JSX::Element {
    jsx! {
        <Card title="Profile">
            <img src="/ada.jpg" alt="Ada Lovelace" />
            {"A pioneer"}
        </Card>
    }
}
```

```jsx
export function Card({ title, children }) {
  return (
    <section className="card">
      <h2>{title}</h2>
      {children}
    </section>
  );
}

export function Profile() {
  return (
    <Card title="Profile">
      <img src="/ada.jpg" alt="Ada Lovelace" />A pioneer
    </Card>
  );
}
```

- `children: Option<&'a dyn ReactNode>` is TypeScript's `children?:
  ReactNode`: callers may give none.
- `children: JSX::Element` takes exactly one element.
- Children written between the tags and a `children={..}` prop together
  are an error, `children were provided twice`.

To look inside children, count them or clone one, use React's `Children`
and `cloneElement`, as [the rest of React's API](api.md#children) says.

## Passing DOM attributes through

### `rest`: what the struct doesn't name

A field of type `react::Rest` is JS's `...rest`: the props a JS caller gave
that the struct doesn't name. Spread it onto an element:

```rust
use react::{JSX, Rest, jsx};

pub struct ButtonProps<'a> {
    pub label: &'a str,
    pub rest: Rest,
}

pub fn Button(ButtonProps { label, rest }: ButtonProps) -> JSX::Element {
    jsx! { <button className="button" {...rest}>{label}</button> }
}
```

```jsx
export function Button({ label, ...rest }) {
  return (
    <button className="button" {...rest}>
      {label}
    </button>
  );
}
```

A `Rest` is taken apart in the parameter or a `let`, as JS's destructuring
is; reading `props.rest` is an error that says so ([ADR 0195](../../decisions/0195-rest-props.md)). Its default
is empty, so Rust callers leave it out.

### `flatten`: an element's attributes as props

To let Rust callers give a component an element's attributes, flatten that
element's attributes struct into the props, `react::attributes::*`, one per
element as @types/react types them:

```rust
use react::attributes::ButtonHTMLAttributes;
use react::{JSX, jsx};

pub struct ButtonProps<'a> {
    pub label: &'a str,
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub button: ButtonHTMLAttributes<'a>,
}

pub fn Button(ButtonProps { label, button }: ButtonProps) -> JSX::Element {
    jsx! { <button className="button" {...button}>{label}</button> }
}

pub fn Toolbar() -> JSX::Element {
    jsx! {
        <div>
            <Button label="Save" type={Some("submit")} />
            <Button label="Delete" disabled={Some(true)} onClick={Some(Box::new(|_| ()))} />
        </div>
    }
}
```

```jsx
export function Button({ label, ...button }) {
  return (
    <button className="button" {...button}>
      {label}
    </button>
  );
}

export function Toolbar() {
  return (
    <div>
      <Button label="Save" type="submit" />
      <Button label="Delete" disabled onClick={() => {}} />
    </div>
  );
}
```

Each flattened field is an optional prop of the component, written by its
JSX name. A component has at most one rest: one `Rest` field, or one
flattened struct ([ADR 0204](../../decisions/0204-flattened-props.md)).

## Rendering lists

Map an iterator to elements and collect a `Vec`. Give each item a `key`:

```rust
use react::{JSX, jsx, use_state};

pub struct Person {
    pub id: u32,
    pub name: String,
    pub admin: bool,
}

pub struct PeopleProps<'a> {
    pub people: &'a [Person],
}

pub fn People(PeopleProps { people }: PeopleProps) -> JSX::Element {
    let (selected, set_selected) = use_state(|| None);
    let items: Vec<JSX::Element> = people
        .iter()
        .map(|person| {
            let id = person.id;
            jsx! {
                <li
                    key={id}
                    className={if *selected == Some(id) { "selected" } else { "" }}
                    onClick={move |_| set_selected.set(Some(id))}
                >
                    {&person.name}
                    {person.admin.then(|| jsx! { <b>{" (admin)"}</b> })}
                </li>
            }
        })
        .collect();
    jsx! {
        <>
            <ul>{items}</ul>
            {people.is_empty().then(|| jsx! { <p>{"No one yet"}</p> })}
        </>
    }
}
```

```jsx
import { useState } from "react";

export function People({ people }) {
  const [selected, setSelected] = useState(undefined);
  const items = people.map((person) => {
    const id = person.id;
    return (
      <li key={id} className={selected === id ? "selected" : ""} onClick={() => setSelected(id)}>
        {person.name}
        {person.admin && <b> (admin)</b>}
      </li>
    );
  });
  return (
    <>
      <ul>{items}</ul>
      {people.length === 0 && <p>No one yet</p>}
    </>
  );
}
```

The map can be written in place too:

```rust
# use react::{JSX, jsx};
#
# pub struct Person {
#     pub id: u32,
#     pub name: String,
#     pub admin: bool,
# }
#
# pub struct PeopleProps<'a> {
#     pub people: &'a [Person],
# }
#
pub fn Names(PeopleProps { people }: PeopleProps) -> JSX::Element {
    jsx! {
        <ul>
            {people
                .iter()
                .map(|person| jsx! { <li key={person.id}>{&person.name}</li> })
                .collect::<Vec<_>>()}
        </ul>
    }
}
```

```jsx
export function Names({ people }) {
  return (
    <ul>
      {people.map((person) => (
        <li key={person.id}>{person.name}</li>
      ))}
    </ul>
  );
}
```

A `key` is a number, a string, or an `Option` of one. It's never a prop:
the component doesn't see it. For a list item of several elements, use a
keyed `<Fragment key={..}>` ([JSX](jsx.md#fragments)).

## Conditional rendering

Rendering nothing is `None`. Each of these is a child:

| Rust | JS |
|---|---|
| `{done.then(\|\| jsx! { <b>{"Done"}</b> })}` | `{done && <b>Done</b>}` |
| `{caption.map(\|text\| jsx! { <i>{text}</i> })}` | `{caption != null && <i>{caption}</i>}` |
| `{if online { "Online" } else { "Offline" }}` | `{online ? "Online" : "Offline"}` |
| `{if ok { Some(jsx! { <p /> }) } else { None }}` | `{ok && <p />}` |

A `match` that picks an element is a `switch` where its arms are
constants and a last `_` or name, its `default`, as any `match` of
constants is:

```rust
use react::{JSX, jsx};

pub struct ScoreProps {
    pub points: u32,
}

pub fn Score(ScoreProps { points }: ScoreProps) -> JSX::Element {
    let label = match points {
        0 => jsx! { <i>{"no points"}</i> },
        1 => jsx! { <span>{"1 point"}</span> },
        n => jsx! { <b>{n}{" points"}</b> },
    };
    jsx! { <p>{label}</p> }
}
```

```jsx
export function Score({ points }) {
  let label;
  switch (points) {
    case 0:
      label = <i>no points</i>;
      break;
    case 1:
      label = <span>1 point</span>;
      break;
    default:
      label = <b>{points} points</b>;
  }
  return <p>{label}</p>;
}
```

A component that renders nothing returns `Option<JSX::Element>`, and `?`
works in it:

```rust
use react::{JSX, jsx};

pub struct BannerProps<'a> {
    pub message: Option<&'a str>,
}

pub fn Banner(BannerProps { message }: BannerProps) -> Option<JSX::Element> {
    let message = message?;
    Some(jsx! { <p className="banner">{message}</p> })
}
```

```jsx
export function Banner({ message }) {
  if (message == null) {
    return;
  }
  return <p className="banner">{message}</p>;
}
```

`None` is `undefined`, which React renders as nothing ([ADR 0286](../../decisions/0286-components-that-render-nothing.md)).

## Events

A handler is a closure. Its event is typed by the tag it's on, as
@types/react types it ([ADR 0224](../../decisions/0224-typed-intrinsic-elements.md)): a `<button>`'s `onClick` gets an
`event::MouseEvent<HTMLButtonElement>`. Write `|_|` when you don't read
it, or name its type with `_` for the element, `&ChangeEvent<_>`, which
the tag fills in.

```rust
use react::event::{ChangeEvent, KeyboardEvent, MouseEventHandler};
use react::webapi::HTMLButtonElement;
use react::{JSX, jsx, use_state};

pub struct ButtonProps<'a> {
    pub label: &'a str,
    pub on_click: MouseEventHandler<HTMLButtonElement>,
}

pub fn Button(ButtonProps { label, on_click }: ButtonProps) -> JSX::Element {
    jsx! { <button onClick={on_click}>{label}</button> }
}

pub struct SizePickerProps {
    pub on_pick: Box<dyn Fn(String)>,
}

pub fn SizePicker(SizePickerProps { on_pick }: SizePickerProps) -> JSX::Element {
    jsx! {
        <select onChange={move |e: &ChangeEvent<_>| on_pick(e.value())}>
            <option value="s">{"Small"}</option>
            <option value="l">{"Large"}</option>
        </select>
    }
}

pub fn Panel() -> JSX::Element {
    let (clicks, set_clicks) = use_state(|| 0);
    let (size, set_size) = use_state(|| String::from("s"));
    jsx! {
        <div onKeyDown={move |e: &KeyboardEvent<_>| if e.key() == "Escape" { set_clicks.set(0) }}>
            <Button label="Like" onClick={Box::new(move |_| set_clicks.update(|n| n + 1))} />
            <SizePicker onPick={Box::new(move |picked| set_size.set(picked))} />
            {clicks}{" likes, size "}{size.clone()}
        </div>
    }
}
```

```jsx
import { useState } from "react";

export function Button({ label, onClick }) {
  return <button onClick={onClick}>{label}</button>;
}

export function SizePicker({ onPick }) {
  return (
    <select onChange={(e) => onPick(e.target.value)}>
      <option value="s">Small</option>
      <option value="l">Large</option>
    </select>
  );
}

export function Panel() {
  const [clicks, setClicks] = useState(0);
  const [size, setSize] = useState("s");
  return (
    <div
      onKeyDown={(e) => {
        if (e.key === "Escape") {
          setClicks(0);
        }
      }}
    >
      <Button label="Like" onClick={() => setClicks((n) => (n + 1) | 0)} />
      <SizePicker onPick={(picked) => setSize(picked)} />
      {clicks} likes, size {size}
    </div>
  );
}
```

- **The event types** are `react::event`'s, @types/react's names:
  `MouseEvent`, `KeyboardEvent`, `ChangeEvent`, `SubmitEvent`,
  `FocusEvent`, `PointerEvent`, `DragEvent`, `InputEvent` and the rest.
  Each field is a method in snake_case, `e.client_x()` for `e.clientX`;
  `e.prevent_default()`, `e.stop_propagation()`, `e.current_target()`
  (the tag's element) and `e.native_event()` (the DOM's event) are on
  every one. `ChangeEvent`'s `e.value()` and `e.checked()` read
  `e.target.value` and `e.target.checked`. A form's `onSubmit` gets a
  `SubmitEvent`.
- **A handler prop** is a boxed closure, `Box<dyn Fn(String)>`, or for an
  element's event, @types/react's alias, `MouseEventHandler<HTMLButtonElement>`.
  The caller writes `Box::new(..)`; JS gets the function itself.
- **An optional handler** is an `Option`, `Option<MouseEventHandler>`, passed
  on as it is.
- **An async handler** is `event::MouseEvent::spawn(async move |e| ..)`, the
  `async` function itself in JS.
- **A handler of any element's event**, `Box<dyn Fn(&event::MouseEvent)>`,
  goes on a tag as `event::MouseEvent::widen(handler)`; an element's event
  is any element's by `e.upcast()`. Each is the value itself in JS.

State setters are `Copy`, so every handler can `move` one in. See
[hooks](hooks.md).
