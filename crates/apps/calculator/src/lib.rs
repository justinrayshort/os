use leptos::ev::KeyboardEvent;
use leptos::*;
use serde_json::Value;

const MAX_HISTORY_ITEMS: usize = 24;
const MAX_ENTRY_DIGITS: usize = 16;

#[derive(Clone)]
struct HistoryEntry {
    id: u64,
    expression: String,
    result_text: String,
    result_value: f64,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
}

impl BinaryOp {
    fn symbol(self) -> &'static str {
        match self {
            Self::Add => "+",
            Self::Subtract => "-",
            Self::Multiply => "*",
            Self::Divide => "/",
        }
    }
}

#[derive(Clone, Copy)]
enum UnaryOp {
    ToggleSign,
    Sqrt,
    Percent,
    Reciprocal,
}

#[derive(Clone, Copy)]
enum CalcAction {
    Digit(char),
    DoubleZero,
    Decimal,
    Backspace,
    ClearEntry,
    ClearAll,
    Binary(BinaryOp),
    Unary(UnaryOp),
    Equals,
    MemoryClear,
    MemoryRecall,
    MemoryStore,
    MemoryAdd,
    MemorySubtract,
    UseLast,
}

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

#[derive(Clone)]
struct CalculatorState {
    entry: String,
    accumulator: Option<f64>,
    pending_op: Option<BinaryOp>,
    last_equals: Option<(BinaryOp, f64)>,
    replace_entry: bool,
    memory: f64,
    history: Vec<HistoryEntry>,
    next_history_id: u64,
    error: Option<String>,
}

impl Default for CalculatorState {
    fn default() -> Self {
        Self {
            entry: "0".to_string(),
            accumulator: None,
            pending_op: None,
            last_equals: None,
            replace_entry: false,
            memory: 0.0,
            history: Vec::new(),
            next_history_id: 1,
            error: None,
        }
    }
}

impl CalculatorState {
    fn apply(&mut self, action: CalcAction) {
        match action {
            CalcAction::Digit(digit) => self.input_digit(digit),
            CalcAction::DoubleZero => {
                self.input_digit('0');
                self.input_digit('0');
            }
            CalcAction::Decimal => self.input_decimal(),
            CalcAction::Backspace => self.backspace(),
            CalcAction::ClearEntry => self.clear_entry(),
            CalcAction::ClearAll => self.clear_all(),
            CalcAction::Binary(op) => self.set_pending_operation(op),
            CalcAction::Unary(op) => self.apply_unary(op),
            CalcAction::Equals => self.equals(),
            CalcAction::MemoryClear => self.memory = 0.0,
            CalcAction::MemoryRecall => {
                let value = self.memory;
                self.use_value(value);
            }
            CalcAction::MemoryStore => {
                if let Some(value) = self.current_value() {
                    self.memory = value;
                }
            }
            CalcAction::MemoryAdd => {
                if let Some(value) = self.current_value() {
                    self.memory += value;
                }
            }
            CalcAction::MemorySubtract => {
                if let Some(value) = self.current_value() {
                    self.memory -= value;
                }
            }
            CalcAction::UseLast => self.use_last_result(),
        }
    }

    fn display_text(&self) -> String {
        self.error.clone().unwrap_or_else(|| self.entry.clone())
    }

    fn expression_text(&self) -> String {
        if let (Some(acc), Some(op)) = (self.accumulator, self.pending_op) {
            if self.replace_entry {
                format!("{} {}", format_number(acc), op.symbol())
            } else {
                format!("{} {} {}", format_number(acc), op.symbol(), self.entry)
            }
        } else if let Some(last) = self.history.last() {
            format!("Last: {} = {}", last.expression, last.result_text)
        } else {
            "Standard mode (95) | keyboard shortcuts enabled".to_string()
        }
    }

