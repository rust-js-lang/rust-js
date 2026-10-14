// Which event each React handler gets, which react/generate.ts and
// react/attributes.ts both name. A JS name as a Rust one is
// scripts/rust-names.ts's.

// Which React event each handler gets, as react.dev's common components
// page groups them, by @types/react's names. Any other is `SyntheticEvent`,
// React's base event.
export const EVENT_TYPES: Record<string, string> = {};
const groups: Record<string, string> = {
  AnimationEvent: "AnimationEnd AnimationIteration AnimationStart",
  MouseEvent: "AuxClick Click ContextMenu DoubleClick MouseDown MouseEnter MouseLeave MouseMove MouseOut MouseOver MouseUp",
  InputEvent: "BeforeInput Input",
  FocusEvent: "Blur Focus",
  CompositionEvent: "CompositionEnd CompositionStart CompositionUpdate",
  ClipboardEvent: "Copy Cut Paste",
  DragEvent: "Drag DragEnd DragEnter DragExit DragLeave DragOver DragStart Drop",
  PointerEvent:
    "GotPointerCapture LostPointerCapture PointerCancel PointerDown PointerEnter PointerLeave PointerMove PointerOut PointerOver PointerUp",
  KeyboardEvent: "KeyDown KeyPress KeyUp",
  TouchEvent: "TouchCancel TouchEnd TouchMove TouchStart",
  TransitionEvent: "TransitionCancel TransitionEnd TransitionRun TransitionStart",
  WheelEvent: "Wheel",
  ToggleEvent: "BeforeToggle Toggle",
  SubmitEvent: "Submit",
  ChangeEvent: "Change",
  UIEvent: "Scroll ScrollEnd",
};
for (const [type, names] of Object.entries(groups)) for (const name of names.split(" ")) EVENT_TYPES[`on${name}`] = type;
