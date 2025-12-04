use chrono::{Local, NaiveDate};
use leptos::{logging::warn, prelude::*};
use reactive_stores::{Field, Patch, Store};
use serde::{Deserialize, Serialize};
use slotmap::{new_key_type, SlotMap};

new_key_type! {
    pub struct TodoKey;
}

#[derive(Debug, Store)] // , Serialize, Deserialize not implemented for Store<Todo>.
struct Todos {
    user: User,
    // Note the difference! We have a SlotMap now, containing a nested Store per todo.
    todos: SlotMap<TodoKey, Store<Todo>>,
}

#[derive(Debug, Store, Patch, Serialize, Deserialize)]
struct User {
    name: String,
    email: String,
}

#[derive(Debug, Store, Serialize, Deserialize)]
struct Todo {
    id: TodoKey,
    label: String,
    status: Status,
}

#[derive(Debug, Default, Clone, Store, Serialize, Deserialize)]
enum Status {
    #[default]
    Pending,
    Scheduled,
    ScheduledFor {
        date: NaiveDate,
    },
    Done,
}

impl Status {
    pub fn next_step(&mut self) {
        *self = match self {
            Status::Pending => Status::ScheduledFor {
                date: Local::now().naive_local().into(),
            },
            Status::Scheduled | Status::ScheduledFor { .. } => Status::Done,
            Status::Done => Status::Done,
        };
    }
}

impl Todo {
    pub fn new(id: TodoKey, label: impl ToString) -> Self {
        Self {
            id,
            label: label.to_string(),
            status: Status::Pending,
        }
    }
}

fn data() -> Todos {
    let mut map: SlotMap<TodoKey, Store<Todo>> = SlotMap::default();

    ["Create reactive store", "???", "Profit"]
        .into_iter()
        .for_each(|label| {
            map.insert_with_key(|key| Store::new(Todo::new(key, label)));
        });

    Todos {
        user: User {
            name: "Bob".to_string(),
            email: "lawblog@bobloblaw.com".into(),
        },
        todos: map,
    }
}

#[component]
pub fn App() -> impl IntoView {
    let store = Store::new(data());

    let input_ref = NodeRef::new();

    view! {
        <p>"Hello, " {move || store.user().name().get()}</p>
        <UserForm user=store.user() />
        <hr />
        <form on:submit=move |ev| {
            ev.prevent_default();
            store
                .todos()
                .write()
                .insert_with_key(|key| {
                    Store::new(Todo::new(key, input_ref.get().unwrap().value()))
                });
        }>
            <label>"Add a Todo" <input type="text" node_ref=input_ref /></label>
            <input type="submit" />
        </form>

        <For each=move || store.todos().get().into_iter() key=|row| row.0 let:((_,todo))>
            <TodoRow store todo />
        </For>
        <ol></ol>
        // Serialization not implemented on Store<T> <pre>{move || serde_json::to_string_pretty(&*store.read())}</pre>
    }
}

#[component]
fn UserForm(#[prop(into)] user: Field<User>) -> impl IntoView {
    let error = RwSignal::new(None);

    view! {
        {move || error.get().map(|n| view! { <p>{n}</p> })}
        <form on:submit:target=move |ev| {
            ev.prevent_default();
            match User::from_event(&ev) {
                Ok(new_user) => {
                    error.set(None);
                    user.patch(new_user);
                }
                Err(e) => error.set(Some(e.to_string())),
            }
        }>
            <label>
                "Name" <input type="text" name="name" prop:value=move || user.name().get() />
            </label>
            <label>
                "Email" <input type="email" name="email" prop:value=move || user.email().get() />
            </label>
            <input type="submit" />
        </form>
    }
}

#[component]
fn TodoRow(store: Store<Todos>, todo: Store<Todo>) -> impl IntoView {
    let status = todo.status();
    let title = todo.label();

    let editing = RwSignal::new(false);

    view! {
        <li style:text-decoration=move || {
            if todo.status().done() { "line-through" } else { Default::default() }
        }>
            <span
                class:hidden=move || editing.get()
                on:click=move |_| {
                    editing.update(|n| *n = !*n);
                }
            >
                {move || title.get()}
            </span>

            <input
                class:hidden=move || !(editing.get())
                type="text"
                prop:value=move || title.get()
                on:change=move |ev| {
                    title.set(event_target_value(&ev));
                }
                on:keyup=move |e| {
                    if e.key_code() == 13 {
                        editing.set(false);
                    }
                }
                on:focusout=move |_| editing.set(false)
            />

            <button on:click=move |_| {
                status.write().next_step()
            }>
                {move || {
                    if todo.status().done() {
                        "Done"
                    } else if status.scheduled() || status.scheduled_for() {
                        "Scheduled"
                    } else {
                        "Pending"
                    }
                }}

            </button>

            <button on:click=move |_| {
                let id = todo.id().get();
                store.todos().write().retain(|_, todo| todo.id().with(|x| x != &id));
            }>"X"</button>
            <input
                type="date"
                prop:value=move || {
                    todo.status().scheduled_for_date().map(|n| n.get().to_string())
                }

                class:hidden=move || !todo.status().scheduled_for()
                on:change:target=move |ev| {
                    if let Some(date) = todo.status().scheduled_for_date() {
                        let value = ev.target().value();
                        match NaiveDate::parse_from_str(&value, "%Y-%m-%d") {
                            Ok(new_date) => {
                                date.set(new_date);
                            }
                            Err(e) => warn!("{e}"),
                        }
                    }
                }
            />

        </li>
    }
}
