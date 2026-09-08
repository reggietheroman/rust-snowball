use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::tui::Mode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    MoveDown,
    MoveUp,
    PrevMonth,
    NextMonth,
    OpenPayments,
    OpenStatements,
    OpenSize,
    OpenDebts,
    Edit,
    SetBalance,
    New,
    Help,
    Back,
    Quit,
    Noop,
}

pub fn command_for_key(mode: Mode, key: KeyEvent) -> Command {
    if key.modifiers.contains(KeyModifiers::CONTROL) || key.modifiers.contains(KeyModifiers::ALT) {
        return Command::Noop;
    }

    match mode {
        Mode::Home => home_command(key),
        Mode::Help => help_command(key),
        Mode::Debts { .. } => debts_command(key),
        Mode::Payments { .. } => payments_command(key),
        Mode::Statements { .. } => statements_command(key),
        Mode::Size { .. } => size_command(key),
        Mode::Form(_) => form_command(key),
    }
}

fn home_command(key: KeyEvent) -> Command {
    match key.code {
        KeyCode::Char('j') => Command::MoveDown,
        KeyCode::Char('k') => Command::MoveUp,
        KeyCode::Char('h') => Command::PrevMonth,
        KeyCode::Char('l') => Command::NextMonth,
        KeyCode::Char('p') => Command::OpenPayments,
        KeyCode::Char('s') => Command::OpenStatements,
        KeyCode::Char('n') => Command::OpenSize,
        KeyCode::Char('d') => Command::OpenDebts,
        KeyCode::Char('?') => Command::Help,
        KeyCode::Char('q') => Command::Quit,
        KeyCode::Esc => Command::Noop,
        _ => Command::Noop,
    }
}

fn help_command(key: KeyEvent) -> Command {
    match key.code {
        KeyCode::Char('?') | KeyCode::Esc => Command::Back,
        KeyCode::Char('q') => Command::Quit,
        _ => Command::Noop,
    }
}

fn debts_command(key: KeyEvent) -> Command {
    match key.code {
        KeyCode::Char('j') => Command::MoveDown,
        KeyCode::Char('k') => Command::MoveUp,
        KeyCode::Char('n') => Command::New,
        KeyCode::Char('e') | KeyCode::Enter => Command::Edit,
        KeyCode::Char('b') => Command::SetBalance,
        KeyCode::Char('?') => Command::Help,
        KeyCode::Char('q') => Command::Quit,
        KeyCode::Esc => Command::Back,
        _ => Command::Noop,
    }
}

fn payments_command(key: KeyEvent) -> Command {
    match key.code {
        KeyCode::Char('j') => Command::MoveDown,
        KeyCode::Char('k') => Command::MoveUp,
        KeyCode::Char('n') => Command::New,
        KeyCode::Char('e') | KeyCode::Enter => Command::Edit,
        KeyCode::Char('?') => Command::Help,
        KeyCode::Char('q') => Command::Quit,
        KeyCode::Esc => Command::Back,
        _ => Command::Noop,
    }
}

fn statements_command(key: KeyEvent) -> Command {
    match key.code {
        KeyCode::Char('j') => Command::MoveDown,
        KeyCode::Char('k') => Command::MoveUp,
        KeyCode::Char('n') => Command::New,
        KeyCode::Char('e') | KeyCode::Enter => Command::Edit,
        KeyCode::Char('?') => Command::Help,
        KeyCode::Char('q') => Command::Quit,
        KeyCode::Esc => Command::Back,
        _ => Command::Noop,
    }
}

fn size_command(key: KeyEvent) -> Command {
    match key.code {
        KeyCode::Char('j') => Command::MoveDown,
        KeyCode::Char('k') => Command::MoveUp,
        KeyCode::Char('n') => Command::New,
        KeyCode::Char('?') => Command::Help,
        KeyCode::Char('q') => Command::Quit,
        KeyCode::Esc => Command::Back,
        _ => Command::Noop,
    }
}

fn form_command(key: KeyEvent) -> Command {
    match key.code {
        KeyCode::Esc => Command::Back,
        KeyCode::Enter => Command::Edit,
        _ => Command::Noop,
    }
}
