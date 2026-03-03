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

    // NOTE: init_picker() must be called before init_keyboard_enhancement()
    // because PushKeyboardEnhancementFlags corrupts the picker's capability
    // detection query (from_query_stdio).

    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

/// Enable keyboard enhancement for key release events (needed for PTT).
/// Must be called AFTER `init_picker()` — the enhancement flags change how the
/// terminal responds to escape sequences, which corrupts the picker's protocol
/// detection.
pub fn init_keyboard_enhancement() {
    let _ = crossterm::execute!(
        io::stdout(),
        crossterm::event::PushKeyboardEnhancementFlags(
            crossterm::event::KeyboardEnhancementFlags::REPORT_EVENT_TYPES
                | crossterm::event::KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES
        )
    );
}

pub fn init_picker() -> ratatui_image::picker::Picker {
    ratatui_image::picker::Picker::from_query_stdio()
        .unwrap_or_else(|_| ratatui_image::picker::Picker::halfblocks())
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
