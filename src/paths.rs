use std::env;
use std::path::{Path, PathBuf};

/// Filesystem locations used by the application.
///
/// Every path funnels through [`Paths::root`] so that tests and the
/// `TERMUX_HOME` override can point the whole application at another
/// directory.
#[derive(Clone, Debug)]
pub struct Paths {
    root: PathBuf,
}

impl Paths {
    /// Build the paths rooted at an explicit Termux configuration directory.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Build the paths from the process environment.
    ///
    /// `$TERMUX_HOME` wins when set; otherwise `$HOME/.termux` is used. This is
    /// the only function that reads the process environment.
    pub fn from_env() -> Self {
        let termux_home = env::var_os("TERMUX_HOME")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from);
        let home = env::var_os("HOME")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from);
        Self::from_parts(termux_home, home)
    }

    /// Build the paths from already-resolved environment parts.
    ///
    /// When neither part is present the root is the relative `.termux`
    /// directory.
    pub fn from_parts(termux_home: Option<PathBuf>, home: Option<PathBuf>) -> Self {
        let root = termux_home
            .or_else(|| home.map(|home| home.join(".termux")))
            .unwrap_or_else(|| PathBuf::from(".termux"));
        Self::new(root)
    }

    /// The Termux configuration directory (`~/.termux`).
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Directory holding the user's theme library.
    pub fn colors_root(&self) -> PathBuf {
        self.root.join("colors")
    }

    /// The active colour scheme file.
    pub fn colors_properties(&self) -> PathBuf {
        self.root.join("colors.properties")
    }

    /// The main Termux app settings file.
    pub fn termux_properties(&self) -> PathBuf {
        self.root.join("termux.properties")
    }

    /// Where timestamped backups are written.
    pub fn backups(&self) -> PathBuf {
        self.root.join("backups")
    }
}