    fn status_text(&self) -> &'static str {
        if self.error.is_some() {
            "Error"
        } else if self.pending_op.is_some() {
            "Pending operation"
        } else {
            "Ready"
        }
    }

    fn memory_active(&self) -> bool {
        self.memory.abs() > 1e-12
    }

    fn history_count(&self) -> usize {
        self.history.len()
    }

    fn current_value(&self) -> Option<f64> {
        if self.error.is_some() {
            return None;
        }
        self.entry.parse::<f64>().ok()
    }

    fn input_digit(&mut self, digit: char) {
        if self.error.is_some() {
            self.clear_all();
        }

        if self.replace_entry {
            self.entry = "0".to_string();
            self.replace_entry = false;
        }

        let is_negative = self.entry.starts_with('-');
        let digits_only = self.entry.chars().filter(|c| c.is_ascii_digit()).count();
        if digits_only >= MAX_ENTRY_DIGITS {
            return;
        }

        if self.entry == "0" {
            if digit == '0' {
                return;
            }
            self.entry = digit.to_string();
            return;
        }

        if self.entry == "-0" {
            if digit == '0' {
                return;
            }
            self.entry = format!("-{digit}");
            return;
        }

        if is_negative && self.entry.len() == 0 {
            self.entry = format!("-{digit}");
            return;
        }

        self.entry.push(digit);
    }

    fn input_decimal(&mut self) {
        if self.error.is_some() {
            self.clear_all();
        }
        if self.replace_entry {
            self.entry = "0".to_string();
            self.replace_entry = false;
        }
        if !self.entry.contains('.') {
            self.entry.push('.');
        }
    }

    fn backspace(&mut self) {
        if self.error.is_some() {
            self.clear_all();
            return;
        }
        if self.replace_entry {
            self.entry = "0".to_string();
            self.replace_entry = false;
            return;
        }
        self.entry.pop();
        if self.entry.is_empty() || self.entry == "-" {
            self.entry = "0".to_string();
        }
    }

    fn clear_entry(&mut self) {
        self.error = None;
        self.entry = "0".to_string();
        self.replace_entry = false;
    }

    fn clear_all(&mut self) {
        self.entry = "0".to_string();
        self.accumulator = None;
        self.pending_op = None;
        self.last_equals = None;
        self.replace_entry = false;
        self.error = None;
    }

    fn set_pending_operation(&mut self, op: BinaryOp) {
        let Some(current) = self.current_value() else {
            return;
        };

        let base = match (self.accumulator, self.pending_op, self.replace_entry) {
            (Some(acc), Some(_), true) => acc,
            (Some(acc), Some(pending), false) => match apply_binary(acc, pending, current) {
                Ok(value) => value,
                Err(message) => {
                    self.set_error(message);
                    return;
                }
            },
            (Some(acc), None, _) => acc,
            (None, _, _) => current,
        };

        self.accumulator = Some(base);
        self.pending_op = Some(op);
        self.last_equals = None;
        self.entry = format_number(base);
        self.replace_entry = true;
    }

    fn apply_unary(&mut self, op: UnaryOp) {
        let Some(current) = self.current_value() else {
            return;
        };

        if matches!(op, UnaryOp::ToggleSign) {
            self.toggle_sign();
            return;
        }

        let before = self.entry.clone();
        let result = match op {
            UnaryOp::Sqrt => {
                if current < 0.0 {
                    self.set_error("Invalid input");
                    return;
                }
                current.sqrt()
            }
            UnaryOp::Percent => {
                if let (Some(acc), Some(_)) = (self.accumulator, self.pending_op) {
                    acc * current / 100.0
                } else {
                    current / 100.0
                }
            }
            UnaryOp::Reciprocal => {
                if current == 0.0 {
                    self.set_error("Cannot divide by zero");
                    return;
                }
                1.0 / current
            }
            UnaryOp::ToggleSign => unreachable!(),
        };

        let result_text = format_number(result);
        self.entry = result_text.clone();
        self.error = None;
        self.replace_entry = true;

        let expression = match op {
            UnaryOp::Sqrt => format!("sqrt({before})"),
            UnaryOp::Percent => format!("percent({before})"),
            UnaryOp::Reciprocal => format!("1/({before})"),
            UnaryOp::ToggleSign => return,
        };
        self.push_history(expression, result_text, result);
    }

    fn toggle_sign(&mut self) {
        if self.error.is_some() {
            return;
        }
        if self.entry == "0" || self.entry == "0." {
            return;
        }
        if self.entry.starts_with('-') {
            self.entry.remove(0);
        } else {
            self.entry = format!("-{}", self.entry);
        }
        self.replace_entry = false;
    }

    fn equals(&mut self) {
        if self.error.is_some() {
            return;
        }

        if let (Some(acc), Some(op)) = (self.accumulator, self.pending_op) {
            let Some(rhs) = self.current_value() else {
                return;
            };
            match apply_binary(acc, op, rhs) {
                Ok(result) => {
                    let lhs_text = format_number(acc);
                    let rhs_text = format_number(rhs);
                    let result_text = format_number(result);
                    self.push_history(
                        format!("{lhs_text} {} {rhs_text}", op.symbol()),
                        result_text.clone(),
                        result,
                    );
                    self.accumulator = Some(result);
                    self.pending_op = None;
                    self.last_equals = Some((op, rhs));
                    self.entry = result_text;
                    self.replace_entry = true;
                    self.error = None;
                }
                Err(message) => self.set_error(message),
            }
            return;
        }

        if let Some((op, rhs)) = self.last_equals {
            let Some(lhs) = self.current_value() else {
                return;
            };
            match apply_binary(lhs, op, rhs) {
                Ok(result) => {
                    let lhs_text = format_number(lhs);
                    let rhs_text = format_number(rhs);
                    let result_text = format_number(result);
                    self.push_history(
                        format!("{lhs_text} {} {rhs_text}", op.symbol()),
                        result_text.clone(),
                        result,
                    );
                    self.accumulator = Some(result);
                    self.entry = result_text;
                    self.replace_entry = true;
                    self.error = None;
                }
                Err(message) => self.set_error(message),
            }
        }
    }

    fn use_last_result(&mut self) {
        if let Some(last) = self.history.last() {
            self.use_value(last.result_value);
        }
    }

    fn use_value(&mut self, value: f64) {
        if !value.is_finite() {
            return;
        }
        self.error = None;
        self.entry = format_number(value);
        self.replace_entry = true;
    }

    fn push_history(&mut self, expression: String, result_text: String, result_value: f64) {
        let id = self.next_history_id;
        self.next_history_id = self.next_history_id.saturating_add(1);
        self.history.push(HistoryEntry {
            id,
            expression,
            result_text,
            result_value,
        });
        if self.history.len() > MAX_HISTORY_ITEMS {
            let overflow = self.history.len() - MAX_HISTORY_ITEMS;
            self.history.drain(0..overflow);
        }
    }

    fn clear_history(&mut self) {
        self.history.clear();
    }

    fn set_error(&mut self, message: &str) {
        self.error = Some(message.to_string());
        self.entry = "0".to_string();
        self.accumulator = None;
        self.pending_op = None;
        self.last_equals = None;
        self.replace_entry = true;
    }
}

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
            <div class="app-menubar" aria-label="Calculator menu">
                <button type="button">"Edit"</button>
                <button type="button">"View"</button>
                <button type="button">"Help"</button>
            </div>

            <div class="app-toolbar calc-toolbar" aria-label="Calculator shortcuts">
                <button type="button" on:click=move |_| calc.update(|s| s.apply(CalcAction::UseLast))>
                    "Reuse Last"
                </button>
                <button type="button" on:click=move |_| calc.update(|s| s.clear_history())>
                    "Clear Tape"
                </button>
                <button type="button" on:click=move |_| calc.update(|s| s.apply(CalcAction::ClearAll))>
                    "Reset"
                </button>
                <span class="calc-toolbar-note">"95 shell, modern tape + keyboard workflow"</span>
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
                                    let mut items = calc.get().history.clone();
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
                <span>{move || format!("Memory: {}", if calc.get().memory_active() { format_number(calc.get().memory) } else { "Empty".to_string() })}</span>
                <span>"Click tape item to restore result"</span>
            </div>
        </div>
    }
}

