//! Error types for the whatawhat-lib crate.
//!
//! This module provides a structured error type with an inner `ErrorKind` enum
//! that categorizes different error conditions. The outer `Error` struct is
//! designed for future extensibility (e.g., adding context, backtraces, etc.).

use std::fmt;

/// The main error type for this crate.
///
/// This struct wraps an `ErrorKind` and is designed for future extensibility.
/// Additional context or metadata can be added to this struct without breaking
/// the public API.
#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
}

impl Error {
    /// Create a new error from an error kind.
    pub fn new(kind: ErrorKind) -> Self {
        Self { kind }
    }

    /// Returns the kind of this error.
    pub fn kind(&self) -> &ErrorKind {
        &self.kind
    }

    /// Consumes this error and returns the underlying error kind.
    pub fn into_kind(self) -> ErrorKind {
        self.kind
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kind)
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.kind.source()
    }
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Self {
        Self::new(kind)
    }
}

// Blanket From implementations for external error types.
// These allow the `?` operator to convert external errors to our Error type directly.

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        ErrorKind::Io(err).into()
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        ErrorKind::Json(err).into()
    }
}

impl From<std::num::ParseIntError> for Error {
    fn from(err: std::num::ParseIntError) -> Self {
        ErrorKind::ParseInt(err).into()
    }
}

impl From<std::env::VarError> for Error {
    fn from(err: std::env::VarError) -> Self {
        ErrorKind::EnvVar(err).into()
    }
}

#[cfg(feature = "x11")]
impl From<xcb::Error> for Error {
    fn from(err: xcb::Error) -> Self {
        ErrorKind::Xcb(err).into()
    }
}

#[cfg(feature = "x11")]
impl From<xcb::ConnError> for Error {
    fn from(err: xcb::ConnError) -> Self {
        ErrorKind::XcbConnection(err).into()
    }
}

#[cfg(any(feature = "gnome", feature = "kde"))]
impl From<zbus::Error> for Error {
    fn from(err: zbus::Error) -> Self {
        ErrorKind::Zbus(err).into()
    }
}

#[cfg(feature = "wayland")]
impl From<wayland_client::backend::WaylandError> for Error {
    fn from(err: wayland_client::backend::WaylandError) -> Self {
        ErrorKind::WaylandBackend(err).into()
    }
}

#[cfg(feature = "wayland")]
impl From<wayland_client::ConnectError> for Error {
    fn from(err: wayland_client::ConnectError) -> Self {
        ErrorKind::WaylandConnect(err).into()
    }
}

#[cfg(feature = "wayland")]
impl From<wayland_client::DispatchError> for Error {
    fn from(err: wayland_client::DispatchError) -> Self {
        ErrorKind::WaylandDispatch(err).into()
    }
}

#[cfg(feature = "wayland")]
impl From<wayland_client::globals::GlobalError> for Error {
    fn from(err: wayland_client::globals::GlobalError) -> Self {
        ErrorKind::WaylandGlobal(err).into()
    }
}

#[cfg(feature = "wayland")]
impl From<wayland_client::globals::BindError> for Error {
    fn from(err: wayland_client::globals::BindError) -> Self {
        ErrorKind::WaylandBind(err).into()
    }
}

#[cfg(feature = "win")]
impl From<windows::core::Error> for Error {
    fn from(err: windows::core::Error) -> Self {
        ErrorKind::Windows(err).into()
    }
}

/// Categorizes the different types of errors that can occur.
#[derive(Debug, thiserror::Error)]
pub enum ErrorKind {
    // ========================
    // General / Initialization
    // ========================
    /// No window manager was selected or available.
    #[error("no window manager was selected or available")]
    NoWindowManager,

    /// The current display server should use X11 instead.
    #[error("X11 should be used instead of the current backend")]
    ShouldUseX11,

    // ========================
    // Window Operations
    // ========================
    /// Failed to get the foreground window.
    #[error("failed to get the foreground window")]
    ForegroundWindowNotFound,

    /// Failed to get the active window.
    #[error("failed to get the active window: {0}")]
    ActiveWindowNotFound(String),

    /// The current window is unknown (no window has been activated yet).
    #[error("the current window is unknown (no window has been activated yet)")]
    CurrentWindowUnknown,

    /// A window was not found by its ID.
    #[error("window not found by ID: {0}")]
    WindowNotFoundById(String),

    /// Failed to get the window process ID.
    #[error("failed to get the window process ID")]
    ProcessIdNotFound,

    /// Failed to get the process name.
    #[error("failed to get the process name for the window")]
    ProcessNameNotFound,

