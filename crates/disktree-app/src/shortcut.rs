//! A shortcut to this program where a Windows user looks for one: the Start
//! menu and the desktop.
//!
//! The Windows download is a bare executable in a zip, with no installer to
//! put it anywhere, so the settings page offers to. Linux has `make
//! install` and its desktop entry, and macOS its app bundle.

use std::io;
use std::path::PathBuf;

/// Where a shortcut can be put.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    StartMenu,
    Desktop,
}

impl Place {
    /// The place in words, for a notice.
    pub const fn name(self) -> &'static str {
        match self {
            Self::StartMenu => "the Start menu",
            Self::Desktop => "the desktop",
        }
    }

    /// The name Windows knows the folder by. Asked for by name, never
    /// built from the profile's path: the desktop is often moved into
    /// `OneDrive`, and only the system knows where it went.
    #[cfg(windows)]
    const fn folder(self) -> &'static str {
        match self {
            Self::StartMenu => "Programs",
            Self::Desktop => "Desktop",
        }
    }
}

/// Write `disktree.lnk` in `place`, pointing at this executable, and return
/// where it is. One that is already there is replaced, so the shortcut
/// follows the program when it is moved and the button is pressed again.
#[cfg(windows)]
pub fn create(place: Place) -> io::Result<PathBuf> {
    use std::os::windows::process::CommandExt as _;
    use std::process::Command;

    /// `CREATE_NO_WINDOW`: no console flashes up behind the settings page.
    const NO_WINDOW: u32 = 0x0800_0000;

    let target = std::env::current_exe()?;
    // A `.lnk` is written by the shell's own COM object, and PowerShell is
    // the way to it that every Windows has. The target goes through the
    // environment, never into the script's text, so no path can be read
    // as code.
    // The link's path comes back as UTF-8 whatever the console's code page:
    // a desktop moved into OneDrive is often named in the user's language.
    let script = format!(
        "$ErrorActionPreference = 'Stop'; \
         [Console]::OutputEncoding = [Text.Encoding]::UTF8; \
         $folder = [Environment]::GetFolderPath('{}'); \
         $link = Join-Path $folder 'disktree.lnk'; \
         $shortcut = (New-Object -ComObject WScript.Shell)\
         .CreateShortcut($link); \
         $shortcut.TargetPath = $env:DISKTREE_SHORTCUT_TARGET; \
         $shortcut.Description = 'A treemap of what is using your disk'; \
         $shortcut.Save(); \
         [Console]::Out.Write($link)",
        place.folder()
    );
    // By its full path: a `powershell.exe` earlier on PATH is not the one
    // to hand this to.
    let system = std::env::var_os("SystemRoot")
        .map(PathBuf::from)
        .ok_or_else(|| io::Error::other("SystemRoot is not set"))?;
    let output = Command::new(
        system.join("System32/WindowsPowerShell/v1.0/powershell.exe"),
    )
    .args(["-NoProfile", "-NonInteractive", "-Command", &script])
    .env("DISKTREE_SHORTCUT_TARGET", &target)
    .creation_flags(NO_WINDOW)
    .output()?;
    if !output.status.success() {
        let said = String::from_utf8_lossy(&output.stderr);
        return Err(io::Error::other(
            said.lines()
                .next()
                .unwrap_or("PowerShell failed")
                .to_owned(),
        ));
    }
    let link = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if link.is_empty() {
        return Err(io::Error::other("no shortcut was written"));
    }
    Ok(PathBuf::from(link))
}

/// Only Windows keeps shortcuts this way.
#[cfg(not(windows))]
pub fn create(_place: Place) -> io::Result<PathBuf> {
    Err(io::Error::from(io::ErrorKind::Unsupported))
}
