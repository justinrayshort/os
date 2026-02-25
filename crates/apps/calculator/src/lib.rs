mod engine;

use crate::engine::{
    format_number, keyboard_action, BinaryOp, CalcAction, CalculatorState, UnaryOp,
};
use leptos::ev::KeyboardEvent;
use leptos::*;
use serde_json::Value;

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
pub fn CalculatorApp(launch_params: Value) -> impl IntoView {
    let _ = launch_params;
    let calc = create_rw_signal(CalculatorState::default());

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
        <div class="app-shell app-calculator-shell">
            <div class="app-menubar calc-toolbar" aria-label="Calculator menu and shortcuts">
                <button type="button">"Edit"</button>
                <button type="button">"View"</button>
                <button type="button">"Help"</button>
                <span class="calc-toolbar-divider" aria-hidden="true"></span>
                <button type="button" on:click=move |_| calc.update(|s| s.apply(CalcAction::UseLast))>
                    "Reuse Last"
                </button>
                <button type="button" on:click=move |_| calc.update(|s| s.clear_history())>
                    "Clear Tape"
                </button>
                <button type="button" on:click=move |_| calc.update(|s| s.apply(CalcAction::ClearAll))>
                    "Reset"
                </button>
            </div>

            <div class="calculator-workspace" tabindex="0" on:keydown=on_keydown>
                <section class="calculator-main" aria-label="Calculator keypad">
                    <div
                        class="calc-display-panel"
                        data-memory=move || if calc.get().memory_active() { "on" } else { "off" }
                    >
                        <div class="calc-display-meta">
                            <span class="calc-mode">"Standard"</span>
                            <span class="calc-memory-indicator">{move || if calc.get().memory_active() { "M" } else { "" }}</span>
                            <span class="calc-status">{move || calc.get().status_text()}</span>
                        </div>
                        <div class="calc-expression" aria-live="off">{move || calc.get().expression_text()}</div>
                        <div class="calc-display" role="status" aria-live="polite">{move || calc.get().display_text()}</div>
                    </div>

                    <div class="calc-keypad" role="group" aria-label="Calculator keys">
                        <For
                            each=move || CALC_KEYS.to_vec()
                            key=|spec| spec.id
                            let:spec
                        >
                            <button
                                type="button"
                                class=format!("calc-key {}", spec.class_name)
                                title=spec.title
                                on:click=move |_| calc.update(|state| state.apply(spec.action))
                            >
                                {spec.label}
                            </button>
                        </For>
                    </div>
                </section>

                <aside class="calc-tape" aria-label="Recent calculations">
                    <div class="calc-tape-header">
                        <div>
                            <strong>"Tape"</strong>
                            <span>{move || format!("{} item(s)", calc.get().history_count())}</span>
                        </div>
                        <button type="button" on:click=move |_| calc.update(|s| s.clear_history())>
                            "Clear"
                        </button>
                    </div>

                    <div class="calc-tape-list" role="list">
                        <Show
                            when=move || { calc.get().history_count() > 0 }
                            fallback=|| {
                                view! {
                                    <p class="calc-empty-tape">
                                        "Recent results appear here. Click a result to reuse it."
                                    </p>
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
                                <button
                                    type="button"
                                    class="calc-tape-item"
                                    on:click=move |_| {
                                        let value = item.result_value;
                                        calc.update(|state| state.use_value(value));
                                    }
                                >
                                    <span class="calc-tape-expr">{item.expression}</span>
                                    <span class="calc-tape-result">{format!("= {}", item.result_text)}</span>
                                </button>
                            </For>
                        </Show>
                    </div>
                </aside>
            </div>

            <div class="app-statusbar">
                <span>"Keys: 0-9, + - * /, Enter, Backspace, Esc, Del, F9"</span>
                <span>
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
                </span>
                <span>"Click tape item to restore result"</span>
            </div>
        </div>
    }
}
