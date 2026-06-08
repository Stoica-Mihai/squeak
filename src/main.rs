//! squeak — terminal configurator for Keychron mice on Linux.
//! M0: terminal lifecycle + event loop + themed sidebar/footer scaffold.

mod app;
mod event;
mod theme;
mod ui;

use std::time::Duration;

use anyhow::Result;
use ratatui::crossterm::event::{self as ct, Event, KeyEventKind};

use crate::app::App;
use crate::event::map_key;

/// Poll interval; also the cadence for the future periodic battery refresh (M1).
const TICK: Duration = Duration::from_millis(250);

fn main() -> Result<()> {
    let mut terminal = ratatui::init();
    let result = run(&mut terminal);
    ratatui::restore();
    result
}

fn run(terminal: &mut ratatui::DefaultTerminal) -> Result<()> {
    let mut app = App::default();
    while app.running {
        terminal.draw(|f| ui::render(f, &app))?;
        if ct::poll(TICK)?
            && let Event::Key(key) = ct::read()?
            && key.kind == KeyEventKind::Press
        {
            app.update(map_key(key));
        }
    }
    Ok(())
}
