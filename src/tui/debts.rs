use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

use crate::debts::{
    BorrowedDebtStore, CreateDebt, CreateDebtKind, DebtKind, DebtStore, UpdateDebt,
};
use crate::tui::money::{format_pesos, parse_pesos};
use crate::tui::{App, Mode};

pub fn debt_count(app: &App) -> Result<usize, crate::error::Error> {
    Ok(BorrowedDebtStore::new(app.db()).list()?.len())
}

pub fn draw(app: &App, frame: &mut Frame, selected: usize) {
    let debts = BorrowedDebtStore::new(app.db()).list().unwrap_or_default();
    let items: Vec<ListItem> = debts
        .iter()
        .enumerate()
        .map(|(index, debt)| {
            let kind = match debt.kind {
                DebtKind::CreditCard => "card",
                DebtKind::Loan { .. } => "loan",
            };
            let text = format!(
                "{kind:<5} {name:<20} {balance}",
                kind = kind,
                name = debt.name,
                balance = format_pesos(debt.balance_cents),
            );
            let style = if index == selected {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };
            ListItem::new(text).style(style)
        })
        .collect();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(1)])
        .split(frame.area());

    let block = Block::default()
        .borders(Borders::ALL)
        .title("Debts — n new  e edit  b balance  Esc home");
    frame.render_widget(List::new(items).block(block), chunks[0]);
    frame.render_widget(Paragraph::new(app.status.as_str()), chunks[1]);
}

pub fn selected_debt_id(app: &App, selected: usize) -> Option<crate::debts::DebtId> {
    BorrowedDebtStore::new(app.db())
        .list()
        .ok()
        .and_then(|debts| debts.get(selected).map(|debt| debt.id.clone()))
}

pub fn open_create_form(app: &mut App) {
    app.mode = Mode::Form(crate::tui::overlays::Form::create_debt());
}

pub fn open_edit_form(app: &mut App, selected: usize) -> Result<(), crate::error::Error> {
    let debt_id = selected_debt_id(app, selected)
        .ok_or_else(|| crate::error::Error::validation("select a debt"))?;
    let debt = BorrowedDebtStore::new(app.db()).get(&debt_id)?;
    app.mode = Mode::Form(crate::tui::overlays::Form::edit_debt(debt));
    Ok(())
}

pub fn open_set_balance_form(app: &mut App, selected: usize) -> Result<(), crate::error::Error> {
    let debt_id = selected_debt_id(app, selected)
        .ok_or_else(|| crate::error::Error::validation("select a debt"))?;
    let debt = BorrowedDebtStore::new(app.db()).get(&debt_id)?;
    app.mode = Mode::Form(crate::tui::overlays::Form::set_balance(debt));
    Ok(())
}

pub fn submit_create(
    form: &crate::tui::overlays::Form,
    app: &App,
) -> Result<(), crate::error::Error> {
    let fields = &form.fields;
    let name = fields[1].value.trim();
    if name.is_empty() {
        return Err(crate::error::Error::validation("name is required"));
    }
    let balance_cents = parse_pesos(&fields[2].value)?;
    let kind = if fields[0].value == "loan" {
        let payment_cents = parse_pesos(&fields[3].value)?;
        let due_day = fields[4]
            .value
            .parse()
            .map_err(|_| crate::error::Error::validation("due day must be 1-31"))?;
        CreateDebtKind::Loan {
            payment_cents,
            due_day,
        }
    } else {
        CreateDebtKind::CreditCard
    };
    BorrowedDebtStore::new(app.db()).create(CreateDebt {
        name: name.into(),
        balance_cents,
        kind,
    })?;
    Ok(())
}

pub fn submit_edit(
    form: &crate::tui::overlays::Form,
    app: &App,
) -> Result<(), crate::error::Error> {
    let debt_id = form
        .debt_id
        .clone()
        .ok_or_else(|| crate::error::Error::validation("missing debt"))?;
    let debt = BorrowedDebtStore::new(app.db()).get(&debt_id)?;
    let name = form.fields[0].value.trim();
    if name.is_empty() {
        return Err(crate::error::Error::validation("name is required"));
    }
    let patch = match debt.kind {
        DebtKind::Loan { .. } => UpdateDebt {
            name: Some(name.into()),
            payment_cents: Some(parse_pesos(&form.fields[1].value)?),
            due_day: Some(
                form.fields[2]
                    .value
                    .parse()
                    .map_err(|_| crate::error::Error::validation("due day must be 1-31"))?,
            ),
        },
        DebtKind::CreditCard => UpdateDebt {
            name: Some(name.into()),
            payment_cents: None,
            due_day: None,
        },
    };
    BorrowedDebtStore::new(app.db()).update(&debt_id, patch)?;
    Ok(())
}

pub fn submit_set_balance(
    form: &crate::tui::overlays::Form,
    app: &App,
) -> Result<(), crate::error::Error> {
    let debt_id = form
        .debt_id
        .clone()
        .ok_or_else(|| crate::error::Error::validation("missing debt"))?;
    let balance_cents = parse_pesos(&form.fields[0].value)?;
    BorrowedDebtStore::new(app.db()).set_balance(&debt_id, balance_cents)?;
    Ok(())
}
