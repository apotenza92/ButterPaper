//! Display information and refresh rate detection for macOS
//!
//! This module provides utilities for:
//! - Detecting display scale factor (Retina support)
//! - Detecting display refresh rate (ProMotion 120Hz support)
//! - Adapting frame timing for optimal performance

use std::time::Duration;

/// Information about the current display
#[derive(Debug, Clone, Copy)]
pub struct DisplayInfo {
    /// Scale factor for Retina displays (1.0 for standard, 2.0 for Retina)
    pub scale_factor: f64,
    /// Refresh rate in Hz (60, 120, etc.)
    pub refresh_rate_hz: u32,
    /// Target frame time based on refresh rate
    pub target_frame_time: Duration,
}

impl Default for DisplayInfo {
    fn default() -> Self {
        Self {
            scale_factor: 1.0,
            refresh_rate_hz: 60,
            target_frame_time: Duration::from_micros(1_000_000 / 60),
        }
    }
}

impl DisplayInfo {
    /// Create new display info with given scale factor and refresh rate
    pub fn new(scale_factor: f64, refresh_rate_hz: u32) -> Self {
        let effective_refresh = refresh_rate_hz.clamp(30, 240);
        Self {
            scale_factor,
            refresh_rate_hz: effective_refresh,
            target_frame_time: Duration::from_micros(1_000_000 / effective_refresh as u64),
        }
    }

    /// Update refresh rate and recalculate target frame time
    pub fn set_refresh_rate(&mut self, hz: u32) {
        let effective_refresh = hz.clamp(30, 240);
        self.refresh_rate_hz = effective_refresh;
        self.target_frame_time = Duration::from_micros(1_000_000 / effective_refresh as u64);
    }

    /// Update scale factor
    pub fn set_scale_factor(&mut self, scale: f64) {
        self.scale_factor = scale.clamp(1.0, 4.0);
    }

    /// Check if this is a high refresh rate display (>60Hz)
    pub fn is_high_refresh_rate(&self) -> bool {
        self.refresh_rate_hz > 60
    }

    /// Check if this is a Retina (HiDPI) display
    pub fn is_retina(&self) -> bool {
        self.scale_factor > 1.5
    }
}

