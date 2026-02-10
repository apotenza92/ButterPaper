//! macOS-specific functionality for appearance management

use std::path::PathBuf;

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
    // Bundled apps should use the icon embedded in the app bundle (Info.plist + Resources/Assets.car
    // and/or .icns). Runtime overrides are for dev runs and can accidentally force the wrong icon
    // (e.g., stable icon in a beta bundle).
    if is_bundled_app_with_assets_catalog() {
        return;
    }

    // Tahoe-style icon appearance uses asset-catalog metadata. Runtime icon overrides
    // force a static image and can bypass system appearance icon styles.
    if is_tahoe_or_newer() {
        return;
    }

    Queue::main().exec_async(move || {
        #[cfg(feature = "beta")]
        const ICON_BYTES: &[u8] =
            include_bytes!("../assets/app-icons/butterpaper-icon-beta.icns");
        #[cfg(not(feature = "beta"))]
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

fn is_bundled_app_with_assets_catalog() -> bool {
    let exe: PathBuf = match std::env::current_exe() {
        Ok(p) => p,
        Err(_) => return false,
    };

    // In a macOS app bundle, the executable lives at:
    //   <App>.app/Contents/MacOS/<exe>
    // and the asset catalog is at:
    //   <App>.app/Contents/Resources/Assets.car
    let macos_dir = match exe.parent() {
        Some(p) => p,
        None => return false,
    };
    if macos_dir.file_name().and_then(|s| s.to_str()) != Some("MacOS") {
        return false;
    }

    let contents_dir = match macos_dir.parent() {
        Some(p) => p,
        None => return false,
    };
    if contents_dir.file_name().and_then(|s| s.to_str()) != Some("Contents") {
        return false;
    }

    contents_dir.join("Resources").join("Assets.car").is_file()
}
