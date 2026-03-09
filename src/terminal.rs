use std::io::{self, Stdout};

use anyhow::Result;
use crossterm::{
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{Terminal, prelude::CrosstermBackend};

#[cfg(unix)]
use std::os::unix::io::{AsRawFd, FromRawFd, OwnedFd};

pub type Tui = Terminal<CrosstermBackend<Stdout>>;

pub fn init() -> Result<Tui> {
    let mut stdout = io::stdout();
    terminal::enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

/// Enable keyboard enhancement for key release events (needed for PTT).
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
    ratatui_image::picker::Picker::halfblocks()
}

/// Redirect stderr (fd 2) to `/dev/null`, returning the saved original fd.
///
/// This prevents native C libraries (e.g. WebRTC/livekit) from writing directly
/// to stderr via `write(2, ...)`, which would bleed through the TUI as visual
/// artifacts.
#[cfg(unix)]
pub fn suppress_stderr() -> Option<OwnedFd> {
    let dev_null = std::fs::File::open("/dev/null").ok()?;
    // SAFETY: dup/dup2 are standard POSIX. We own the returned fd via OwnedFd.
    let saved_fd = unsafe { libc::dup(2) };
    if saved_fd < 0 {
        return None;
    }
    unsafe { libc::dup2(dev_null.as_raw_fd(), 2) };
    Some(unsafe { OwnedFd::from_raw_fd(saved_fd) })
}

/// Restore stderr from a previously saved fd.
#[cfg(unix)]
pub fn restore_stderr(saved: &OwnedFd) {
    // SAFETY: restoring the original stderr fd.
    unsafe { libc::dup2(saved.as_raw_fd(), 2) };
}

pub fn restore() -> Result<()> {
    let mut stdout = io::stdout();

    // Pop keyboard enhancement (gracefully ignored if not supported)
    let _ = crossterm::execute!(stdout, crossterm::event::PopKeyboardEnhancementFlags);

    execute!(stdout, LeaveAlternateScreen)?;
    terminal::disable_raw_mode()?;
    Ok(())
}