    // ========================
    // Idle Detection
    // ========================
    /// Failed to retrieve user idle time.
    #[error("failed to retrieve user idle time")]
    IdleTimeRetrievalFailed,

    /// Failed to get idle time from DBus.
    #[error("failed to get idle time from DBus: {0}")]
    DbusIdleTimeFailed(String),

    /// Failed to deserialize idle time.
    #[error("failed to deserialize idle time response")]
    IdleTimeDeserializationFailed,

    // ========================
    // X11 Specific
    // ========================
    /// X11 connection error.
    #[error("X11 connection error: {0}")]
    X11Connection(String),

    /// Invalid X11 screen configuration.
    #[error("invalid X11 screen: preferred screen is negative ({0})")]
    X11InvalidScreen(i32),

    // ========================
    // Wayland Specific
    // ========================
    /// Failed to connect to the Wayland compositor.
    #[error("failed to connect to the Wayland compositor")]
    WaylandConnectionFailed,

    /// Wayland event queue processing failed.
    #[error("Wayland event queue processing failed: {0}")]
    WaylandEventQueueFailed(String),

    /// Failed to bind a Wayland global.
    #[error("failed to bind Wayland global: {0}")]
    WaylandGlobalBindFailed(String),

    // ========================
    // GNOME Specific
    // ========================
    /// The runtime doesn't appear to be GNOME.
    #[error("the runtime doesn't appear to be GNOME")]
    NotGnomeRuntime,

    /// The GNOME extension appears to have stopped.
    #[error("the GNOME extension appears to have stopped responding")]
    GnomeExtensionStopped,

    /// Failed to parse DBus response.
    #[error("failed to parse DBus response: {0}")]
    DbusResponseParseFailed(String),

    /// DBus call failed.
    #[error("DBus call failed: {0}")]
    DbusCallFailed(String),

    /// Failed to install GNOME extension.
    #[error("failed to install GNOME extension")]
    GnomeExtensionInstallFailed,

    /// Failed to activate GNOME extension.
    #[error("failed to activate GNOME extension")]
    GnomeExtensionActivateFailed,

    // ========================
    // KDE Specific
    // ========================
    /// Failed to create KWin script.
    #[error("failed to create KWin script: {0}")]
    KwinScriptCreationFailed(String),

    /// Failed to start KWin script.
    #[error("failed to start KWin script")]
    KwinScriptStartFailed,

    /// KWin version not found.
    #[error("KWin version not found in support information")]
    KwinVersionNotFound,

    /// KWin version is invalid.
    #[error("KWin version is invalid: {0}")]
    KwinVersionInvalid(String),

    /// Failed to run DBus interface.
    #[error("failed to run DBus interface: {0}")]
    DbusInterfaceFailed(String),

    /// Temporary file path is not valid UTF-8.
    #[error("temporary file path is not valid UTF-8")]
    InvalidTempPath,

    // ========================
    // macOS Specific
    // ========================
    /// OSAScript execution error.
    #[error("OSAScript execution error: {0}")]
    OsaScriptExecutionFailed(String),

    /// OSAScript compilation error.
    #[error("OSAScript compilation error: {0}")]
    OsaScriptCompilationFailed(String),

    /// No result from OSAScript execution.
    #[error("no result returned from OSAScript execution")]
    OsaScriptNoResult,

    /// OSAScript did not return a string value.
    #[error("OSAScript did not return a string value")]
    OsaScriptNotString,

    /// Failed to parse JXA JSON output.
    #[error("failed to parse JXA JSON: {reason}; payload: {payload}")]
    JxaJsonParseFailed { reason: String, payload: String },

    /// No app info was loaded.
    #[error("no app info was loaded from the background process")]
    NoAppInfoLoaded,

    /// Failed to get JavaScript OSALanguage.
    #[error("failed to get JavaScript OSALanguage")]
    OsaLanguageNotFound,

    /// macOS permissions may be denied (startup error).
    #[error("macOS startup error (permissions may be denied): {0}")]
    MacosStartupFailed(String),

