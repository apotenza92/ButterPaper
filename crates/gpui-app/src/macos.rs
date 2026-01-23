//! macOS-specific functionality for appearance management

use crate::AppearanceMode;
use dispatch::Queue;
use objc2_app_kit::{NSAppearance, NSApplication};
use objc2_foundation::{MainThreadMarker, NSString};

/// Set the application's appearance based on the given mode.
/// Dispatched async to main queue to avoid re-entrancy issues with GPUI.
pub fn set_app_appearance(mode: AppearanceMode) {
    // Dispatch to main queue asynchronously to break out of GPUI's update cycle
    Queue::main().exec_async(move || {
        let appearance_name = match mode {
            AppearanceMode::Light => Some("NSAppearanceNameAqua"),
            AppearanceMode::Dark => Some("NSAppearanceNameDarkAqua"),
            AppearanceMode::System => None,
        };

        // SAFETY: dispatch to main queue guarantees we're on the main thread
        let mtm = unsafe { MainThreadMarker::new_unchecked() };
        let app = NSApplication::sharedApplication(mtm);

        let appearance = appearance_name.and_then(|name| {
            let name = NSString::from_str(name);
            NSAppearance::appearanceNamed(&name)
        });

        app.setAppearance(appearance.as_deref());
    });
}
