use serde::{Deserialize, Serialize};

const MAX_HISTORY_ITEMS: usize = 24;
const MAX_ENTRY_DIGITS: usize = 16;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct HistoryEntry {
    pub(crate) id: u64,
    pub(crate) expression: String,
    pub(crate) result_text: String,
    pub(crate) result_value: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum BinaryOp {
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum UnaryOp {
    ToggleSign,
    Sqrt,
    Percent,
    Reciprocal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CalcAction {
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

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct CalculatorState {
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
    pub(crate) fn apply(&mut self, action: CalcAction) {
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

    pub(crate) fn display_text(&self) -> String {
        self.error.clone().unwrap_or_else(|| self.entry.clone())
    }

    pub(crate) fn expression_text(&self) -> String {
        if let (Some(acc), Some(op)) = (self.accumulator, self.pending_op) {
            if self.replace_entry {
                format!("{} {}", format_number(acc), op.symbol())
            } else {
                format!("{} {} {}", format_number(acc), op.symbol(), self.entry)
            }
        } else if let Some(last) = self.history.last() {
            format!("Last: {} = {}", last.expression, last.result_text)
        } else {
            "Standard mode (XP) | keyboard shortcuts enabled".to_string()
        }
    }

    pub(crate) fn status_text(&self) -> &'static str {
        if self.error.is_some() {
            "Error"
        } else if self.pending_op.is_some() {
            "Pending operation"
        } else {
            "Ready"
        }
    }

    pub(crate) fn memory_active(&self) -> bool {
        self.memory.abs() > 1e-12
    }

    pub(crate) fn memory_value(&self) -> f64 {
        self.memory
    }

    pub(crate) fn history_count(&self) -> usize {
        self.history.len()
    }

    pub(crate) fn history(&self) -> &[HistoryEntry] {
        &self.history
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

        let digits_only = self.entry.chars().filter(|c| c.is_ascii_digit()).count();
        if digits_only >= MAX_ENTRY_DIGITS {
            return;
        }

        if self.entry == "0" {
            if digit == '0' {
                self.last_equals = None;
                return;
            }
            self.entry = digit.to_string();
            self.last_equals = None;
            return;
        }

        if self.entry == "-0" {
            if digit == '0' {
                self.last_equals = None;
                return;
            }
            self.entry = format!("-{digit}");
            self.last_equals = None;
            return;
        }

        self.entry.push(digit);
        self.last_equals = None;
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
            self.last_equals = None;
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
            self.last_equals = None;
            return;
        }
        self.entry.pop();
        if self.entry.is_empty() || self.entry == "-" {
            self.entry = "0".to_string();
        }
        self.last_equals = None;
    }

    fn clear_entry(&mut self) {
        self.error = None;
        self.entry = "0".to_string();
        self.replace_entry = false;
        self.last_equals = None;
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

        let base = if let Some(pending) = self.pending_op {
            match (self.accumulator, self.replace_entry) {
                (Some(acc), true) => acc,
                (Some(acc), false) => match apply_binary(acc, pending, current) {
                    Ok(value) => value,
                    Err(message) => {
                        self.set_error(message);
                        return;
                    }
                },
                (None, _) => current,
            }
        } else {
            current
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
        self.last_equals = None;

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
        self.last_equals = None;
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

    pub(crate) fn use_value(&mut self, value: f64) {
        if !value.is_finite() {
            return;
        }
        self.error = None;
        self.entry = format_number(value);
        self.replace_entry = true;
        self.last_equals = None;
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

    pub(crate) fn clear_history(&mut self) {
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

pub(crate) fn keyboard_action(key: &str) -> Option<CalcAction> {
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

pub(crate) fn format_number(value: f64) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn apply_all(state: &mut CalculatorState, actions: &[CalcAction]) {
        for action in actions {
            state.apply(*action);
        }
    }

    fn enter_number(state: &mut CalculatorState, text: &str) {
        for ch in text.chars() {
            match ch {
                '0'..='9' => state.apply(CalcAction::Digit(ch)),
                '.' => state.apply(CalcAction::Decimal),
                _ => panic!("unsupported test char: {ch}"),
            }
        }
    }

    fn display(state: &CalculatorState) -> String {
        state.display_text()
    }

    #[test]
    fn default_state_is_ready() {
        let state = CalculatorState::default();
        assert_eq!(display(&state), "0");
        assert_eq!(state.status_text(), "Ready");
        assert_eq!(state.history_count(), 0);
        assert!(!state.memory_active());
        assert!(state.pending_op.is_none());
    }

    #[test]
    fn digit_entry_normalization_and_limits() {
        let mut state = CalculatorState::default();

        apply_all(
            &mut state,
            &[
                CalcAction::Digit('0'),
                CalcAction::DoubleZero,
                CalcAction::Digit('0'),
            ],
        );
        assert_eq!(display(&state), "0");

        apply_all(
            &mut state,
            &[
                CalcAction::Digit('1'),
                CalcAction::Decimal,
                CalcAction::Decimal,
                CalcAction::Digit('2'),
                CalcAction::Backspace,
            ],
        );
        assert_eq!(display(&state), "1.");

        state.apply(CalcAction::Binary(BinaryOp::Add));
        state.apply(CalcAction::Backspace);
        assert_eq!(display(&state), "0");

        state.apply(CalcAction::ClearAll);
        for _ in 0..(MAX_ENTRY_DIGITS + 4) {
            state.apply(CalcAction::Digit('9'));
        }
        assert_eq!(display(&state).len(), MAX_ENTRY_DIGITS);
    }

    #[test]
    fn clear_entry_and_clear_all_behavior() {
        let mut state = CalculatorState::default();

        enter_number(&mut state, "12");
        state.apply(CalcAction::MemoryStore);
        apply_all(
            &mut state,
            &[
                CalcAction::Digit('2'),
                CalcAction::Binary(BinaryOp::Add),
                CalcAction::Digit('3'),
                CalcAction::Equals,
            ],
        );
        assert_eq!(state.history_count(), 1);

        apply_all(
            &mut state,
            &[
                CalcAction::Digit('7'),
                CalcAction::Binary(BinaryOp::Multiply),
                CalcAction::Digit('8'),
            ],
        );
        state.apply(CalcAction::ClearEntry);
        assert_eq!(display(&state), "0");
        assert_eq!(state.status_text(), "Pending operation");
        assert!(state.pending_op.is_some());

        state.apply(CalcAction::ClearAll);
        assert_eq!(display(&state), "0");
        assert_eq!(state.status_text(), "Ready");
        assert!(state.pending_op.is_none());
        assert!(state.accumulator.is_none());
        assert_eq!(state.history_count(), 1);
        assert!(state.memory_active());
        assert_eq!(state.memory_value(), 12.0);
    }

    #[test]
    fn immediate_execution_and_operator_replacement() {
        let mut state = CalculatorState::default();

        apply_all(
            &mut state,
            &[
                CalcAction::Digit('2'),
                CalcAction::Binary(BinaryOp::Add),
                CalcAction::Digit('3'),
                CalcAction::Equals,
            ],
        );
        assert_eq!(display(&state), "5");

        state.apply(CalcAction::ClearAll);
        apply_all(
            &mut state,
            &[
                CalcAction::Digit('2'),
                CalcAction::Binary(BinaryOp::Add),
                CalcAction::Digit('3'),
                CalcAction::Binary(BinaryOp::Multiply),
                CalcAction::Digit('4'),
                CalcAction::Equals,
            ],
        );
        assert_eq!(display(&state), "20");

        state.apply(CalcAction::ClearAll);
        apply_all(
            &mut state,
            &[
                CalcAction::Digit('2'),
                CalcAction::Binary(BinaryOp::Add),
                CalcAction::Binary(BinaryOp::Subtract),
                CalcAction::Digit('3'),
                CalcAction::Equals,
            ],
        );
        assert_eq!(display(&state), "-1");

        state.apply(CalcAction::ClearAll);
        apply_all(
            &mut state,
            &[
                CalcAction::Digit('2'),
                CalcAction::Binary(BinaryOp::Add),
                CalcAction::Equals,
            ],
        );
        assert_eq!(display(&state), "4");
    }

    #[test]
    fn repeated_equals_repeats_last_rhs() {
        let mut state = CalculatorState::default();

        apply_all(
            &mut state,
            &[
                CalcAction::Digit('2'),
                CalcAction::Binary(BinaryOp::Add),
                CalcAction::Digit('3'),
                CalcAction::Equals,
                CalcAction::Equals,
                CalcAction::Equals,
            ],
        );
        assert_eq!(display(&state), "11");

        state.apply(CalcAction::ClearAll);
        apply_all(
            &mut state,
            &[
                CalcAction::Digit('1'),
                CalcAction::Digit('0'),
                CalcAction::Binary(BinaryOp::Subtract),
                CalcAction::Digit('2'),
                CalcAction::Equals,
                CalcAction::Equals,
            ],
        );
        assert_eq!(display(&state), "6");

        state.apply(CalcAction::ClearAll);
        apply_all(
            &mut state,
            &[
                CalcAction::Digit('5'),
                CalcAction::Binary(BinaryOp::Multiply),
                CalcAction::Digit('2'),
                CalcAction::Equals,
                CalcAction::Equals,
            ],
        );
        assert_eq!(display(&state), "20");

        state.apply(CalcAction::ClearAll);
        apply_all(
            &mut state,
            &[
                CalcAction::Digit('2'),
                CalcAction::Digit('0'),
                CalcAction::Binary(BinaryOp::Divide),
                CalcAction::Digit('2'),
                CalcAction::Equals,
                CalcAction::Equals,
            ],
        );
        assert_eq!(display(&state), "5");
    }

    #[test]
    fn new_entry_after_equals_starts_fresh_operation() {
        let mut state = CalculatorState::default();
        apply_all(
            &mut state,
            &[
                CalcAction::Digit('2'),
                CalcAction::Binary(BinaryOp::Add),
                CalcAction::Digit('3'),
                CalcAction::Equals,
            ],
        );
        assert_eq!(display(&state), "5");

        apply_all(
            &mut state,
            &[
                CalcAction::Digit('7'),
                CalcAction::Binary(BinaryOp::Add),
                CalcAction::Digit('1'),
                CalcAction::Equals,
            ],
        );
        assert_eq!(display(&state), "8");

        state.apply(CalcAction::Digit('9'));
        state.apply(CalcAction::Equals);
        assert_eq!(display(&state), "9");
    }

    #[test]
    fn unary_operations_and_percent_semantics() {
        let mut state = CalculatorState::default();

        state.apply(CalcAction::Digit('5'));
        state.apply(CalcAction::Unary(UnaryOp::ToggleSign));
        assert_eq!(display(&state), "-5");
        state.apply(CalcAction::Unary(UnaryOp::ToggleSign));
        assert_eq!(display(&state), "5");

        state.apply(CalcAction::ClearAll);
        state.apply(CalcAction::Unary(UnaryOp::ToggleSign));
        assert_eq!(display(&state), "0");

        state.apply(CalcAction::Digit('9'));
        state.apply(CalcAction::Unary(UnaryOp::Sqrt));
        assert_eq!(display(&state), "3");

        state.apply(CalcAction::Digit('4'));
        state.apply(CalcAction::Unary(UnaryOp::Reciprocal));
        assert_eq!(display(&state), "0.25");

        state.apply(CalcAction::ClearAll);
        enter_number(&mut state, "50");
        state.apply(CalcAction::Unary(UnaryOp::Percent));
        assert_eq!(display(&state), "0.5");

        state.apply(CalcAction::ClearAll);
        enter_number(&mut state, "200");
        state.apply(CalcAction::Binary(BinaryOp::Add));
        enter_number(&mut state, "10");
        state.apply(CalcAction::Unary(UnaryOp::Percent));
        assert_eq!(display(&state), "20");
        state.apply(CalcAction::Equals);
        assert_eq!(display(&state), "220");
    }

    #[test]
    fn error_handling_and_recovery() {
        let mut state = CalculatorState::default();

        apply_all(
            &mut state,
            &[
                CalcAction::Digit('8'),
                CalcAction::Binary(BinaryOp::Divide),
                CalcAction::Digit('0'),
                CalcAction::Equals,
            ],
        );
        assert_eq!(display(&state), "Cannot divide by zero");
        assert_eq!(state.status_text(), "Error");

        state.apply(CalcAction::Digit('7'));
        assert_eq!(display(&state), "7");
        assert_eq!(state.status_text(), "Ready");

        state.apply(CalcAction::ClearAll);
        state.apply(CalcAction::Digit('0'));
        state.apply(CalcAction::Unary(UnaryOp::Reciprocal));
        assert_eq!(display(&state), "Cannot divide by zero");
        state.apply(CalcAction::ClearAll);
        assert_eq!(display(&state), "0");

        enter_number(&mut state, "9");
        state.apply(CalcAction::Unary(UnaryOp::ToggleSign));
        state.apply(CalcAction::Unary(UnaryOp::Sqrt));
        assert_eq!(display(&state), "Invalid input");
        state.apply(CalcAction::Digit('4'));
        assert_eq!(display(&state), "4");
    }

    #[test]
    fn memory_operations_and_indicator_threshold() {
        let mut state = CalculatorState::default();

        enter_number(&mut state, "5");
        state.apply(CalcAction::MemoryStore);
        assert!(state.memory_active());
        assert_eq!(state.memory_value(), 5.0);

        state.apply(CalcAction::ClearEntry);
        enter_number(&mut state, "2");
        state.apply(CalcAction::MemoryAdd);
        assert_eq!(state.memory_value(), 7.0);

        state.apply(CalcAction::ClearEntry);
        state.apply(CalcAction::Digit('1'));
        state.apply(CalcAction::MemorySubtract);
        assert_eq!(state.memory_value(), 6.0);

        state.apply(CalcAction::MemoryRecall);
        assert_eq!(display(&state), "6");

        state.apply(CalcAction::MemoryClear);
        assert!(!state.memory_active());
        assert_eq!(state.memory_value(), 0.0);

        state.memory = 1e-13;
        assert!(!state.memory_active());
        state.memory = 1e-9;
        assert!(state.memory_active());
    }

    #[test]
    fn history_tape_behavior_cap_clear_and_reuse() {
        let mut state = CalculatorState::default();

        apply_all(
            &mut state,
            &[
                CalcAction::Digit('2'),
                CalcAction::Binary(BinaryOp::Add),
                CalcAction::Digit('3'),
                CalcAction::Equals,
            ],
        );
        assert_eq!(state.history_count(), 1);
        let first = state.history().last().expect("history item");
        assert_eq!(first.expression, "2 + 3");
        assert_eq!(first.result_text, "5");
        assert_eq!(first.result_value, 5.0);

        state.apply(CalcAction::UseLast);
        assert_eq!(display(&state), "5");
        apply_all(
            &mut state,
            &[
                CalcAction::Binary(BinaryOp::Add),
                CalcAction::Digit('1'),
                CalcAction::Equals,
            ],
        );
        assert_eq!(display(&state), "6");

        for _ in 0..(MAX_HISTORY_ITEMS + 5) {
            state.apply(CalcAction::ClearAll);
            enter_number(&mut state, "50");
            state.apply(CalcAction::Unary(UnaryOp::Percent));
        }
        assert_eq!(state.history_count(), MAX_HISTORY_ITEMS);

        let preserved_display = display(&state);
        state.clear_history();
        assert_eq!(state.history_count(), 0);
        assert_eq!(display(&state), preserved_display);
    }

    #[test]
    fn keyboard_action_maps_supported_keys() {
        assert_eq!(keyboard_action("0"), Some(CalcAction::Digit('0')));
        assert_eq!(
            keyboard_action("+"),
            Some(CalcAction::Binary(BinaryOp::Add))
        );
        assert_eq!(
            keyboard_action("*"),
            Some(CalcAction::Binary(BinaryOp::Multiply))
        );
        assert_eq!(keyboard_action("Enter"), Some(CalcAction::Equals));
        assert_eq!(keyboard_action("Backspace"), Some(CalcAction::Backspace));
        assert_eq!(keyboard_action("Delete"), Some(CalcAction::ClearEntry));
        assert_eq!(keyboard_action("Escape"), Some(CalcAction::ClearAll));
        assert_eq!(
            keyboard_action("F9"),
            Some(CalcAction::Unary(UnaryOp::ToggleSign))
        );
        assert_eq!(keyboard_action("nope"), None);
    }
}
