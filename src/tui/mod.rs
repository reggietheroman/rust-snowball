mod debts;
mod home;
mod keys;
mod money;
mod overlays;

pub use money::{format_pesos, parse_pesos};

use crossterm::event::{self, Event, KeyEvent};
use ratatui::backend::CrosstermBackend;
use ratatui::{Frame, Terminal};

use crate::db::SqliteDb;
use crate::debts::{BorrowedDebtStore, DebtId, DebtKind, DebtStore};
use crate::error::Error;
use crate::plan::{Plan, compute_plan};
use crate::plan::{next_month, previous_month};
use crate::snowball_size::SqliteSnowballSizeStore;
use crate::statements::SqliteStatementStore;
use crate::tui::keys::{Command, command_for_key};
use crate::tui::overlays::{draw_help, draw_mode};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    Home,
    Help,
    Debts { selected: usize },
    Payments { debt_id: DebtId, selected: usize },
    Statements { debt_id: DebtId, selected: usize },
    Size { selected: usize },
    Form(crate::tui::overlays::Form),
}

pub struct App {
    db: SqliteDb,
    today: String,
    payment_month: String,
    pub mode: Mode,
    pub status: String,
    pub quit: bool,
    pub plan: Option<Plan>,
    pub selected: usize,
}

impl App {
    pub fn new(db: SqliteDb, today: String) -> Result<Self, Error> {
        if today.len() != 10 || today.as_bytes().get(4) != Some(&b'-') {
            return Err(Error::validation("today must be YYYY-MM-DD"));
        }
        let payment_month = today[..7].to_string();
        let mut app = Self {
            db,
            today,
            payment_month,
            mode: Mode::Home,
            status: String::new(),
            quit: false,
            plan: None,
            selected: 0,
        };
        app.refresh_plan()?;
        Ok(app)
    }

    pub fn db(&self) -> &SqliteDb {
        &self.db
    }

    pub fn today(&self) -> &str {
        &self.today
    }

    pub fn payment_month(&self) -> &str {
        &self.payment_month
    }

    pub fn refresh_plan(&mut self) -> Result<(), Error> {
        let debts = BorrowedDebtStore::new(&self.db);
        let sizes = SqliteSnowballSizeStore::new(&self.db);
        let statements = SqliteStatementStore::new(&self.db);
        let plan = compute_plan(&self.payment_month, &debts, &sizes, &statements)?;
        self.plan = Some(plan);
        self.clamp_selection();
        Ok(())
    }

    fn clamp_selection(&mut self) {
        let len = self.plan.as_ref().map(|plan| plan.lines.len()).unwrap_or(0);
        if len == 0 {
            self.selected = 0;
        } else if self.selected >= len {
            self.selected = len - 1;
        }
    }

    pub fn selected_line(&self) -> Option<&crate::plan::PlanLine> {
        self.plan
            .as_ref()
            .and_then(|plan| plan.lines.get(self.selected))
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        if matches!(self.mode, Mode::Form(_)) && overlays::handle_form_key(self, key) {
            return;
        }

        let command = command_for_key(self.mode.clone(), key);
        if let Err(err) = self.apply_command(command) {
            self.status = err.message;
        }
    }

    fn apply_command(&mut self, command: Command) -> Result<(), Error> {
        match command {
            Command::Noop => {}
            Command::Quit => self.quit = true,
            Command::Back => self.back(),
            Command::Help => self.mode = Mode::Help,
            Command::MoveDown => self.move_down(),
            Command::MoveUp => self.move_up(),
            Command::PrevMonth => self.shift_month(previous_month(&self.payment_month)?)?,
            Command::NextMonth => self.shift_month(next_month(&self.payment_month)?)?,
            Command::OpenDebts => {
                self.mode = Mode::Debts { selected: 0 };
                self.status.clear();
            }
            Command::OpenSize => {
                if matches!(self.mode, Mode::Home) {
                    self.mode = Mode::Size { selected: 0 };
                    self.status.clear();
                }
            }
            Command::OpenPayments => self.open_payments()?,
            Command::OpenStatements => self.open_statements()?,
            Command::New | Command::Edit | Command::SetBalance => {
                overlays::handle_write_command(self, command)?;
            }
        }
        Ok(())
    }

    fn back(&mut self) {
        self.mode = match &self.mode {
            Mode::Form(form) => form.parent_mode(),
            Mode::Help => Mode::Home,
            Mode::Debts { .. }
            | Mode::Payments { .. }
            | Mode::Statements { .. }
            | Mode::Size { .. } => {
                self.refresh_plan().ok();
                Mode::Home
            }
            Mode::Home => Mode::Home,
        };
        self.status.clear();
    }

