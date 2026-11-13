// A component that reads a ref as it renders, as a person would write
// it: React Compiler refuses it, "Cannot access refs during render", as
// it does rust-js's of the same Rust.
import { createRef, useRef } from "react";

export function List({ items }) {
  const refs = useRef(items.map(() => createRef()));
  return items.map((item, i) => <button key={i} ref={refs.current[i]}>{item}</button>);
}

export function Normal(p) {
  return <input ref={p.ref} tabIndex={p.title} />;
}
