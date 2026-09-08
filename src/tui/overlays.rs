use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

use crate::debts::{BorrowedDebtStore, Debt, DebtId, DebtKind, DebtStore};
use crate::payments::{
    Payment, PaymentId, PaymentStore, RecordPayment, SqlitePaymentStore, UpdatePayment,
};
use crate::plan::previous_month;
use crate::snowball_size::{RecordSize, SnowballSizeStore, SqliteSnowballSizeStore};
use crate::statements::{RecordStatement, SqliteStatementStore, StatementStore};
use crate::tui::debts;
use crate::tui::keys::Command;
use crate::tui::money::{format_pesos, parse_pesos};
use crate::tui::{App, Mode};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormKind {
    CreateDebt,
    EditDebt,
    SetBalance,
    RecordPayment,
    EditPayment,
    RecordStatement,
    RecordSize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormField {
    pub label: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParentMode {
    Home,
    Debts { selected: usize },
    Payments { debt_id: DebtId, selected: usize },
    Statements { debt_id: DebtId, selected: usize },
    Size { selected: usize },
}

impl ParentMode {
    pub fn into_mode(self) -> Mode {
        match self {
            ParentMode::Home => Mode::Home,
            ParentMode::Debts { selected } => Mode::Debts { selected },
            ParentMode::Payments { debt_id, selected } => Mode::Payments { debt_id, selected },
            ParentMode::Statements { debt_id, selected } => Mode::Statements { debt_id, selected },
            ParentMode::Size { selected } => Mode::Size { selected },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Form {
    pub kind: FormKind,
    pub fields: Vec<FormField>,
    pub focused: usize,
    pub parent: ParentMode,
    pub debt_id: Option<DebtId>,
    pub payment_id: Option<PaymentId>,
    pub debt_picker_index: usize,
}

impl Form {
    pub fn create_debt() -> Self {
        Self {
            kind: FormKind::CreateDebt,
            fields: vec![
                FormField {
                    label: "Kind (loan/card)".into(),
                    value: "loan".into(),
                },
                FormField {
                    label: "Name".into(),
                    value: String::new(),
                },
                FormField {
                    label: "Balance".into(),
                    value: String::new(),
                },
                FormField {
                    label: "Monthly payment".into(),
                    value: String::new(),
                },
                FormField {
                    label: "Due day".into(),
                    value: String::new(),
                },
            ],
            focused: 1,
            parent: ParentMode::Debts { selected: 0 },
            debt_id: None,
            payment_id: None,
            debt_picker_index: 0,
        }
    }

    pub fn edit_debt(debt: Debt) -> Self {
        let fields = match debt.kind {
            DebtKind::Loan {
                payment_cents,
                due_day,
            } => vec![
                FormField {
                    label: "Name".into(),
                    value: debt.name,
                },
                FormField {
                    label: "Monthly payment".into(),
                    value: format_pesos(payment_cents),
                },
                FormField {
                    label: "Due day".into(),
                    value: due_day.to_string(),
                },
            ],
            DebtKind::CreditCard => vec![FormField {
                label: "Name".into(),
                value: debt.name,
            }],
        };
        Self {
            kind: FormKind::EditDebt,
            fields,
            focused: 0,
            parent: ParentMode::Debts { selected: 0 },
            debt_id: Some(debt.id),
            payment_id: None,
            debt_picker_index: 0,
        }
    }

    pub fn set_balance(debt: Debt) -> Self {
        Self {
            kind: FormKind::SetBalance,
            fields: vec![FormField {
                label: "Balance".into(),
                value: format_pesos(debt.balance_cents),
            }],
            focused: 0,
            parent: ParentMode::Debts { selected: 0 },
            debt_id: Some(debt.id),
            payment_id: None,
            debt_picker_index: 0,
        }
    }

    pub fn record_payment(debt_id: DebtId, today: &str) -> Self {
        Self {
            kind: FormKind::RecordPayment,
            fields: vec![
                FormField {
                    label: "Amount".into(),
                    value: String::new(),
                },
                FormField {
                    label: "Paid on".into(),
                    value: today.to_string(),
                },
            ],
            focused: 0,
            parent: ParentMode::Payments {
                debt_id: debt_id.clone(),
                selected: 0,
            },
            debt_id: Some(debt_id),
            payment_id: None,
            debt_picker_index: 0,
        }
    }

    pub fn edit_payment(payment: Payment, debts: &[Debt], debt_picker_index: usize) -> Self {
        let debt_name = debts
            .get(debt_picker_index)
            .map(|d| d.name.clone())
            .unwrap_or_else(|| payment.debt_id.to_string());
        Self {
            kind: FormKind::EditPayment,
            fields: vec![
                FormField {
                    label: "Amount".into(),
                    value: format_pesos(payment.amount_cents),
                },
                FormField {
                    label: "Paid on".into(),
                    value: payment.paid_on.clone(),
                },
                FormField {
                    label: "Debt".into(),
                    value: debt_name,
                },
            ],
            focused: 0,
            parent: ParentMode::Payments {
                debt_id: payment.debt_id.clone(),
                selected: 0,
            },
            debt_id: Some(payment.debt_id.clone()),
            payment_id: Some(payment.id),
            debt_picker_index,
        }
    }

    pub fn record_statement(debt_id: DebtId, payment_month: &str) -> Self {
        let statement_month =
            previous_month(payment_month).unwrap_or_else(|_| payment_month.to_string());
        Self {
            kind: FormKind::RecordStatement,
            fields: vec![
                FormField {
                    label: "Cycle month (YYYY-MM)".into(),
                    value: statement_month,
                },
                FormField {
                    label: "Minimum".into(),
                    value: String::new(),
                },
                FormField {
                    label: "Due on".into(),
                    value: String::new(),
                },
            ],
            focused: 0,
            parent: ParentMode::Statements {
                debt_id: debt_id.clone(),
                selected: 0,
            },
            debt_id: Some(debt_id),
            payment_id: None,
            debt_picker_index: 0,
        }
    }

    pub fn record_size() -> Self {
        Self {
            kind: FormKind::RecordSize,
            fields: vec![FormField {
                label: "Amount".into(),
                value: String::new(),
            }],
            focused: 0,
            parent: ParentMode::Size { selected: 0 },
            debt_id: None,
            payment_id: None,
            debt_picker_index: 0,
        }
    }

    pub fn parent_mode(&self) -> Mode {
        self.parent.clone().into_mode()
    }
}

pub fn draw_help(frame: &mut Frame) {
    let text = [
        "j/k move   h/l month (home)   p payments   s statements (card)",
        "n size (home) or new (overlay)   d debts   ? help   q quit   Esc back",
    ];
    let block = Block::default().borders(Borders::ALL).title("Help");
    frame.render_widget(Paragraph::new(text.join("\n")).block(block), frame.area());
}

pub fn draw_mode(app: &App, frame: &mut Frame, mode: &Mode) {
    match mode {
        Mode::Debts { selected } => debts::draw(app, frame, *selected),
        Mode::Payments { debt_id, selected } => draw_payments(app, frame, debt_id, *selected),
        Mode::Statements { debt_id, selected } => draw_statements(app, frame, debt_id, *selected),
        Mode::Size { selected } => draw_size(app, frame, *selected),
        Mode::Form(form) => draw_form(app, frame, form),
        Mode::Home | Mode::Help => {}
    }
}

pub fn overlay_len(app: &App) -> Result<usize, crate::error::Error> {
    match &app.mode {
        Mode::Payments { debt_id, .. } => Ok(SqlitePaymentStore::new(app.db())
            .list_for_debt(debt_id)?
            .len()),
        Mode::Statements { debt_id, .. } => Ok(SqliteStatementStore::new(app.db())
            .list_for_debt(debt_id)?
            .len()),
        Mode::Size { .. } => Ok(SqliteSnowballSizeStore::new(app.db()).list()?.len()),
        _ => Ok(0),
    }
}

pub fn handle_write_command(app: &mut App, command: Command) -> Result<(), crate::error::Error> {
    match (&app.mode, command) {
        (Mode::Debts { selected: _ }, Command::New) => {
            debts::open_create_form(app);
        }
        (Mode::Debts { selected }, Command::Edit) => {
            debts::open_edit_form(app, *selected)?;
        }
        (Mode::Debts { selected }, Command::SetBalance) => {
            debts::open_set_balance_form(app, *selected)?;
        }
        (Mode::Payments { debt_id, .. }, Command::New) => {
            app.mode = Mode::Form(Form::record_payment(debt_id.clone(), app.today()));
        }
        (Mode::Payments { debt_id, selected }, Command::Edit) => {
            let payments = SqlitePaymentStore::new(app.db()).list_for_debt(debt_id)?;
            let payment = payments
                .get(*selected)
                .ok_or_else(|| crate::error::Error::validation("select a payment"))?;
            let debts = BorrowedDebtStore::new(app.db()).list()?;
            let debt_picker_index = debts
                .iter()
                .position(|d| d.id == payment.debt_id)
                .unwrap_or(0);
            app.mode = Mode::Form(Form::edit_payment(
                payment.clone(),
                &debts,
                debt_picker_index,
            ));
        }
        (Mode::Statements { debt_id, .. }, Command::New | Command::Edit) => {
            app.mode = Mode::Form(Form::record_statement(debt_id.clone(), app.payment_month()));
        }
        (Mode::Size { .. }, Command::New) => {
            app.mode = Mode::Form(Form::record_size());
        }
        _ => {}
    }
    Ok(())
}

#[allow(clippy::needless_return)]
pub fn handle_form_key(app: &mut App, key: KeyEvent) -> bool {
    let Mode::Form(mut form) = app.mode.clone() else {
        return false;
    };

    if key.modifiers.contains(KeyModifiers::CONTROL) || key.modifiers.contains(KeyModifiers::ALT) {
        return true;
    }

    match key.code {
        KeyCode::Esc => {
            app.mode = form.parent_mode();
            app.status.clear();
            return true;
        }
        KeyCode::Tab => {
            form.focused = (form.focused + 1) % form.fields.len();
            app.mode = Mode::Form(form);
            return true;
        }
        KeyCode::BackTab => {
            form.focused = if form.focused == 0 {
                form.fields.len() - 1
            } else {
                form.focused - 1
            };
            app.mode = Mode::Form(form);
            return true;
        }
        KeyCode::Enter => {
            if let Err(err) = submit_form(app, &form) {
                app.status = err.message;
                app.mode = Mode::Form(form);
            } else {
                app.mode = form.parent_mode();
                app.refresh_plan().ok();
                app.status.clear();
            }
            return true;
        }
        KeyCode::Char('h') if is_debt_picker(&form) => {
            let debts = BorrowedDebtStore::new(app.db()).list().unwrap_or_default();
            if !debts.is_empty() {
                form.debt_picker_index = if form.debt_picker_index == 0 {
                    debts.len() - 1
                } else {
                    form.debt_picker_index - 1
                };
                form.fields[2].value = debts[form.debt_picker_index].name.clone();
                form.debt_id = Some(debts[form.debt_picker_index].id.clone());
            }
            app.mode = Mode::Form(form);
            return true;
        }
        KeyCode::Char('l') if is_debt_picker(&form) => {
            let debts = BorrowedDebtStore::new(app.db()).list().unwrap_or_default();
            if !debts.is_empty() {
                form.debt_picker_index = (form.debt_picker_index + 1) % debts.len();
                form.fields[2].value = debts[form.debt_picker_index].name.clone();
                form.debt_id = Some(debts[form.debt_picker_index].id.clone());
            }
            app.mode = Mode::Form(form);
            return true;
        }
        KeyCode::Char(ch) if ch == 'q' && form.kind != FormKind::CreateDebt => {
            form.fields[form.focused].value.push(ch);
            app.mode = Mode::Form(form);
            return true;
        }
        KeyCode::Char(ch) => {
            if form.kind == FormKind::CreateDebt && form.focused == 0 {
                if ch == 'l' || ch == 'c' {
                    form.fields[0].value = if ch == 'l' {
                        "loan".into()
                    } else {
                        "card".into()
                    };
                }
            } else {
                form.fields[form.focused].value.push(ch);
            }
            app.mode = Mode::Form(form);
            return true;
        }
        KeyCode::Backspace => {
            form.fields[form.focused].value.pop();
            app.mode = Mode::Form(form);
            return true;
        }
        _ => return true,
    }
}

fn is_debt_picker(form: &Form) -> bool {
    form.kind == FormKind::EditPayment && form.focused == 2
}

fn submit_form(app: &App, form: &Form) -> Result<(), crate::error::Error> {
    match form.kind {
        FormKind::CreateDebt => debts::submit_create(form, app),
        FormKind::EditDebt => debts::submit_edit(form, app),
        FormKind::SetBalance => debts::submit_set_balance(form, app),
        FormKind::RecordPayment => submit_payment(form, app, None),
        FormKind::EditPayment => {
            let id = form
                .payment_id
                .clone()
                .ok_or_else(|| crate::error::Error::validation("missing payment"))?;
            submit_payment(form, app, Some(id))
        }
        FormKind::RecordStatement => submit_statement(form, app),
        FormKind::RecordSize => submit_size(form, app),
    }
}

fn submit_payment(
    form: &Form,
    app: &App,
    payment_id: Option<PaymentId>,
) -> Result<(), crate::error::Error> {
    let amount_cents = parse_pesos(&form.fields[0].value)?;
    let paid_on = form.fields[1].value.trim().to_string();
    let debt_id = form
        .debt_id
        .clone()
        .ok_or_else(|| crate::error::Error::validation("missing debt"))?;
    let store = SqlitePaymentStore::new(app.db());
    if let Some(id) = payment_id {
        store.update(
            &id,
            UpdatePayment {
                debt_id: Some(debt_id),
                amount_cents: Some(amount_cents),
                paid_on: Some(paid_on),
            },
        )?;
    } else {
        store.record(RecordPayment {
            debt_id,
            amount_cents,
            paid_on,
        })?;
    }
    Ok(())
}

fn submit_statement(form: &Form, app: &App) -> Result<(), crate::error::Error> {
    let debt_id = form
        .debt_id
        .clone()
        .ok_or_else(|| crate::error::Error::validation("missing debt"))?;
    SqliteStatementStore::new(app.db()).record(RecordStatement {
        debt_id,
        statement_month: form.fields[0].value.trim().to_string(),
        minimum_cents: parse_pesos(&form.fields[1].value)?,
        due_on: form.fields[2].value.trim().to_string(),
    })?;
    Ok(())
}

fn submit_size(form: &Form, app: &App) -> Result<(), crate::error::Error> {
    SqliteSnowballSizeStore::new(app.db()).record(RecordSize {
        amount_cents: parse_pesos(&form.fields[0].value)?,
    })?;
    Ok(())
}

fn draw_payments(app: &App, frame: &mut Frame, debt_id: &DebtId, selected: usize) {
    let payments = SqlitePaymentStore::new(app.db())
        .list_for_debt(debt_id)
        .unwrap_or_default();
    let items: Vec<ListItem> = payments
        .iter()
        .enumerate()
        .map(|(index, payment)| {
            let flag = if payment.is_overpayment() { " !" } else { "" };
            let text = format!(
                "{amount}{flag}  {paid_on}",
                amount = format_pesos(payment.amount_cents),
                flag = flag,
                paid_on = payment.paid_on,
            );
            let style = if index == selected {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };
            ListItem::new(text).style(style)
        })
        .collect();
    render_overlay_list(frame, "Payments — n new  e edit  Esc home", items, app);
}

fn draw_statements(app: &App, frame: &mut Frame, debt_id: &DebtId, selected: usize) {
    let statements = SqliteStatementStore::new(app.db())
        .list_for_debt(debt_id)
        .unwrap_or_default();
    let items: Vec<ListItem> = statements
        .iter()
        .enumerate()
        .map(|(index, statement)| {
            let overdue = if statement.due_on.as_str() < app.today() {
                " !"
            } else {
                ""
            };
            let text = format!(
                "{month}  min {min}  due {due}{overdue}",
                month = statement.statement_month,
                min = format_pesos(statement.minimum_cents),
                due = statement.due_on,
                overdue = overdue,
            );
            let style = if index == selected {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };
            ListItem::new(text).style(style)
        })
        .collect();
    render_overlay_list(frame, "Statements — n record  e edit  Esc home", items, app);
}

fn draw_size(app: &App, frame: &mut Frame, selected: usize) {
    let sizes = SqliteSnowballSizeStore::new(app.db())
        .list()
        .unwrap_or_default();
    let items: Vec<ListItem> = sizes
        .iter()
        .rev()
        .enumerate()
        .map(|(index, size)| {
            let text = format_pesos(size.amount_cents);
            let style = if index == selected {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };
            ListItem::new(text).style(style)
        })
        .collect();
    render_overlay_list(frame, "Snowball size — n record  Esc home", items, app);
}

fn draw_form(_app: &App, frame: &mut Frame, form: &Form) {
    let lines: Vec<String> = form
        .fields
        .iter()
        .enumerate()
        .map(|(index, field)| {
            let marker = if index == form.focused { ">" } else { " " };
            format!(
                "{marker} {label}: {value}",
                label = field.label,
                value = field.value
            )
        })
        .collect();
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Form — Enter save  Esc cancel  Tab next");
    frame.render_widget(Paragraph::new(lines.join("\n")).block(block), frame.area());
}

fn render_overlay_list(frame: &mut Frame, title: &str, items: Vec<ListItem>, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(1)])
        .split(frame.area());
    let block = Block::default().borders(Borders::ALL).title(title);
    frame.render_widget(List::new(items).block(block), chunks[0]);
    frame.render_widget(Paragraph::new(app.status.as_str()), chunks[1]);
}
