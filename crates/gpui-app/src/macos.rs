//! macOS-specific functionality for appearance management

use crate::AppearanceMode;
use dispatch::Queue;
use objc2::ClassType;
use objc2_app_kit::{NSAppearance, NSApplication, NSImage};
use objc2_foundation::{MainThreadMarker, NSData, NSProcessInfo, NSString};

/// macOS major version where Tahoe-style icon appearance settings are introduced.
const MACOS_TAHOE_MAJOR: isize = 26;

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

/// Set the Dock icon for dev runs (non-bundled builds).
pub fn set_app_icon() {
    // Tahoe-style icon appearance uses asset-catalog metadata. Runtime icon overrides
    // force a static image and can bypass system appearance icon styles.
    if is_tahoe_or_newer() {
        return;
    }

    Queue::main().exec_async(move || {
        const ICON_BYTES: &[u8] = include_bytes!("../assets/app-icons/butterpaper-icon.icns");

        // SAFETY: dispatch to main queue guarantees we're on the main thread
        let mtm = unsafe { MainThreadMarker::new_unchecked() };
        let app = NSApplication::sharedApplication(mtm);

        let icon_data = NSData::with_bytes(ICON_BYTES);
        let image = NSImage::initWithData(NSImage::alloc(), &icon_data);
        if let Some(image) = image.as_deref() {
            unsafe { app.setApplicationIconImage(Some(image)) };
        }
    });
}

fn is_tahoe_or_newer() -> bool {
    let os_version = NSProcessInfo::processInfo().operatingSystemVersion();
    os_version.majorVersion >= MACOS_TAHOE_MAJOR
}
