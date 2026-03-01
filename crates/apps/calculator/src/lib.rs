//! Calculator desktop app UI component and persistence integration.
//!
//! The app persists calculator memory/tape state through the runtime-managed app-state channel and
//! renders its keypad and display entirely through the shared `system_ui` primitive set.

#![warn(missing_docs, rustdoc::broken_intra_doc_links)]

mod engine;

use crate::engine::{
    format_number, keyboard_action, BinaryOp, CalcAction, CalculatorState, UnaryOp,
};
use desktop_app_contract::AppServices;
use leptos::ev::KeyboardEvent;
use leptos::*;
use serde_json::Value;
use system_ui::prelude::*;

#[derive(Clone, Copy)]
struct CalcKeySpec {
    id: &'static str,
    label: &'static str,
    class_name: &'static str,
    title: &'static str,
    action: CalcAction,
}

const CALC_KEYS: [CalcKeySpec; 30] = [
    CalcKeySpec {
        id: "mc",
        label: "MC",
        class_name: "memory",
        title: "Clear memory",
        action: CalcAction::MemoryClear,
    },
    CalcKeySpec {
        id: "mr",
        label: "MR",
        class_name: "memory",
        title: "Recall memory",
        action: CalcAction::MemoryRecall,
    },
    CalcKeySpec {
        id: "ms",
        label: "MS",
        class_name: "memory",
        title: "Store memory",
        action: CalcAction::MemoryStore,
    },
    CalcKeySpec {
        id: "mplus",
        label: "M+",
        class_name: "memory",
        title: "Add to memory",
        action: CalcAction::MemoryAdd,
    },
    CalcKeySpec {
        id: "mminus",
        label: "M-",
        class_name: "memory",
        title: "Subtract from memory",
        action: CalcAction::MemorySubtract,
    },
    CalcKeySpec {
        id: "back",
        label: "Back",
        class_name: "util",
        title: "Backspace",
        action: CalcAction::Backspace,
    },
    CalcKeySpec {
        id: "ce",
        label: "CE",
        class_name: "util",
        title: "Clear entry",
        action: CalcAction::ClearEntry,
    },
    CalcKeySpec {
        id: "c",
        label: "C",
        class_name: "util danger",
        title: "Clear all",
        action: CalcAction::ClearAll,
    },
    CalcKeySpec {
        id: "sign",
        label: "+/-",
        class_name: "util",
        title: "Toggle sign (F9)",
        action: CalcAction::Unary(UnaryOp::ToggleSign),
    },
    CalcKeySpec {
        id: "sqrt",
        label: "sqrt",
        class_name: "util",
        title: "Square root",
        action: CalcAction::Unary(UnaryOp::Sqrt),
    },
    CalcKeySpec {
        id: "7",
        label: "7",
        class_name: "digit",
        title: "7",
        action: CalcAction::Digit('7'),
    },
    CalcKeySpec {
        id: "8",
        label: "8",
        class_name: "digit",
        title: "8",
        action: CalcAction::Digit('8'),
    },
    CalcKeySpec {
        id: "9",
        label: "9",
        class_name: "digit",
        title: "9",
        action: CalcAction::Digit('9'),
    },
    CalcKeySpec {
        id: "divide",
        label: "/",
        class_name: "operator",
        title: "Divide",
        action: CalcAction::Binary(BinaryOp::Divide),
    },
    CalcKeySpec {
        id: "percent",
        label: "%",
        class_name: "operator",
        title: "Percent",
        action: CalcAction::Unary(UnaryOp::Percent),
    },
    CalcKeySpec {
        id: "4",
        label: "4",
        class_name: "digit",
        title: "4",
        action: CalcAction::Digit('4'),
    },
    CalcKeySpec {
        id: "5",
        label: "5",
        class_name: "digit",
        title: "5",
        action: CalcAction::Digit('5'),
    },
    CalcKeySpec {
        id: "6",
        label: "6",
        class_name: "digit",
        title: "6",
        action: CalcAction::Digit('6'),
    },
    CalcKeySpec {
        id: "mul",
        label: "*",
        class_name: "operator",
        title: "Multiply",
        action: CalcAction::Binary(BinaryOp::Multiply),
    },
    CalcKeySpec {
        id: "inv",
        label: "1/x",
        class_name: "util",
        title: "Reciprocal",
        action: CalcAction::Unary(UnaryOp::Reciprocal),
    },
    CalcKeySpec {
        id: "1",
        label: "1",
        class_name: "digit",
        title: "1",
        action: CalcAction::Digit('1'),
    },
    CalcKeySpec {
        id: "2",
        label: "2",
        class_name: "digit",
        title: "2",
        action: CalcAction::Digit('2'),
    },
    CalcKeySpec {
        id: "3",
        label: "3",
        class_name: "digit",
        title: "3",
        action: CalcAction::Digit('3'),
    },
    CalcKeySpec {
        id: "sub",
        label: "-",
        class_name: "operator",
        title: "Subtract",
        action: CalcAction::Binary(BinaryOp::Subtract),
    },
    CalcKeySpec {
        id: "eq",
        label: "=",
        class_name: "operator equals",
        title: "Equals (Enter)",
        action: CalcAction::Equals,
    },
    CalcKeySpec {
        id: "0",
        label: "0",
        class_name: "digit",
        title: "0",
        action: CalcAction::Digit('0'),
    },
    CalcKeySpec {
        id: "00",
        label: "00",
        class_name: "digit",
        title: "Double zero",
        action: CalcAction::DoubleZero,
    },
    CalcKeySpec {
        id: "dot",
        label: ".",
        class_name: "digit",
        title: "Decimal point",
        action: CalcAction::Decimal,
    },
    CalcKeySpec {
        id: "add",
        label: "+",
        class_name: "operator",
        title: "Add",
        action: CalcAction::Binary(BinaryOp::Add),
    },
    CalcKeySpec {
        id: "ans",
        label: "Ans",
        class_name: "util accent",
        title: "Reuse last result",
        action: CalcAction::UseLast,
    },
];

