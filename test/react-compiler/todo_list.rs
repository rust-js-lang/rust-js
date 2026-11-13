//! A component as the guide teaches one: props, state, an effect, a memo,
//! a callback, a list and a condition, each of which React Compiler
//! memoizes as it would hand-written React.
#![allow(non_snake_case)]
use react::{JSX, jsx, use_callback, use_effect, use_memo, use_state};

#[derive(Clone, PartialEq)]
pub struct Todo {
    pub id: u32,
    pub text: String,
    pub done: bool,
}

pub struct TodoListProps {
    pub title: &'static str,
    pub todos: Vec<Todo>,
}

pub fn TodoList(props: TodoListProps) -> JSX::Element {
    let (filter, set_filter) = use_state(|| false);
    let label = use_memo(move || if *filter { "Show all" } else { "Hide done" }, [filter]);
    let toggle = use_callback(move || set_filter.set(!filter), [filter]);
    use_effect(
        move || {
            web_title(props.title);
        },
        Some(&[props.title]),
    );
    let items: Vec<JSX::Element> = (props.todos.iter())
        .filter(|todo| !filter || !todo.done)
        .map(|todo| jsx! { <li key={todo.id}>{todo.text.clone()}</li> })
        .collect();
    jsx! {
        <section>
            <h2>{props.title}</h2>
            <button onClick={move |_| toggle()}>{*label}</button>
            {if items.is_empty() { None } else { Some(jsx! { <ul>{items}</ul> }) }}
        </section>
    }
}

fn web_title(title: &str) {
    println!("{title}");
}
