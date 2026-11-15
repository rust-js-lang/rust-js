//! Headless components and hooks, as Base UI and Radix give them: a
//! popover placed beside its trigger by `geometry`, and a menu of a
//! collection's items, unstyled, for a kit to style.

use std::fmt::Display;

pub use geometry::{Placement, Point, Rect};

use geometry::{Flip, Middleware, Offset, Shift, compute_position};
use react::attributes::HTMLAttributes;
use react::event::MouseEvent;
use react::webapi::window_global::{add_event_listener, remove_event_listener};
use react::webapi::{Event, HTMLButtonElement, HTMLElement, HTMLLIElement, events};
use react::{Context, JSX, ReactNode, RefObject, create_context, jsx, use_context, use_layout_effect, use_ref, use_state};

/// What a menu lists, each item shown by its `Display`.
pub trait Collection {
    type Item: Display;

    fn items(&self) -> Vec<Self::Item>;

    fn len(&self) -> usize {
        self.items().len()
    }

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn labels(&self) -> Vec<String> {
        self.items().iter().map(ToString::to_string).collect()
    }
}

thread_local! {
    /// Whether the nearest popover is open.
    pub static OPEN: Context<bool> = create_context(false);
}

/// Where a floating element goes and on which side, once it's measured.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Floating {
    pub placement: Placement,
    pub point: Point,
}

/// The middleware a popover's panel is moved by: away from its trigger,
/// flipped where it doesn't fit, and kept inside `boundary`.
pub fn middleware(gap: f64) -> Vec<Box<dyn Middleware>> {
    vec![Box::new(Offset(gap)), Box::new(Flip), Box::new(Shift { padding: 8.0 })]
}

fn rect_of(element: &HTMLElement) -> Rect {
    let rect = element.get_bounding_client_rect();
    Rect::new(rect.x(), rect.y(), rect.width(), rect.height())
}

/// Where `floating` goes beside `reference`, measured after each render
/// that's laid out, and again as the window resizes, as Floating UI's
/// `autoUpdate` does; `placement` and the origin before.
pub fn use_floating(
    reference: RefObject<Option<&'static HTMLElement>>,
    floating: RefObject<Option<&'static HTMLElement>>,
    placement: Placement,
    boundary: Rect,
) -> Floating {
    let (position, set_position) = use_state(|| Floating {
        placement,
        point: Point::default(),
    });
    let current = *position;
    use_layout_effect(
        move || {
            let measure = move || {
                if let (Some(reference), Some(floating)) = (reference.current(), floating.current()) {
                    let state = compute_position(rect_of(reference), rect_of(floating), boundary, placement, &middleware(4.0));
                    let measured = Floating {
                        placement: state.placement,
                        point: state.point,
                    };
                    if measured != current {
                        set_position.set(measured);
                    }
                }
            };
            measure();
            let on_resize = move |_: &'static Event| measure();
            add_event_listener(events::Resize, on_resize, None);
            // Stops listening, as `autoUpdate`'s cleanup does.
            move || remove_event_listener(events::Resize, on_resize, None)
        },
        None,
    );
    current
}

pub struct PopoverProps<'a> {
    pub open: bool,
    pub trigger: &'a str,
    pub placement: Option<Placement>,
    pub on_open_change: Option<Box<dyn Fn(bool)>>,
    pub children: Option<&'a dyn ReactNode>,
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub html: HTMLAttributes<'a>,
}

/// A trigger, and the panel it opens beside it.
pub fn Popover(props: PopoverProps) -> JSX::Element {
    let PopoverProps {
        open,
        trigger,
        placement,
        on_open_change,
        children,
        html,
    } = props;
    let placement = placement.unwrap_or_default();
    let reference = use_ref::<Option<&'static HTMLElement>>(None);
    let floating = use_ref::<Option<&'static HTMLElement>>(None);
    let Floating { placement: side, point } = use_floating(reference, floating, placement, Rect::new(0.0, 0.0, 1024.0, 768.0));
    let toggle = move |_: &MouseEvent<HTMLButtonElement>| {
        if let Some(change) = &on_open_change {
            change(!open);
        }
    };
    jsx! {
        <OPEN value={open}>
            <span {...html} data-placement={side.to_string()}>
                <button ref={reference} aria-expanded={open} onClick={toggle}>{trigger}</button>
                {open.then(|| jsx! {
                    <div ref={floating} role="dialog" data-x={point.x} data-y={point.y}>{children}</div>
                })}
            </span>
        </OPEN>
    }
}

pub struct MenuProps<'a> {
    pub label: &'a str,
    pub items: Vec<String>,
    pub on_select: Option<Box<dyn Fn(usize)>>,
}

/// A collection's items, each one chosen by a click, inside a popover.
pub fn Menu(MenuProps { label, items, on_select }: MenuProps) -> JSX::Element {
    let open = use_context(&OPEN);
    let on_select = std::rc::Rc::new(on_select);
    let rows: Vec<JSX::Element> = items
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let on_select = on_select.clone();
            let choose = move |_: &MouseEvent<HTMLLIElement>| {
                if let Some(select) = &*on_select {
                    select(index);
                }
            };
            jsx! { <li key={index} role="menuitem" onClick={choose}>{item.as_str()}</li> }
        })
        .collect();
    jsx! { <ul role="menu" aria-label={label} data-open={*open}>{rows}</ul> }
}
