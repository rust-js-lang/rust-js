//! Styled components, as shadcn/ui gives them: each one a primitive's or
//! an element's, its classes joined by `cn`.

use primitives::Popover;
use react::attributes::ButtonHTMLAttributes;
use react::{JSX, ReactNode, jsx};

pub use primitives::{Menu, MenuProps, Placement};
/// A dropdown is a popover, as shadcn's `DropdownMenu` is Radix's.
pub use primitives::PopoverProps as DropdownProps;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Variant {
    #[default]
    Default,
    Destructive,
    Outline,
    Ghost,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Size {
    Small,
    #[default]
    Medium,
    Large,
}

/// The classes given, joined, the empty and missing left out, as shadcn's
/// `cn` joins them.
pub fn cn(parts: &[Option<&str>]) -> String {
    parts
        .iter()
        .flatten()
        .filter(|part| !part.is_empty())
        .copied()
        .collect::<Vec<_>>()
        .join(" ")
}

/// A button's classes, of its variant and size, and `extra`'s.
pub fn button_classes(variant: Variant, size: Size, extra: Option<&str>) -> String {
    let variant = match variant {
        Variant::Default => "bg-primary text-primary-foreground",
        Variant::Destructive => "bg-destructive text-white",
        Variant::Outline => "border bg-background",
        Variant::Ghost => "hover:bg-accent",
    };
    let size = match size {
        Size::Small => "h-8 px-3",
        Size::Medium => "h-9 px-4",
        Size::Large => "h-10 px-6",
    };
    cn(&[Some("inline-flex items-center rounded-md"), Some(variant), Some(size), extra])
}

pub struct ButtonProps<'a> {
    pub variant: Option<Variant>,
    pub size: Option<Size>,
    pub children: Option<&'a dyn ReactNode>,
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub button: ButtonHTMLAttributes<'a>,
}

pub fn Button(props: ButtonProps) -> JSX::Element {
    let ButtonProps {
        variant,
        size,
        children,
        mut button,
    } = props;
    let classes = button_classes(variant.unwrap_or_default(), size.unwrap_or_default(), button.html.class_name);
    button.html.class_name = Some(&classes);
    jsx! { <button {...button}>{children}</button> }
}

/// A menu of actions behind a trigger, Radix's popover with shadcn's look.
pub fn DropdownMenu(props: DropdownProps) -> JSX::Element {
    jsx! { <Popover {...props} /> }
}
