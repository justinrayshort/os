//! Calculator desktop app UI component and persistence integration.

#![warn(missing_docs, rustdoc::broken_intra_doc_links)]

mod engine;

use crate::engine::{
    format_number, keyboard_action, BinaryOp, CalcAction, CalculatorState, UnaryOp,
};
use desktop_app_contract::AppHost;
use leptos::ev::KeyboardEvent;
use leptos::*;
use platform_storage::{self, CALCULATOR_STATE_NAMESPACE};
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

const CALCULATOR_STATE_SCHEMA_VERSION: u32 = 1;

fn migrate_calculator_state(
    schema_version: u32,
    envelope: &platform_storage::AppStateEnvelope,
) -> Result<Option<CalculatorState>, String> {
    match schema_version {
        0 => platform_storage::migrate_envelope_payload(envelope).map(Some),
        _ => Ok(None),
    }
}

#[component]
/// Calculator app window contents.
///
/// The component restores and persists calculator state through [`platform_storage`].
pub fn CalculatorApp(
    /// App launch parameters from the desktop runtime (currently unused).
    launch_params: Value,
    /// Manager-restored app state payload for this window instance.
    restored_state: Option<Value>,
    /// Optional app-host bridge for manager-owned commands.
    host: Option<AppHost>,
) -> impl IntoView {
    let _ = launch_params;
    let calc = create_rw_signal(CalculatorState::default());
    let hydrated = create_rw_signal(false);
    let last_saved = create_rw_signal::<Option<String>>(None);
    let host_for_persist = host;

    if let Some(restored_state) = restored_state.as_ref() {
        if let Ok(restored) = serde_json::from_value::<CalculatorState>(restored_state.clone()) {
            let serialized = serde_json::to_string(&restored).ok();
            calc.set(restored);
            last_saved.set(serialized);
            hydrated.set(true);
        }
    }
    create_effect(move |_| {
        let calc = calc;
        let hydrated = hydrated;
        let last_saved = last_saved;
        spawn_local(async move {
            match platform_storage::load_app_state_with_migration::<CalculatorState, _>(
                CALCULATOR_STATE_NAMESPACE,
                CALCULATOR_STATE_SCHEMA_VERSION,
                migrate_calculator_state,
            )
            .await
            {
                Ok(Some(restored)) => {
                    if last_saved.get_untracked().is_none() {
                        let serialized = serde_json::to_string(&restored).ok();
                        calc.set(restored);
                        last_saved.set(serialized);
                    }
                }
                Ok(None) => {}
                Err(err) => logging::warn!("calculator state hydrate failed: {err}"),
            }
            hydrated.set(true);
        });
    });

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

        if let Some(host) = host_for_persist {
            if let Ok(value) = serde_json::to_value(&snapshot) {
                host.persist_state(value);
            }
        }

        spawn_local(async move {
            if let Err(err) = platform_storage::save_app_state(
                CALCULATOR_STATE_NAMESPACE,
                CALCULATOR_STATE_SCHEMA_VERSION,
                &snapshot,
            )
            .await
            {
                logging::warn!("calculator state persist failed: {err}");
            }
        });
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
        <div class="app-shell app-calculator-shell">
            <div class="app-menubar calc-toolbar" aria-label="Calculator menu and shortcuts">
                <button type="button" class="app-action">"Edit"</button>
                <button type="button" class="app-action">"View"</button>
                <button type="button" class="app-action">"Help"</button>
                <span class="calc-toolbar-divider" aria-hidden="true"></span>
                <button type="button" class="app-action" on:click=move |_| calc.update(|s| s.apply(CalcAction::UseLast))>
                    "Reuse Last"
                </button>
                <button type="button" class="app-action" on:click=move |_| calc.update(|s| s.clear_history())>
                    "Clear Tape"
                </button>
                <button type="button" class="app-action" on:click=move |_| calc.update(|s| s.apply(CalcAction::ClearAll))>
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
                                class=format!("calc-key app-action {}", spec.class_name)
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
                        <button type="button" class="app-action" on:click=move |_| calc.update(|s| s.clear_history())>
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
                                    class="calc-tape-item app-action"
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
                <span>{move || if hydrated.get() { "State: synced" } else { "State: hydrating..." }}</span>
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculator_namespace_migration_supports_schema_zero() {
        let envelope = platform_storage::build_app_state_envelope(
            CALCULATOR_STATE_NAMESPACE,
            0,
            &CalculatorState::default(),
        )
        .expect("build envelope");

        let migrated =
            migrate_calculator_state(0, &envelope).expect("schema-zero migration should succeed");
        assert!(migrated.is_some(), "expected migrated calculator state");
    }
}
