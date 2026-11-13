// A component that writes to what it's given as it renders, as a
// person would write it: React Compiler refuses it, "This value cannot
// be modified", as it does rust-js's of the same Rust.
import { useState } from "react";

export function Edited(e) {
  const n = e.n;
  e.n = 5;
  return <p>{n}</p>;
}
