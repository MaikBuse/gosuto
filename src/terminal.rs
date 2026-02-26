use std::io::{self, Stdout};

use anyhow::Result;
use crossterm::{
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{Terminal, prelude::CrosstermBackend};

pub type Tui = Terminal<CrosstermBackend<Stdout>>;

pub fn init() -> Result<Tui> {
    let mut stdout = io::stdout();
    terminal::enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen)?;
    crossterm::execute!(stdout, crossterm::event::EnableMouseCapture)?;

    // Enable keyboard enhancement for key release events (needed for PTT)
    // Gracefully ignored if terminal doesn't support it
    let _ = crossterm::execute!(
        stdout,
        crossterm::event::PushKeyboardEnhancementFlags(
            crossterm::event::KeyboardEnhancementFlags::REPORT_EVENT_TYPES
        )
    );

    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

pub fn restore() -> Result<()> {
    let mut stdout = io::stdout();

    // Pop keyboard enhancement (gracefully ignored if not supported)
    let _ = crossterm::execute!(stdout, crossterm::event::PopKeyboardEnhancementFlags);

    crossterm::execute!(stdout, crossterm::event::DisableMouseCapture)?;
    execute!(stdout, LeaveAlternateScreen)?;
    terminal::disable_raw_mode()?;
    Ok(())
}
