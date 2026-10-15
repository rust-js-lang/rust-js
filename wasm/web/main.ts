// The playground is written in Rust (ADR 0032), with React (ADR 0044), as a
// Vite app (ADR 0045): rust/lib.rs and its components/, which compile-rust.ts
// compiles to JSX with rust-js itself. This file only starts it.

import "./styles.css";
import { start } from "./rust/lib.jsx";

start();