fn keyboard_action(key: &str) -> Option<CalcAction> {
    match key {
        "0" => Some(CalcAction::Digit('0')),
        "1" => Some(CalcAction::Digit('1')),
        "2" => Some(CalcAction::Digit('2')),
        "3" => Some(CalcAction::Digit('3')),
        "4" => Some(CalcAction::Digit('4')),
        "5" => Some(CalcAction::Digit('5')),
        "6" => Some(CalcAction::Digit('6')),
        "7" => Some(CalcAction::Digit('7')),
        "8" => Some(CalcAction::Digit('8')),
        "9" => Some(CalcAction::Digit('9')),
        "." | "," => Some(CalcAction::Decimal),
        "+" => Some(CalcAction::Binary(BinaryOp::Add)),
        "-" => Some(CalcAction::Binary(BinaryOp::Subtract)),
        "*" | "x" | "X" => Some(CalcAction::Binary(BinaryOp::Multiply)),
        "/" => Some(CalcAction::Binary(BinaryOp::Divide)),
        "%" => Some(CalcAction::Unary(UnaryOp::Percent)),
        "=" | "Enter" => Some(CalcAction::Equals),
        "Backspace" => Some(CalcAction::Backspace),
        "Delete" => Some(CalcAction::ClearEntry),
        "Escape" => Some(CalcAction::ClearAll),
        "F9" => Some(CalcAction::Unary(UnaryOp::ToggleSign)),
        _ => None,
    }
}

fn apply_binary(lhs: f64, op: BinaryOp, rhs: f64) -> Result<f64, &'static str> {
    let result = match op {
        BinaryOp::Add => lhs + rhs,
        BinaryOp::Subtract => lhs - rhs,
        BinaryOp::Multiply => lhs * rhs,
        BinaryOp::Divide => {
            if rhs == 0.0 {
                return Err("Cannot divide by zero");
            }
            lhs / rhs
        }
    };

    if result.is_finite() {
        Ok(result)
    } else {
        Err("Overflow")
    }
}

fn format_number(value: f64) -> String {
    if value == 0.0 {
        return "0".to_string();
    }

    if value.fract() == 0.0 && value.abs() < 1e15 {
        return format!("{value:.0}");
    }

    let mut text = format!("{value:.12}");
    while text.contains('.') && text.ends_with('0') {
        text.pop();
    }
    if text.ends_with('.') {
        text.pop();
    }
    text
}