    fn move_down(&mut self) {
        if matches!(self.mode, Mode::Debts { .. }) {
            let len = debts::debt_count(self).unwrap_or(0);
            if let Mode::Debts { selected } = &mut self.mode
                && len > 0
                && *selected + 1 < len
            {
                *selected += 1;
            }
            return;
        }
        if matches!(
            self.mode,
            Mode::Payments { .. } | Mode::Statements { .. } | Mode::Size { .. }
        ) {
            let len = overlays::overlay_len(self).unwrap_or(0);
            if let Mode::Payments { selected, .. }
            | Mode::Statements { selected, .. }
            | Mode::Size { selected } = &mut self.mode
                && len > 0
                && *selected + 1 < len
            {
                *selected += 1;
            }
            return;
        }

        match &mut self.mode {
            Mode::Home => {
                let len = self.plan.as_ref().map(|p| p.lines.len()).unwrap_or(0);
                if len > 0 && self.selected + 1 < len {
                    self.selected += 1;
                }
            }
            Mode::Help
            | Mode::Form(_)
            | Mode::Debts { .. }
            | Mode::Payments { .. }
            | Mode::Statements { .. }
            | Mode::Size { .. } => {}
        }
    }

    fn move_up(&mut self) {
        match &mut self.mode {
            Mode::Home => {
                if self.selected > 0 {
                    self.selected -= 1;
                }
            }
            Mode::Debts { selected } => {
                if *selected > 0 {
                    *selected -= 1;
                }
            }
            Mode::Payments { selected, .. }
            | Mode::Statements { selected, .. }
            | Mode::Size { selected } => {
                if *selected > 0 {
                    *selected -= 1;
                }
            }
            Mode::Help | Mode::Form(_) => {}
        }
    }

    fn shift_month(&mut self, payment_month: String) -> Result<(), Error> {
        self.payment_month = payment_month;
        self.refresh_plan()?;
        self.status.clear();
        Ok(())
    }

    fn open_payments(&mut self) -> Result<(), Error> {
        let debt_id = match self.selected_line() {
            Some(line) => line.debt_id.clone(),
            None => {
                self.status = "Select a debt first".into();
                return Ok(());
            }
        };
        self.mode = Mode::Payments {
            debt_id,
            selected: 0,
        };
        self.status.clear();
        Ok(())
    }

    fn open_statements(&mut self) -> Result<(), Error> {
        let line = match self.selected_line() {
            Some(line) => line,
            None => {
                self.status = "Select a debt first".into();
                return Ok(());
            }
        };
        let debt = BorrowedDebtStore::new(&self.db).get(&line.debt_id)?;
        if !matches!(debt.kind, DebtKind::CreditCard) {
            self.status = "Statements are for credit cards only".into();
            return Ok(());
        }
        self.mode = Mode::Statements {
            debt_id: line.debt_id.clone(),
            selected: 0,
        };
        self.status.clear();
        Ok(())
    }
}

pub fn draw(app: &App, frame: &mut Frame) {
    match &app.mode {
        Mode::Home => home::draw(app, frame),
        Mode::Help => draw_help(frame),
        other => draw_mode(app, frame, other),
    }
}

pub fn run(db: SqliteDb) -> Result<(), Error> {
    let today = local_today()?;
    let mut app = App::new(db, today)?;
    let mut stdout = std::io::stdout();
    map_io(crossterm::terminal::enable_raw_mode())?;
    map_io(crossterm::execute!(
        stdout,
        crossterm::terminal::EnterAlternateScreen
    ))?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).map_err(map_io_error)?;
    let result = run_loop(&mut terminal, &mut app);
    let _ = crossterm::terminal::disable_raw_mode();
    let _ = crossterm::execute!(
        terminal.backend_mut(),
        crossterm::terminal::LeaveAlternateScreen
    );
    let _ = terminal.show_cursor();
    result
}

fn map_io<T>(result: std::io::Result<T>) -> Result<T, Error> {
    result.map_err(map_io_error)
}

fn map_io_error(err: std::io::Error) -> Error {
    Error::validation(err.to_string())
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
) -> Result<(), Error> {
    while !app.quit {
        map_io(terminal.draw(|frame| draw(app, frame)))?;
        if map_io(event::poll(std::time::Duration::from_millis(100)))?
            && let Event::Key(key) = map_io(event::read())?
        {
            app.handle_key(key);
        }
    }
    Ok(())
}

pub fn local_today() -> Result<String, Error> {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| Error::validation("system clock is before unix epoch"))?
        .as_secs();
    // Approximate local date using UTC for the binary path; tests inject today.
    let days = secs / 86_400;
    Ok(epoch_day_to_ymd(days))
}

fn epoch_day_to_ymd(days: u64) -> String {
    // Civil date from days since 1970-01-01 (proleptic Gregorian).
    let z = days as i64 + 719_468;
    let era = if z >= 0 { z } else { z - 1_461_571 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    let year = y + if m <= 2 { 1 } else { 0 };
    format!("{year:04}-{m:02}-{d:02}")
}
