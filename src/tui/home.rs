use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table};

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
    let widths = [
        Constraint::Length(2),
        Constraint::Min(12),
        Constraint::Length(14),
        Constraint::Length(14),
        Constraint::Length(14),
        Constraint::Length(14),
        Constraint::Length(14),
    ];

    let header_style = Style::default().add_modifier(Modifier::BOLD);
    let header = Row::new(vec![
        Cell::from(""),
        Cell::from("Name"),
        Cell::from(Line::from("Remaining").alignment(Alignment::Right)),
        Cell::from(Line::from("Required").alignment(Alignment::Right)),
        Cell::from(Line::from("Extra").alignment(Alignment::Right)),
        Cell::from(Line::from("Send").alignment(Alignment::Right)),
        Cell::from(Line::from("Due").alignment(Alignment::Right)),
    ])
    .style(header_style)
    .bottom_margin(1);

    let rows: Vec<Row> = plan
        .lines
        .iter()
        .enumerate()
        .map(|(index, line)| {
            let due = line
                .due_on
                .as_deref()
                .map(|d| d.to_string())
                .unwrap_or_else(|| {
                    if line.missing_statement {
                        "?".into()
                    } else {
                        "—".into()
                    }
                });

            let style = if index == app.selected {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };

            Row::new(vec![
                Cell::from(line_mark(app, line)),
                Cell::from(line.name.clone()),
                peso_cell(line.remaining_cents),
                peso_cell(line.required_cents),
                peso_cell(line.extra_cents),
                peso_cell(line.send_cents),
                Cell::from(Line::from(due).alignment(Alignment::Right)),
            ])
            .style(style)
        })
        .collect();

    let table = Table::new(rows, widths)
        .column_spacing(1)
        .header(header)
        .block(Block::default().borders(Borders::ALL).title("This month"));

    frame.render_widget(table, area);
}

fn line_mark(app: &App, line: &crate::plan::PlanLine) -> &'static str {
    if line.missing_statement {
        return "?";
    }
    if let Some(due_on) = &line.due_on
        && due_on.as_str() < app.today.as_str()
    {
        return "!";
    }
    ""
}

fn peso_cell(cents: i64) -> Cell<'static> {
    Cell::from(Line::from(format_pesos(cents)).alignment(Alignment::Right))
}
