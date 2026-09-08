use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

use crate::plan::Plan;
use crate::tui::App;
use crate::tui::money::format_pesos;

pub fn draw(app: &App, frame: &mut Frame) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7),
            Constraint::Min(3),
            Constraint::Length(1),
        ])
        .split(frame.area());

    draw_header(app, frame, chunks[0]);

    if let Some(plan) = &app.plan {
        draw_lines(app, frame, chunks[1], plan);
    } else {
        let block = Block::default().borders(Borders::ALL).title("Plan");
        frame.render_widget(Paragraph::new("Loading…").block(block), chunks[1]);
    }

    frame.render_widget(
        Paragraph::new(app.status.as_str()).style(Style::default().add_modifier(Modifier::DIM)),
        chunks[2],
    );
}

fn draw_header(app: &App, frame: &mut Frame, area: Rect) {
    let mut lines = vec![Line::from(vec![
        Span::raw("Payment month: "),
        Span::styled(
            app.payment_month.clone(),
            Style::default().add_modifier(Modifier::BOLD),
        ),
    ])];

    if let Some(plan) = &app.plan {
        match plan.snowball_amount_cents {
            Some(cents) => {
                lines.push(Line::from(format!("Snowball: {}", format_pesos(cents))));
            }
            None => lines.push(Line::from("Snowball: no size recorded")),
        }

        if let Some(shortfall) = plan.shortfall_cents
            && shortfall > 0
        {
            lines.push(Line::from(format!(
                "Shortfall: {}",
                format_pesos(shortfall)
            )));
        }

        if plan.unallocated_cents > 0 {
            lines.push(Line::from(format!(
                "Unallocated: {}",
                format_pesos(plan.unallocated_cents)
            )));
        }
    } else {
        lines.push(Line::from("Snowball: no size recorded"));
    }

    if app.plan.as_ref().is_some_and(|plan| plan.lines.is_empty()) {
        lines.push(Line::from("No debts yet — press d to add one"));
    }

    let block = Block::default().borders(Borders::ALL).title("Snowball");
    frame.render_widget(Paragraph::new(lines).block(block), area);
}

fn draw_lines(app: &App, frame: &mut Frame, area: Rect, plan: &Plan) {
    let items: Vec<ListItem> = plan
        .lines
        .iter()
        .enumerate()
        .map(|(index, line)| {
            let mark = line_mark(app, line);
            let due = line
                .due_on
                .as_deref()
                .map(|d| d.to_string())
                .unwrap_or_else(|| if line.missing_statement { "?".into() } else { "—".into() });

            let text = format!(
                "{mark}{name:<16} rem {remaining} req {required} extra {extra} send {send} due {due}",
                mark = mark,
                name = truncate_name(&line.name, 16),
                remaining = format_pesos(line.remaining_cents),
                required = format_pesos(line.required_cents),
                extra = format_pesos(line.extra_cents),
                send = format_pesos(line.send_cents),
                due = due,
            );

            let style = if index == app.selected {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };
            ListItem::new(text).style(style)
        })
        .collect();

    let block = Block::default().borders(Borders::ALL).title("This month");
    let list = List::new(items).block(block);
    frame.render_widget(list, area);
}

fn line_mark(app: &App, line: &crate::plan::PlanLine) -> &'static str {
    if line.missing_statement {
        return "? ";
    }
    if let Some(due_on) = &line.due_on
        && due_on.as_str() < app.today.as_str()
    {
        return "! ";
    }
    "  "
}

fn truncate_name(name: &str, max: usize) -> String {
    if name.len() <= max {
        name.to_string()
    } else {
        format!(
            "{}…",
            name.chars().take(max.saturating_sub(1)).collect::<String>()
        )
    }
}
