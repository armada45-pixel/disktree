//! View choices kept between launches: the theme, what is measured and how
//! deep it is drawn, and the side panel.
//!
//! One `key=value` per line, beside the Power Efficiency preset. Reading is
//! lenient on purpose: a key this build does not know, or a value it cannot
//! use, is skipped and the rest still apply, so an older and a newer build
//! can share the file, and a hand edit gone wrong costs one setting instead
//! of all of them. The command line still wins over anything saved.

use std::io::{self, Write as _};
use std::path::{Path, PathBuf};

use crate::appearance::ThemeChoice;
use crate::state::{
    DEEP_LEVELS, PANEL_MAX_REMS, PANEL_MIN_REMS, PANEL_REMS, USUAL_LEVELS,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Settings {
    pub theme: ThemeChoice,
    /// Levels drawn at once: 1 to 6, or to 10 with `deep_levels`.
    pub depth: u32,
    pub include_hidden: bool,
    pub apparent_size: bool,
    /// The Size | Files | Age choice, as the control indexes it.
    pub mode: usize,
    /// The side panel's width, in rem.
    pub panel_rems: f32,
    /// Whether the side panel is shown at all.
    pub show_selection: bool,
    /// Whether more than six levels may be drawn at once.
    pub deep_levels: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: ThemeChoice::default(),
            depth: 3,
            include_hidden: true,
            apparent_size: false,
            mode: 0,
            panel_rems: PANEL_REMS,
            show_selection: true,
            deep_levels: false,
        }
    }
}

impl Settings {
    /// Read `text`, keeping the default for anything missing or unusable.
    pub fn parse(text: &str) -> Self {
        let mut settings = Self::default();
        let flag = |value: &str| match value {
            "true" => Some(true),
            "false" => Some(false),
            _ => None,
        };
        for line in text.lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let value = value.trim();
            match key.trim() {
                "theme" => {
                    if let Some(theme) = ThemeChoice::parse(value) {
                        settings.theme = theme;
                    }
                }
                "depth" => {
                    if let Ok(depth) = value.parse::<u32>()
                        && (1..=DEEP_LEVELS).contains(&depth)
                    {
                        settings.depth = depth;
                    }
                }
                "hidden" => {
                    if let Some(on) = flag(value) {
                        settings.include_hidden = on;
                    }
                }
                "apparent-size" => {
                    if let Some(on) = flag(value) {
                        settings.apparent_size = on;
                    }
                }
                "mode" => {
                    if let Some(index) =
                        MODES.iter().position(|mode| *mode == value)
                    {
                        settings.mode = index;
                    }
                }
                "panel-width" => {
                    // A width outside the limits a drag keeps to would
                    // open a panel the handle could not have made.
                    if let Ok(rems) = value.parse::<f32>()
                        && (PANEL_MIN_REMS..=PANEL_MAX_REMS).contains(&rems)
                    {
                        settings.panel_rems = rems;
                    }
                }
                "panel" => {
                    if let Some(on) = flag(value) {
                        settings.show_selection = on;
                    }
                }
                "deep-levels" => {
                    if let Some(on) = flag(value) {
                        settings.deep_levels = on;
                    }
                }
                _ => {}
            }
        }
        // The lines come in any order, so the depth is held to what the
        // file as a whole allows only once all of it is read.
        if !settings.deep_levels {
            settings.depth = settings.depth.min(USUAL_LEVELS);
        }
        settings
    }

    fn text(&self) -> String {
        format!(
            "theme={}\ndepth={}\nhidden={}\napparent-size={}\nmode={}\n\
             panel-width={:.2}\npanel={}\ndeep-levels={}\n",
            self.theme.key(),
            self.depth,
            self.include_hidden,
            self.apparent_size,
            MODES[self.mode.min(MODES.len() - 1)],
            self.panel_rems,
            self.show_selection,
            self.deep_levels,
        )
    }
}

/// The Size | Files | Age choice by name, in the control's order, so the
/// file still means the same if the control is ever reordered.
const MODES: [&str; 3] = ["size", "files", "age"];

/// Beside the Power Efficiency preset, in the same per-user directory.
pub fn path() -> Option<PathBuf> {
    crate::power::settings_path().map(|path| path.with_file_name("settings"))
}

/// The saved settings, or the defaults when nothing was saved yet.
pub fn load(path: &Path) -> io::Result<Settings> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(Settings::parse(&text)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            Ok(Settings::default())
        }
        Err(error) => Err(error),
    }
}

pub fn save(path: &Path, settings: &Settings) -> io::Result<()> {
    let parent = path.parent().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "no settings directory")
    })?;
    std::fs::create_dir_all(parent)?;
    // Replaced whole, as the preset beside it is: a failed write leaves
    // the previous choices, and a reader never sees half a file.
    let mut file = tempfile::NamedTempFile::new_in(parent)?;
    file.write_all(settings.text().as_bytes())?;
    file.persist(path).map_err(|error| error.error)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saved_choices_come_back_as_they_were() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("config/settings");
        assert_eq!(load(&path).expect("missing"), Settings::default());
        let chosen = Settings {
            theme: ThemeChoice::Light,
            depth: 5,
            include_hidden: false,
            apparent_size: true,
            mode: 2,
            panel_rems: 30.5,
            show_selection: false,
            deep_levels: false,
        };
        save(&path, &chosen).expect("save");
        assert_eq!(load(&path).expect("load"), chosen);
        assert!(save(dir.path(), &chosen).is_err());
    }

    #[test]
    fn one_bad_line_costs_one_setting() {
        let settings = Settings::parse(
            "theme=plaid\ndepth=4\nfrom-a-newer-build=yes\nnot a setting\n\
             hidden=maybe\nmode=files\npanel-width=900\n",
        );
        assert_eq!(
            settings,
            Settings {
                depth: 4,
                mode: 1,
                ..Settings::default()
            }
        );
    }

    #[test]
    fn a_depth_the_view_cannot_draw_is_not_taken() {
        assert_eq!(Settings::parse("depth=0").depth, 3);
        assert_eq!(Settings::parse("depth=6").depth, 6);
        // Past six only for whoever unlocked it, whichever line is first.
        assert_eq!(Settings::parse("depth=9").depth, 6);
        assert_eq!(Settings::parse("depth=9\ndeep-levels=true").depth, 9);
        assert_eq!(Settings::parse("deep-levels=true\ndepth=10").depth, 10);
        assert_eq!(Settings::parse("deep-levels=true\ndepth=11").depth, 3);
    }
}