/// Query the refresh rate for the main display on macOS
/// Returns the refresh rate in Hz, or 60 if unable to determine
#[cfg(target_os = "macos")]
pub fn get_main_display_refresh_rate() -> u32 {
    use objc::runtime::{Class, Object};
    use objc::{msg_send, sel, sel_impl};

    unsafe {
        // Get the main screen
        let ns_screen_class = Class::get("NSScreen").unwrap();
        let main_screen: *mut Object = msg_send![ns_screen_class, mainScreen];

        if main_screen.is_null() {
            return 60;
        }

        // Get the display ID from the screen's device description
        let device_desc: *mut Object = msg_send![main_screen, deviceDescription];
        if device_desc.is_null() {
            return 60;
        }

        // Get the NSScreenNumber key
        let screen_number_key: *mut Object =
            msg_send![Class::get("NSString").unwrap(), stringWithUTF8String: b"NSScreenNumber\0".as_ptr()];
        let screen_number: *mut Object = msg_send![device_desc, objectForKey: screen_number_key];

        if screen_number.is_null() {
            return 60;
        }

        let display_id: u32 = msg_send![screen_number, unsignedIntValue];

        // Use Core Graphics to get the display mode
        let mode = core_graphics::display::CGDisplayCopyDisplayMode(display_id);
        if mode.is_null() {
            return 60;
        }

        let refresh_rate = core_graphics::display::CGDisplayModeGetRefreshRate(mode);

        // CGDisplayModeRelease is needed to avoid leak
        core_graphics::display::CGDisplayModeRelease(mode);

        // ProMotion displays report 0 for variable refresh rate
        // In that case, assume 120Hz as the maximum
        if refresh_rate < 1.0 {
            120
        } else {
            refresh_rate as u32
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub fn get_main_display_refresh_rate() -> u32 {
    60 // Default for non-macOS platforms
}

/// Get the refresh rate from a monitor handle
pub fn get_monitor_refresh_rate(monitor: &winit::monitor::MonitorHandle) -> u32 {
    monitor
        .refresh_rate_millihertz()
        .map(|mhz| (mhz / 1000) as u32)
        .unwrap_or_else(get_main_display_refresh_rate)
}

/// Core Graphics FFI for display mode queries
#[cfg(target_os = "macos")]
mod core_graphics {
    pub mod display {
        use std::ffi::c_void;

        #[link(name = "CoreGraphics", kind = "framework")]
        extern "C" {
            pub fn CGDisplayCopyDisplayMode(display: u32) -> *mut c_void;
            pub fn CGDisplayModeGetRefreshRate(mode: *mut c_void) -> f64;
            pub fn CGDisplayModeRelease(mode: *mut c_void);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_info_default() {
        let info = DisplayInfo::default();
        assert_eq!(info.scale_factor, 1.0);
        assert_eq!(info.refresh_rate_hz, 60);
        assert_eq!(info.target_frame_time, Duration::from_micros(16666));
    }

    #[test]
    fn test_display_info_new_120hz() {
        let info = DisplayInfo::new(2.0, 120);
        assert_eq!(info.scale_factor, 2.0);
        assert_eq!(info.refresh_rate_hz, 120);
        assert_eq!(info.target_frame_time, Duration::from_micros(8333));
    }

    #[test]
    fn test_display_info_clamps_refresh_rate() {
        let info = DisplayInfo::new(1.0, 1000);
        assert_eq!(info.refresh_rate_hz, 240); // Clamped to max

        let info = DisplayInfo::new(1.0, 10);
        assert_eq!(info.refresh_rate_hz, 30); // Clamped to min
    }

    #[test]
    fn test_is_retina() {
        let standard = DisplayInfo::new(1.0, 60);
        assert!(!standard.is_retina());

        let retina = DisplayInfo::new(2.0, 60);
        assert!(retina.is_retina());

        let borderline = DisplayInfo::new(1.5, 60);
        assert!(!borderline.is_retina());

        let borderline_plus = DisplayInfo::new(1.6, 60);
        assert!(borderline_plus.is_retina());
    }

    #[test]
    fn test_is_high_refresh_rate() {
        let standard = DisplayInfo::new(1.0, 60);
        assert!(!standard.is_high_refresh_rate());

        let promotion = DisplayInfo::new(2.0, 120);
        assert!(promotion.is_high_refresh_rate());

        let borderline = DisplayInfo::new(1.0, 90);
        assert!(borderline.is_high_refresh_rate());
    }

    #[test]
    fn test_set_refresh_rate() {
        let mut info = DisplayInfo::default();
        info.set_refresh_rate(120);
        assert_eq!(info.refresh_rate_hz, 120);
        assert_eq!(info.target_frame_time, Duration::from_micros(8333));
    }

    #[test]
    fn test_set_scale_factor() {
        let mut info = DisplayInfo::default();
        info.set_scale_factor(2.0);
        assert_eq!(info.scale_factor, 2.0);

        // Test clamping
        info.set_scale_factor(0.5);
        assert_eq!(info.scale_factor, 1.0);

        info.set_scale_factor(5.0);
        assert_eq!(info.scale_factor, 4.0);
    }

    #[test]
    fn test_frame_time_calculation() {
        // 60 Hz = 16.666ms
        let info60 = DisplayInfo::new(1.0, 60);
        assert!(info60.target_frame_time.as_micros() >= 16666);
        assert!(info60.target_frame_time.as_micros() <= 16667);

        // 120 Hz = 8.333ms
        let info120 = DisplayInfo::new(1.0, 120);
        assert!(info120.target_frame_time.as_micros() >= 8333);
        assert!(info120.target_frame_time.as_micros() <= 8334);

        // 240 Hz = 4.166ms
        let info240 = DisplayInfo::new(1.0, 240);
        assert!(info240.target_frame_time.as_micros() >= 4166);
        assert!(info240.target_frame_time.as_micros() <= 4167);
    }
}
