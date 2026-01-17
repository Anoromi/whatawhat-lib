//! Error types for the whatawhat-lib crate.
//!
//! This module provides a structured error type with nested `ErrorKind` enums
//! that categorize different error conditions by domain and platform.

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

// ============================================================================
// Top-level ErrorKind
// ============================================================================

/// Categorizes the different types of errors that can occur.
#[derive(Debug, thiserror::Error)]
pub enum ErrorKind {
    /// No window manager was selected or available.
    #[error("no window manager was selected or available")]
    NoWindowManager,

    /// The current display server should use X11 instead.
    #[error("X11 should be used instead of the current backend")]
    ShouldUseX11,

    /// Window-related errors.
    #[error(transparent)]
    Window(#[from] WindowError),

    /// Idle detection errors.
    #[error(transparent)]
    Idle(#[from] IdleError),

    /// Platform-specific errors.
    #[error(transparent)]
    Platform(#[from] PlatformError),

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
}

// ============================================================================
// Window Errors
// ============================================================================

/// Errors related to window operations.
#[derive(Debug, thiserror::Error)]
pub enum WindowError {
    /// Failed to get the foreground window.
    #[error("failed to get the foreground window")]
    ForegroundNotFound,

    /// Failed to get the active window.
    #[error("failed to get the active window: {0}")]
    ActiveNotFound(String),

    /// The current window is unknown (no window has been activated yet).
    #[error("the current window is unknown (no window has been activated yet)")]
    CurrentUnknown,

    /// A window was not found by its ID.
    #[error("window not found by ID: {0}")]
    NotFoundById(String),

    /// Failed to get the window process ID.
    #[error("failed to get the window process ID")]
    ProcessIdNotFound,

    /// Failed to get the process name.
    #[error("failed to get the process name for the window")]
    ProcessNameNotFound,
}

// ============================================================================
// Idle Errors
// ============================================================================

/// Errors related to idle detection.
#[derive(Debug, thiserror::Error)]
pub enum IdleError {
    /// Failed to retrieve user idle time.
    #[error("failed to retrieve user idle time")]
    RetrievalFailed,

    /// Failed to deserialize idle time response.
    #[error("failed to deserialize idle time response")]
    DeserializationFailed,
}

// ============================================================================
// Platform Errors
// ============================================================================

/// Platform-specific errors.
#[derive(Debug, thiserror::Error)]
pub enum PlatformError {
    /// X11-specific error.
    #[cfg(feature = "x11")]
    #[error(transparent)]
    X11(#[from] X11Error),

    /// Wayland-specific error.
    #[cfg(feature = "wayland")]
    #[error(transparent)]
    Wayland(#[from] WaylandError),

    /// GNOME-specific error.
    #[cfg(feature = "gnome")]
    #[error(transparent)]
    Gnome(#[from] GnomeError),

    /// KDE-specific error.
    #[cfg(feature = "kde")]
    #[error(transparent)]
    Kde(#[from] KdeError),

    /// macOS-specific error.
    #[cfg(feature = "macos")]
    #[error(transparent)]
    Macos(#[from] MacosError),

    /// Windows-specific error.
    #[cfg(feature = "win")]
    #[error(transparent)]
    Windows(#[from] WindowsError),

    /// DBus-related error (shared by GNOME and KDE).
    #[cfg(any(feature = "gnome", feature = "kde"))]
    #[error(transparent)]
    Dbus(#[from] DbusError),
}

// ============================================================================
// X11 Errors
// ============================================================================

/// X11-specific errors.
#[cfg(feature = "x11")]
#[derive(Debug, thiserror::Error)]
pub enum X11Error {
    /// Invalid X11 screen configuration.
    #[error("invalid X11 screen: preferred screen is negative ({0})")]
    InvalidScreen(i32),

    /// X11 connection error (string description).
    #[error("X11 connection error: {0}")]
    Connection(String),

    /// XCB protocol error.
    #[error("XCB protocol error: {0}")]
    Protocol(#[from] xcb::Error),

    /// XCB connection error.
    #[error("XCB connection error: {0}")]
    ConnError(#[from] xcb::ConnError),
}

// ============================================================================
// Wayland Errors
// ============================================================================

/// Wayland-specific errors.
#[cfg(feature = "wayland")]
#[derive(Debug, thiserror::Error)]
pub enum WaylandError {
    /// Failed to connect to the Wayland compositor.
    #[error("failed to connect to the Wayland compositor")]
    ConnectionFailed,

    /// Wayland event queue processing failed.
    #[error("Wayland event queue processing failed: {0}")]
    EventQueueFailed(String),

    /// Wayland backend error.
    #[error("Wayland backend error: {0}")]
    Backend(#[from] wayland_client::backend::WaylandError),

    /// Wayland connection error.
    #[error("Wayland connection error: {0}")]
    Connect(#[from] wayland_client::ConnectError),

    /// Wayland dispatch error.
    #[error("Wayland dispatch error: {0}")]
    Dispatch(#[from] wayland_client::DispatchError),

    /// Wayland global error.
    #[error("Wayland global error: {0}")]
    Global(#[from] wayland_client::globals::GlobalError),

    /// Wayland bind error.
    #[error("Wayland bind error: {0}")]
    Bind(#[from] wayland_client::globals::BindError),
}

// ============================================================================
// DBus Errors (shared by GNOME and KDE)
// ============================================================================

/// DBus-related errors.
#[cfg(any(feature = "gnome", feature = "kde"))]
#[derive(Debug, thiserror::Error)]
pub enum DbusError {
    /// Failed to parse DBus response.
    #[error("failed to parse DBus response: {0}")]
    ResponseParseFailed(String),

    /// DBus call failed.
    #[error("DBus call failed: {0}")]
    CallFailed(String),

    /// Failed to get idle time from DBus.
    #[error("failed to get idle time from DBus: {0}")]
    IdleTimeFailed(String),

    /// Failed to run DBus interface.
    #[error("failed to run DBus interface: {0}")]
    InterfaceFailed(String),

    /// ZBus error.
    #[error("ZBus error: {0}")]
    Zbus(#[from] zbus::Error),
}

// ============================================================================
// GNOME Errors
// ============================================================================

/// GNOME-specific errors.
#[cfg(feature = "gnome")]
#[derive(Debug, thiserror::Error)]
pub enum GnomeError {
    /// The runtime doesn't appear to be GNOME.
    #[error("the runtime doesn't appear to be GNOME")]
    NotGnomeRuntime,

    /// The GNOME extension appears to have stopped.
    #[error("the GNOME extension appears to have stopped responding")]
    ExtensionStopped,

    /// Failed to install GNOME extension.
    #[error("failed to install GNOME extension")]
    ExtensionInstallFailed,

    /// Failed to activate GNOME extension.
    #[error("failed to activate GNOME extension")]
    ExtensionActivateFailed,
}

// ============================================================================
// KDE Errors
// ============================================================================

/// KDE-specific errors.
#[cfg(feature = "kde")]
#[derive(Debug, thiserror::Error)]
pub enum KdeError {
    /// Failed to create KWin script.
    #[error("failed to create KWin script: {0}")]
    ScriptCreationFailed(String),

    /// Failed to start KWin script.
    #[error("failed to start KWin script")]
    ScriptStartFailed,

    /// KWin version not found.
    #[error("KWin version not found in support information")]
    VersionNotFound,

    /// KWin version is invalid.
    #[error("KWin version is invalid: {0}")]
    VersionInvalid(String),

    /// Temporary file path is not valid UTF-8.
    #[error("temporary file path is not valid UTF-8")]
    InvalidTempPath,
}

// ============================================================================
// macOS Errors
// ============================================================================

/// macOS-specific errors.
#[cfg(feature = "macos")]
#[derive(Debug, thiserror::Error)]
pub enum MacosError {
    /// OSAScript execution error.
    #[error("OSAScript execution error: {0}")]
    ScriptExecutionFailed(String),

    /// OSAScript compilation error.
    #[error("OSAScript compilation error: {0}")]
    ScriptCompilationFailed(String),

    /// No result from OSAScript execution.
    #[error("no result returned from OSAScript execution")]
    ScriptNoResult,

    /// OSAScript did not return a string value.
    #[error("OSAScript did not return a string value")]
    ScriptNotString,

    /// Failed to parse JXA JSON output.
    #[error("failed to parse JXA JSON: {reason}; payload: {payload}")]
    JxaJsonParseFailed { reason: String, payload: String },

    /// No app info was loaded.
    #[error("no app info was loaded from the background process")]
    NoAppInfoLoaded,

    /// Failed to get JavaScript OSALanguage.
    #[error("failed to get JavaScript OSALanguage")]
    LanguageNotFound,

    /// macOS permissions may be denied (startup error).
    #[error("macOS startup error (permissions may be denied): {0}")]
    StartupFailed(String),
}

// ============================================================================
// Windows Errors
// ============================================================================

/// Windows-specific errors.
#[cfg(feature = "win")]
#[derive(Debug, thiserror::Error)]
pub enum WindowsError {
    /// Windows API error.
    #[error("Windows API error: {0}")]
    Api(#[from] windows::core::Error),
}

// ============================================================================
// Result type alias
// ============================================================================

/// A specialized Result type for this crate.
pub type Result<T> = std::result::Result<T, Error>;

// ============================================================================
// From implementations for Error
// ============================================================================

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

impl From<WindowError> for Error {
    fn from(err: WindowError) -> Self {
        ErrorKind::Window(err).into()
    }
}

impl From<IdleError> for Error {
    fn from(err: IdleError) -> Self {
        ErrorKind::Idle(err).into()
    }
}

impl From<PlatformError> for Error {
    fn from(err: PlatformError) -> Self {
        ErrorKind::Platform(err).into()
    }
}

#[cfg(feature = "x11")]
impl From<X11Error> for Error {
    fn from(err: X11Error) -> Self {
        PlatformError::X11(err).into()
    }
}

#[cfg(feature = "x11")]
impl From<xcb::Error> for Error {
    fn from(err: xcb::Error) -> Self {
        X11Error::Protocol(err).into()
    }
}

#[cfg(feature = "x11")]
impl From<xcb::ConnError> for Error {
    fn from(err: xcb::ConnError) -> Self {
        X11Error::ConnError(err).into()
    }
}

#[cfg(feature = "wayland")]
impl From<WaylandError> for Error {
    fn from(err: WaylandError) -> Self {
        PlatformError::Wayland(err).into()
    }
}

#[cfg(feature = "wayland")]
impl From<wayland_client::backend::WaylandError> for Error {
    fn from(err: wayland_client::backend::WaylandError) -> Self {
        WaylandError::Backend(err).into()
    }
}

#[cfg(feature = "wayland")]
impl From<wayland_client::ConnectError> for Error {
    fn from(err: wayland_client::ConnectError) -> Self {
        WaylandError::Connect(err).into()
    }
}

#[cfg(feature = "wayland")]
impl From<wayland_client::DispatchError> for Error {
    fn from(err: wayland_client::DispatchError) -> Self {
        WaylandError::Dispatch(err).into()
    }
}

#[cfg(feature = "wayland")]
impl From<wayland_client::globals::GlobalError> for Error {
    fn from(err: wayland_client::globals::GlobalError) -> Self {
        WaylandError::Global(err).into()
    }
}

#[cfg(feature = "wayland")]
impl From<wayland_client::globals::BindError> for Error {
    fn from(err: wayland_client::globals::BindError) -> Self {
        WaylandError::Bind(err).into()
    }
}

#[cfg(any(feature = "gnome", feature = "kde"))]
impl From<DbusError> for Error {
    fn from(err: DbusError) -> Self {
        PlatformError::Dbus(err).into()
    }
}

#[cfg(any(feature = "gnome", feature = "kde"))]
impl From<zbus::Error> for Error {
    fn from(err: zbus::Error) -> Self {
        DbusError::Zbus(err).into()
    }
}

#[cfg(feature = "gnome")]
impl From<GnomeError> for Error {
    fn from(err: GnomeError) -> Self {
        PlatformError::Gnome(err).into()
    }
}

#[cfg(feature = "kde")]
impl From<KdeError> for Error {
    fn from(err: KdeError) -> Self {
        PlatformError::Kde(err).into()
    }
}

#[cfg(feature = "macos")]
impl From<MacosError> for Error {
    fn from(err: MacosError) -> Self {
        PlatformError::Macos(err).into()
    }
}

#[cfg(feature = "win")]
impl From<WindowsError> for Error {
    fn from(err: WindowsError) -> Self {
        PlatformError::Windows(err).into()
    }
}

#[cfg(feature = "win")]
impl From<windows::core::Error> for Error {
    fn from(err: windows::core::Error) -> Self {
        WindowsError::Api(err).into()
    }
}

// ============================================================================
// Convenience constructors on Error
// ============================================================================

impl Error {
    // General
    pub fn no_window_manager() -> Self {
        ErrorKind::NoWindowManager.into()
    }

    pub fn should_use_x11() -> Self {
        ErrorKind::ShouldUseX11.into()
    }

    // Window errors
    pub fn foreground_window_not_found() -> Self {
        WindowError::ForegroundNotFound.into()
    }

    pub fn active_window_not_found(details: impl Into<String>) -> Self {
        WindowError::ActiveNotFound(details.into()).into()
    }

    pub fn current_window_unknown() -> Self {
        WindowError::CurrentUnknown.into()
    }

    pub fn window_not_found_by_id(id: impl Into<String>) -> Self {
        WindowError::NotFoundById(id.into()).into()
    }

    pub fn process_id_not_found() -> Self {
        WindowError::ProcessIdNotFound.into()
    }

    pub fn process_name_not_found() -> Self {
        WindowError::ProcessNameNotFound.into()
    }

    // Idle errors
    pub fn idle_time_retrieval_failed() -> Self {
        IdleError::RetrievalFailed.into()
    }

    pub fn idle_time_deserialization_failed() -> Self {
        IdleError::DeserializationFailed.into()
    }

    // X11 errors
    #[cfg(feature = "x11")]
    pub fn x11_invalid_screen(screen: i32) -> Self {
        X11Error::InvalidScreen(screen).into()
    }

    #[cfg(feature = "x11")]
    pub fn x11_connection(details: impl Into<String>) -> Self {
        X11Error::Connection(details.into()).into()
    }

    // Wayland errors
    #[cfg(feature = "wayland")]
    pub fn wayland_connection_failed() -> Self {
        WaylandError::ConnectionFailed.into()
    }

    #[cfg(feature = "wayland")]
    pub fn wayland_event_queue_failed(details: impl Into<String>) -> Self {
        WaylandError::EventQueueFailed(details.into()).into()
    }

    // DBus errors
    #[cfg(any(feature = "gnome", feature = "kde"))]
    pub fn dbus_response_parse_failed(details: impl Into<String>) -> Self {
        DbusError::ResponseParseFailed(details.into()).into()
    }

    #[cfg(any(feature = "gnome", feature = "kde"))]
    pub fn dbus_call_failed(details: impl Into<String>) -> Self {
        DbusError::CallFailed(details.into()).into()
    }

    #[cfg(any(feature = "gnome", feature = "kde"))]
    pub fn dbus_idle_time_failed(details: impl Into<String>) -> Self {
        DbusError::IdleTimeFailed(details.into()).into()
    }

    #[cfg(any(feature = "gnome", feature = "kde"))]
    pub fn dbus_interface_failed(details: impl Into<String>) -> Self {
        DbusError::InterfaceFailed(details.into()).into()
    }

    // GNOME errors
    #[cfg(feature = "gnome")]
    pub fn not_gnome_runtime() -> Self {
        GnomeError::NotGnomeRuntime.into()
    }

    #[cfg(feature = "gnome")]
    pub fn gnome_extension_stopped() -> Self {
        GnomeError::ExtensionStopped.into()
    }

    pub fn gnome_extension_install_failed() -> Self {
        #[cfg(feature = "gnome")]
        return GnomeError::ExtensionInstallFailed.into();
        #[cfg(not(feature = "gnome"))]
        panic!("gnome feature not enabled")
    }

    pub fn gnome_extension_activate_failed() -> Self {
        #[cfg(feature = "gnome")]
        return GnomeError::ExtensionActivateFailed.into();
        #[cfg(not(feature = "gnome"))]
        panic!("gnome feature not enabled")
    }

    // KDE errors
    #[cfg(feature = "kde")]
    pub fn kwin_script_creation_failed(details: impl Into<String>) -> Self {
        KdeError::ScriptCreationFailed(details.into()).into()
    }

    #[cfg(feature = "kde")]
    pub fn kwin_script_start_failed() -> Self {
        KdeError::ScriptStartFailed.into()
    }

    #[cfg(feature = "kde")]
    pub fn kwin_version_not_found() -> Self {
        KdeError::VersionNotFound.into()
    }

    #[cfg(feature = "kde")]
    pub fn kwin_version_invalid(version: impl Into<String>) -> Self {
        KdeError::VersionInvalid(version.into()).into()
    }

    #[cfg(feature = "kde")]
    pub fn invalid_temp_path() -> Self {
        KdeError::InvalidTempPath.into()
    }

    // macOS errors
    #[cfg(feature = "macos")]
    pub fn osa_script_execution_failed(details: impl Into<String>) -> Self {
        MacosError::ScriptExecutionFailed(details.into()).into()
    }

    #[cfg(feature = "macos")]
    pub fn osa_script_compilation_failed(details: impl Into<String>) -> Self {
        MacosError::ScriptCompilationFailed(details.into()).into()
    }

    #[cfg(feature = "macos")]
    pub fn osa_script_no_result() -> Self {
        MacosError::ScriptNoResult.into()
    }

    #[cfg(feature = "macos")]
    pub fn osa_script_not_string() -> Self {
        MacosError::ScriptNotString.into()
    }

    #[cfg(feature = "macos")]
    pub fn jxa_json_parse_failed(reason: impl Into<String>, payload: impl Into<String>) -> Self {
        MacosError::JxaJsonParseFailed {
            reason: reason.into(),
            payload: payload.into(),
        }
        .into()
    }

    #[cfg(feature = "macos")]
    pub fn no_app_info_loaded() -> Self {
        MacosError::NoAppInfoLoaded.into()
    }

    #[cfg(feature = "macos")]
    pub fn osa_language_not_found() -> Self {
        MacosError::LanguageNotFound.into()
    }

    #[cfg(feature = "macos")]
    pub fn macos_startup_failed(details: impl Into<String>) -> Self {
        MacosError::StartupFailed(details.into()).into()
    }
}
