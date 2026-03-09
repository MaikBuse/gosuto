use std::io::Write;
use std::path::Path;

use anyhow::Result;

/// Write `contents` to `path` with owner-only permissions (0o600) on Unix.
/// On non-Unix platforms, falls back to `std::fs::write()`.
pub fn write_private_file(path: &Path, contents: &str) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(path)?;
        file.write_all(contents.as_bytes())?;
    }
    #[cfg(not(unix))]
    {
        std::fs::write(path, contents)?;
    }
    Ok(())
}