#[component]
/// Calculator app window contents.
///
/// The component restores and persists calculator state through typed host contracts.
pub fn CalculatorApp(
    /// App launch parameters from the desktop runtime (currently unused).
    launch_params: Value,
    /// Manager-restored app state payload for this window instance.
    restored_state: Option<Value>,
    /// Optional app-host bridge for manager-owned commands.
    services: Option<AppServices>,
) -> impl IntoView {
    let _ = launch_params;
    let calc = create_rw_signal(CalculatorState::default());
    let hydrated = create_rw_signal(false);
    let last_saved = create_rw_signal::<Option<String>>(None);
    let services_for_persist = services.clone();

    if let Some(restored_state) = restored_state.as_ref() {
        if let Ok(restored) = serde_json::from_value::<CalculatorState>(restored_state.clone()) {
            let serialized = serde_json::to_string(&restored).ok();
            calc.set(restored);
            last_saved.set(serialized);
            hydrated.set(true);
        }
    }
    hydrated.set(true);

    create_effect(move |_| {
        if !hydrated.get() {
            return;
        }

        let snapshot = calc.get();
        let serialized = match serde_json::to_string(&snapshot) {
            Ok(raw) => raw,
            Err(err) => {
                logging::warn!("calculator state serialize failed: {err}");
                return;
            }
        };

        if last_saved.get().as_deref() == Some(serialized.as_str()) {
            return;
        }
        last_saved.set(Some(serialized));

        if let Some(services) = services_for_persist.clone() {
            if let Ok(value) = serde_json::to_value(&snapshot) {
                services.state.persist_window_state(value);
            }
        }
    });

    let on_keydown = move |ev: KeyboardEvent| {
        if ev.ctrl_key() || ev.meta_key() || ev.alt_key() {
            return;
        }

        if let Some(action) = keyboard_action(&ev.key()) {
            ev.prevent_default();
            calc.update(|state| state.apply(action));
        }
    };

    view! {
        <AppShell>
            <MenuBar aria_label="Calculator menu and shortcuts">
                <Button variant=ButtonVariant::Quiet>"Edit"</Button>
                <Button variant=ButtonVariant::Quiet>"View"</Button>
                <Button variant=ButtonVariant::Quiet>"Help"</Button>
                <Button variant=ButtonVariant::Quiet on_click=Callback::new(move |_| calc.update(|s| s.apply(CalcAction::UseLast)))>
                    "Reuse Last"
                </Button>
                <Button variant=ButtonVariant::Quiet on_click=Callback::new(move |_| calc.update(|s| s.clear_history()))>
                    "Clear Tape"
                </Button>
                <Button
                    variant=ButtonVariant::Danger
                    on_click=Callback::new(move |_| calc.update(|s| s.apply(CalcAction::ClearAll)))
                >
                    "Reset"
                </Button>
            </MenuBar>

            <SplitLayout ui_slot="workspace" tabindex=0 on_keydown=Callback::new(on_keydown)>
                <Pane ui_slot="primary-pane" aria_label="Calculator keypad">
                    <Panel ui_slot="display-panel">
                        <div data-ui-slot="meta">
                            <span data-ui-slot="badge">"Standard"</span>
                            <span data-ui-slot="badge">{move || if calc.get().memory_active() { "M" } else { "" }}</span>
                            <span data-ui-slot="status">{move || calc.get().status_text()}</span>
                        </div>
                        <div data-ui-slot="expression" aria-live="off">{move || calc.get().expression_text()}</div>
                        <div data-ui-slot="display" role="status" aria-live="polite">{move || calc.get().display_text()}</div>
                    </Panel>

                    <div data-ui-slot="keypad" role="group" aria-label="Calculator keys">
                        <For
                            each=move || CALC_KEYS.to_vec()
                            key=|spec| spec.id
                            let:spec
                        >
                            <Button
                                variant=if spec.class_name.contains("danger") {
                                    ButtonVariant::Danger
                                } else if spec.class_name.contains("accent") || spec.class_name.contains("equals") {
                                    ButtonVariant::Primary
                                } else if spec.class_name.contains("memory") {
                                    ButtonVariant::Quiet
                                } else {
                                    ButtonVariant::Standard
                                }
                                title=spec.title
                                on_click=Callback::new(move |_| calc.update(|state| state.apply(spec.action)))
                            >
                                {spec.label}
                            </Button>
                        </For>
                    </div>
                </Pane>

                <Pane ui_slot="secondary-pane" aria_label="Recent calculations">
                    <PaneHeader
                        title="Tape"
                        meta=Signal::derive(move || format!("{} item(s)", calc.get().history_count()))
                    >
                        <Button
                            variant=ButtonVariant::Quiet
                            on_click=Callback::new(move |_| calc.update(|s| s.clear_history()))
                        >
                            "Clear"
                        </Button>
                    </PaneHeader>

                    <ListSurface role="list">
                        <Show
                            when=move || { calc.get().history_count() > 0 }
                            fallback=|| {
                                view! {
                                    <EmptyState>
                                        "Recent results appear here. Click a result to reuse it."
                                    </EmptyState>
                                }
                            }
                        >
                            <For
                                each=move || {
                                    let mut items = calc.get().history().to_vec();
                                    items.reverse();
                                    items
                                }
                                key=|item| item.id
                                let:item
                            >
                                <Button
                                    ui_slot="list-item"
                                    variant=ButtonVariant::Quiet
                                    on_click=Callback::new(move |_| {
                                        let value = item.result_value;
                                        calc.update(|state| state.use_value(value));
                                    })
                                >
                                    <span>{item.expression}</span>
                                    <span>{format!("= {}", item.result_text)}</span>
                                </Button>
                            </For>
                        </Show>
                    </ListSurface>
                </Pane>
            </SplitLayout>

            <StatusBar>
                <StatusBarItem>"Keys: 0-9, + - * /, Enter, Backspace, Esc, Del, F9"</StatusBarItem>
                <StatusBarItem>
                    {move || {
                        let state = calc.get();
                        format!(
                            "Memory: {}",
                            if state.memory_active() {
                                format_number(state.memory_value())
                            } else {
                                "Empty".to_string()
                            }
                        )
                    }}
                </StatusBarItem>
                <StatusBarItem>{move || if hydrated.get() { "State: synced" } else { "State: hydrating..." }}</StatusBarItem>
            </StatusBar>
        </AppShell>
    }
}
