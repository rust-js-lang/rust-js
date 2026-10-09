# 0295. A module's imports are one block, packages first

Status: Accepted. Orders what ADRs 0028, 0103 and 0110 import.

Case: A ([0262](0262-when-rust-and-js-disagree.md)), except for imports for
an effect, whose order a program can observe and which keep it.

## Context

A module's imports came in three runs: its bindings' packages by path,
`./Console` before `react` since `.` sorts first, then the runtime's
helpers, then a blank line and the crate's own modules. react.dev's
modules import packages first and their own relative modules after, in one
block, and a CSS file after what it styles:

```ts
import {useRef, useState, useEffect, useMemo, useId} from 'react';
import {useSandpack, SandpackStack} from '@codesandbox/sandpack-react/unstyled';
import cn from 'classnames';
import {ErrorMessage} from './ErrorMessage';
```

The original's own order, by hand, can't come from the Rust: rustfmt sorts
its `use`s.

## Decision

**A module's imports are one block, ordered as eslint's `import/order`
orders them:**

1. packages, by path;
2. its relative modules, a binding's and the crate's alike, by path, a
   parent's before a sibling's;
3. what it imports for its effect, `js::import!("./app.css")`, in the order
   the Rust writes them, since that's the order they run in;
4. rust-js's runtime helpers.

## Why

- **It's the JavaScript a person writes**: packages, then the project's own
  modules, then styles, the layout linters keep.
- **It's the same program**: Rust says nothing of the order modules load
  in, which was rust-js's choice already, by path; an effect's import, whose
  order the Rust does say, keeps it among its kind.

## Consequences

- A CSS import now comes after the crate's own modules, where it came
  before them: a module's own styles after those of what it renders, as a
  stylesheet overrides what it follows.