    // ========================
    // I/O and System Errors
    // ========================
    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON parsing error.
    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),

    /// Integer parsing error.
    #[error("integer parsing error: {0}")]
    ParseInt(#[from] std::num::ParseIntError),

    /// Environment variable error.
    #[error("environment variable error: {0}")]
    EnvVar(#[from] std::env::VarError),

    // ========================
    // External Library Errors
    // ========================
    /// XCB (X11) protocol error.
    #[cfg(feature = "x11")]
    #[error("XCB protocol error: {0}")]
    Xcb(#[from] xcb::Error),

    /// XCB connection error.
    #[cfg(feature = "x11")]
    #[error("XCB connection error: {0}")]
    XcbConnection(#[from] xcb::ConnError),

    /// ZBus (DBus) error.
    #[cfg(any(feature = "gnome", feature = "kde"))]
    #[error("DBus error: {0}")]
    Zbus(#[from] zbus::Error),

    /// Wayland backend error.
    #[cfg(feature = "wayland")]
    #[error("Wayland backend error: {0}")]
    WaylandBackend(#[from] wayland_client::backend::WaylandError),

    /// Wayland connection error.
    #[cfg(feature = "wayland")]
    #[error("Wayland connection error: {0}")]
    WaylandConnect(#[from] wayland_client::ConnectError),

    /// Wayland dispatch error.
    #[cfg(feature = "wayland")]
    #[error("Wayland dispatch error: {0}")]
    WaylandDispatch(#[from] wayland_client::DispatchError),

    /// Wayland global error.
    #[cfg(feature = "wayland")]
    #[error("Wayland global error: {0}")]
    WaylandGlobal(#[from] wayland_client::globals::GlobalError),

    /// Wayland bind error.
    #[cfg(feature = "wayland")]
    #[error("Wayland bind error: {0}")]
    WaylandBind(#[from] wayland_client::globals::BindError),

    /// Windows API error.
    #[cfg(feature = "win")]
    #[error("Windows API error: {0}")]
    Windows(#[from] windows::core::Error),
}

/// A specialized Result type for this crate.
pub type Result<T> = std::result::Result<T, Error>;

// ========================
// Convenience constructors
// ========================

impl Error {
    /// Creates an error for when no window manager is available.
    pub fn no_window_manager() -> Self {
        ErrorKind::NoWindowManager.into()
    }

    /// Creates an error indicating X11 should be used instead.
    pub fn should_use_x11() -> Self {
        ErrorKind::ShouldUseX11.into()
    }

    /// Creates an error for when the foreground window cannot be found.
    pub fn foreground_window_not_found() -> Self {
        ErrorKind::ForegroundWindowNotFound.into()
    }

    /// Creates an error for when the active window cannot be found.
    pub fn active_window_not_found(details: impl Into<String>) -> Self {
        ErrorKind::ActiveWindowNotFound(details.into()).into()
    }

    /// Creates an error for when the current window is unknown.
    pub fn current_window_unknown() -> Self {
        ErrorKind::CurrentWindowUnknown.into()
    }

    /// Creates an error for when a window is not found by ID.
    pub fn window_not_found_by_id(id: impl Into<String>) -> Self {
        ErrorKind::WindowNotFoundById(id.into()).into()
    }

    /// Creates an error for when the process ID cannot be found.
    pub fn process_id_not_found() -> Self {
        ErrorKind::ProcessIdNotFound.into()
    }

    /// Creates an error for when the process name cannot be found.
    pub fn process_name_not_found() -> Self {
        ErrorKind::ProcessNameNotFound.into()
    }

    /// Creates an error for idle time retrieval failure.
    pub fn idle_time_retrieval_failed() -> Self {
        ErrorKind::IdleTimeRetrievalFailed.into()
    }

    /// Creates an error for DBus idle time failure.
    pub fn dbus_idle_time_failed(details: impl Into<String>) -> Self {
        ErrorKind::DbusIdleTimeFailed(details.into()).into()
    }

    /// Creates an error for idle time deserialization failure.
    pub fn idle_time_deserialization_failed() -> Self {
        ErrorKind::IdleTimeDeserializationFailed.into()
    }

    /// Creates an error for X11 connection issues.
    #[cfg(feature = "x11")]
    pub fn x11_connection(details: impl Into<String>) -> Self {
        ErrorKind::X11Connection(details.into()).into()
    }

    /// Creates an error for invalid X11 screen.
    #[cfg(feature = "x11")]
    pub fn x11_invalid_screen(screen: i32) -> Self {
        ErrorKind::X11InvalidScreen(screen).into()
    }

    /// Creates an error for Wayland connection failure.
    #[cfg(feature = "wayland")]
    pub fn wayland_connection_failed() -> Self {
        ErrorKind::WaylandConnectionFailed.into()
    }

    /// Creates an error for Wayland event queue failure.
    #[cfg(feature = "wayland")]
    pub fn wayland_event_queue_failed(details: impl Into<String>) -> Self {
        ErrorKind::WaylandEventQueueFailed(details.into()).into()
    }

    /// Creates an error for Wayland global bind failure.
    #[cfg(feature = "wayland")]
    pub fn wayland_global_bind_failed(details: impl Into<String>) -> Self {
        ErrorKind::WaylandGlobalBindFailed(details.into()).into()
    }

    /// Creates an error for non-GNOME runtime.
    #[cfg(feature = "gnome")]
    pub fn not_gnome_runtime() -> Self {
        ErrorKind::NotGnomeRuntime.into()
    }

    /// Creates an error for GNOME extension stopped.
    #[cfg(feature = "gnome")]
    pub fn gnome_extension_stopped() -> Self {
        ErrorKind::GnomeExtensionStopped.into()
    }

    /// Creates an error for DBus response parse failure.
    pub fn dbus_response_parse_failed(details: impl Into<String>) -> Self {
        ErrorKind::DbusResponseParseFailed(details.into()).into()
    }

    /// Creates an error for DBus call failure.
    pub fn dbus_call_failed(details: impl Into<String>) -> Self {
        ErrorKind::DbusCallFailed(details.into()).into()
    }

    /// Creates an error for GNOME extension install failure.
    pub fn gnome_extension_install_failed() -> Self {
        ErrorKind::GnomeExtensionInstallFailed.into()
    }

    /// Creates an error for GNOME extension activate failure.
    pub fn gnome_extension_activate_failed() -> Self {
        ErrorKind::GnomeExtensionActivateFailed.into()
    }

    /// Creates an error for KWin script creation failure.
    #[cfg(feature = "kde")]
    pub fn kwin_script_creation_failed(details: impl Into<String>) -> Self {
        ErrorKind::KwinScriptCreationFailed(details.into()).into()
    }

    /// Creates an error for KWin script start failure.
    #[cfg(feature = "kde")]
    pub fn kwin_script_start_failed() -> Self {
        ErrorKind::KwinScriptStartFailed.into()
    }

    /// Creates an error for KWin version not found.
    #[cfg(feature = "kde")]
    pub fn kwin_version_not_found() -> Self {
        ErrorKind::KwinVersionNotFound.into()
    }

    /// Creates an error for invalid KWin version.
    #[cfg(feature = "kde")]
    pub fn kwin_version_invalid(version: impl Into<String>) -> Self {
        ErrorKind::KwinVersionInvalid(version.into()).into()
    }

    /// Creates an error for DBus interface failure.
    pub fn dbus_interface_failed(details: impl Into<String>) -> Self {
        ErrorKind::DbusInterfaceFailed(details.into()).into()
    }

    /// Creates an error for invalid temp path.
    #[cfg(feature = "kde")]
    pub fn invalid_temp_path() -> Self {
        ErrorKind::InvalidTempPath.into()
    }

    /// Creates an error for OSAScript execution failure.
    #[cfg(feature = "macos")]
    pub fn osa_script_execution_failed(details: impl Into<String>) -> Self {
        ErrorKind::OsaScriptExecutionFailed(details.into()).into()
    }

    /// Creates an error for OSAScript compilation failure.
    #[cfg(feature = "macos")]
    pub fn osa_script_compilation_failed(details: impl Into<String>) -> Self {
        ErrorKind::OsaScriptCompilationFailed(details.into()).into()
    }

    /// Creates an error for no OSAScript result.
    #[cfg(feature = "macos")]
    pub fn osa_script_no_result() -> Self {
        ErrorKind::OsaScriptNoResult.into()
    }

    /// Creates an error for OSAScript not returning a string.
    #[cfg(feature = "macos")]
    pub fn osa_script_not_string() -> Self {
        ErrorKind::OsaScriptNotString.into()
    }

    /// Creates an error for JXA JSON parse failure.
    #[cfg(feature = "macos")]
    pub fn jxa_json_parse_failed(reason: impl Into<String>, payload: impl Into<String>) -> Self {
        ErrorKind::JxaJsonParseFailed {
            reason: reason.into(),
            payload: payload.into(),
        }
        .into()
    }

    /// Creates an error for no app info loaded.
    #[cfg(feature = "macos")]
    pub fn no_app_info_loaded() -> Self {
        ErrorKind::NoAppInfoLoaded.into()
    }

    /// Creates an error for OSA language not found.
    #[cfg(feature = "macos")]
    pub fn osa_language_not_found() -> Self {
        ErrorKind::OsaLanguageNotFound.into()
    }

    /// Creates an error for macOS startup failure.
    #[cfg(feature = "macos")]
    pub fn macos_startup_failed(details: impl Into<String>) -> Self {
        ErrorKind::MacosStartupFailed(details.into()).into()
    }
}
