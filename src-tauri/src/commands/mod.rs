pub(crate) mod desktop;
pub(crate) mod diagnostics;
pub(crate) mod preferences;
pub(crate) mod status;

pub use desktop::{get_autostart_enabled, set_autostart_enabled, show_desktop_notification};
pub use diagnostics::get_diagnostics;
pub use preferences::set_preferences;
pub use status::{get_status, refresh_provider};
