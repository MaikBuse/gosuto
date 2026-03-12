use std::io;

use anyhow::Result;
use crossterm::{
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{Terminal, prelude::CrosstermBackend};

#[cfg(unix)]
use std::os::unix::io::{AsRawFd, FromRawFd, OwnedFd};

#[cfg(unix)]
use std::sync::OnceLock;

/// Ratatui writes through a dup'd fd so we can redirect the original fd 1
/// to `/dev/null` without breaking the TUI output.
#[cfg(unix)]
pub type Tui = Terminal<CrosstermBackend<std::io::BufWriter<std::fs::File>>>;

#[cfg(not(unix))]
pub type Tui = Terminal<CrosstermBackend<io::Stdout>>;

/// Raw fd of the saved stdout, stored globally so the panic handler can
/// restore fd 1 before printing the backtrace.
#[cfg(unix)]
static SAVED_STDOUT_FD: OnceLock<i32> = OnceLock::new();

#[cfg(unix)]
pub fn init() -> Result<Tui> {
    // Dup stdout so ratatui writes to the real terminal PTY via a separate
    // fd.  After init, main() redirects fd 1 to /dev/null so native C
    // libraries (VAAPI, GStreamer, …) can't corrupt the TUI.
    // SAFETY: dup is standard POSIX; we own the returned fd via File.
    let tui_fd = unsafe { libc::dup(1) };
    anyhow::ensure!(tui_fd >= 0, "dup(stdout) failed");
    let file = unsafe { std::fs::File::from_raw_fd(tui_fd) };
    let mut writer = std::io::BufWriter::with_capacity(1 << 20, file);

    terminal::enable_raw_mode()?;
    execute!(writer, EnterAlternateScreen)?;

    Ok(Terminal::new(CrosstermBackend::new(writer))?)
}

#[cfg(not(unix))]
pub fn init() -> Result<Tui> {
    let mut stdout = io::stdout();
    terminal::enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

/// Enable keyboard enhancement for key release events (needed for PTT).
///
/// Must be called **before** `suppress_stdout` so the escape sequence
/// reaches the real terminal via fd 1.
pub fn init_keyboard_enhancement() {
    let _ = crossterm::execute!(
        io::stdout(),
        crossterm::event::PushKeyboardEnhancementFlags(
            crossterm::event::KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                | crossterm::event::KeyboardEnhancementFlags::REPORT_EVENT_TYPES
                | crossterm::event::KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES
        )
    );
}

pub fn init_picker() -> ratatui_image::picker::Picker {
    ratatui_image::picker::Picker::halfblocks()
}

/// Redirect stdout (fd 1) to `/dev/null`, returning the saved original fd.
///
/// Native C libraries (VAAPI, GStreamer, WebRTC) sometimes write directly to
/// stdout via `write(1, ...)`, which corrupts the TUI.  Ratatui already uses
/// a dup'd fd (see [`init`]), so redirecting fd 1 is safe.
#[cfg(unix)]
pub fn suppress_stdout() -> Option<OwnedFd> {
    let dev_null = std::fs::File::open("/dev/null").ok()?;
    // SAFETY: dup/dup2 are standard POSIX. We own the returned fd via OwnedFd.
    let saved_fd = unsafe { libc::dup(1) };
    if saved_fd < 0 {
        return None;
    }
    unsafe { libc::dup2(dev_null.as_raw_fd(), 1) };
    // Store for the panic handler.
    let _ = SAVED_STDOUT_FD.set(saved_fd);
    Some(unsafe { OwnedFd::from_raw_fd(saved_fd) })
}

/// Restore stdout from a previously saved fd.
#[cfg(unix)]
pub fn restore_stdout(saved: &OwnedFd) {
    // SAFETY: restoring the original stdout fd.
    unsafe { libc::dup2(saved.as_raw_fd(), 1) };
}

/// Restore stdout from the global saved fd (for use in the panic handler).
#[cfg(unix)]
pub fn restore_stdout_from_global() {
    if let Some(&fd) = SAVED_STDOUT_FD.get() {
        // SAFETY: restoring the original stdout fd.
        unsafe { libc::dup2(fd, 1) };
    }
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

    execute!(stdout, crossterm::cursor::Show, LeaveAlternateScreen)?;
    terminal::disable_raw_mode()?;
    Ok(())
}
