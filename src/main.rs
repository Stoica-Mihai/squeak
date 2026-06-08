//! squeak — terminal configurator for Keychron mice on Linux.
//! M1: hidraw transport + 0x06 block read on a worker thread + live Overview.

mod app;
mod event;
mod hid;
mod proto;
mod theme;
mod ui;
mod worker;

use std::time::{Duration, Instant};

use anyhow::Result;
use ratatui::crossterm::event::{self as ct, Event, KeyEventKind};

use crate::app::App;
use crate::event::map_key;
use crate::worker::Worker;

/// Event-poll interval (UI responsiveness floor).
const TICK: Duration = Duration::from_millis(100);
/// Periodic snapshot refresh (battery etc.).
const AUTO_REFRESH: Duration = Duration::from_secs(5);

fn main() -> Result<()> {
    let mut terminal = ratatui::init();
    let result = run(&mut terminal);
    ratatui::restore();
    result
}

fn run(terminal: &mut ratatui::DefaultTerminal) -> Result<()> {
    let worker = Worker::spawn();
    let mut app = App::new(worker.cmd_tx.clone());
    app.request_read(); // initial snapshot

    let mut last_auto = Instant::now();
    while app.running {
        terminal.draw(|f| ui::render(f, &app))?;

        if ct::poll(TICK)?
            && let Event::Key(key) = ct::read()?
            && key.kind == KeyEventKind::Press
        {
            app.update(map_key(key));
        }

        while let Ok(update) = worker.update_rx.try_recv() {
            app.apply(update);
        }

        if last_auto.elapsed() >= AUTO_REFRESH {
            app.request_read();
            last_auto = Instant::now();
        }
    }
    Ok(()) // Worker's Drop sends Shutdown and joins
}
