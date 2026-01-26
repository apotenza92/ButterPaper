//! macOS-specific functionality for appearance management

use crate::AppearanceMode;
use dispatch::Queue;
use objc2::ClassType;
use objc2_app_kit::{NSAppearance, NSApplication, NSImage};
use objc2_foundation::{MainThreadMarker, NSString};
use std::path::PathBuf;

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
    Queue::main().exec_async(move || {
        let icon_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("assets")
            .join("butterpaper-icon.icns");
        if !icon_path.exists() {
            return;
        }

        // SAFETY: dispatch to main queue guarantees we're on the main thread
        let mtm = unsafe { MainThreadMarker::new_unchecked() };
        let app = NSApplication::sharedApplication(mtm);

        let icon_path = icon_path.to_string_lossy();
        let icon_path = NSString::from_str(icon_path.as_ref());
        let image = unsafe { NSImage::initWithContentsOfFile(NSImage::alloc(), &icon_path) };
        if let Some(image) = image.as_deref() {
            unsafe { app.setApplicationIconImage(Some(image)) };
        }
    });
}
