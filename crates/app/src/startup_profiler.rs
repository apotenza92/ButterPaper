//! Startup Profiler
//!
//! Tracks detailed timing information for each phase of application startup.
//! Enable with the `--profile-startup` flag.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// Global flag to enable startup profiling
static PROFILING_ENABLED: AtomicBool = AtomicBool::new(false);

/// Enable startup profiling
pub fn enable_profiling() {
    PROFILING_ENABLED.store(true, Ordering::SeqCst);
}

/// Check if profiling is enabled
pub fn is_profiling_enabled() -> bool {
    PROFILING_ENABLED.load(Ordering::SeqCst)
}

/// Startup phase identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StartupPhase {
    /// Application entry point
    AppStart,
    /// Command line argument parsing
    ArgParsing,
    /// macOS platform setup (activation policy, etc.)
    PlatformSetup,
    /// Menu bar initialization
    MenuBarSetup,
    /// Event loop creation
    EventLoopCreation,
    /// App struct initialization
    AppStructInit,
    /// Window creation
    WindowCreation,
    /// Metal/GPU layer setup
    MetalSetup,
    /// GPU context creation
    GpuContextCreation,
    /// Scene renderer creation
    SceneRendererCreation,
    /// PDF file loading (if initial file provided)
    PdfLoading,
    /// Text layer extraction
    TextExtraction,
    /// First page render
    FirstPageRender,
    /// First frame rendered (GPU present)
    FirstFrameRendered,
}

impl StartupPhase {
    /// Get a human-readable name for the phase
    pub fn name(&self) -> &'static str {
        match self {
            StartupPhase::AppStart => "Application Start",
            StartupPhase::ArgParsing => "Argument Parsing",
            StartupPhase::PlatformSetup => "Platform Setup",
            StartupPhase::MenuBarSetup => "Menu Bar Setup",
            StartupPhase::EventLoopCreation => "Event Loop Creation",
            StartupPhase::AppStructInit => "App Struct Init",
            StartupPhase::WindowCreation => "Window Creation",
            StartupPhase::MetalSetup => "Metal Layer Setup",
            StartupPhase::GpuContextCreation => "GPU Context Creation",
            StartupPhase::SceneRendererCreation => "Scene Renderer Creation",
            StartupPhase::PdfLoading => "PDF Loading",
            StartupPhase::TextExtraction => "Text Extraction",
            StartupPhase::FirstPageRender => "First Page Render",
            StartupPhase::FirstFrameRendered => "First Frame Rendered",
        }
    }
}

/// Startup profiler that tracks timing for each phase
#[derive(Debug)]
pub struct StartupProfiler {
    /// The instant when the profiler was created (app start)
    start_time: Instant,
    /// Map of phase to its completion time (relative to start)
    phase_times: HashMap<StartupPhase, Duration>,
    /// Whether profiling output has been printed
    output_printed: bool,
}

impl Default for StartupProfiler {
    fn default() -> Self {
        Self::new()
    }
}

impl StartupProfiler {
    /// Create a new startup profiler
    pub fn new() -> Self {
        let profiler = Self {
            start_time: Instant::now(),
            phase_times: HashMap::new(),
            output_printed: false,
        };
        if is_profiling_enabled() {
            println!("STARTUP_PROFILE: Profiling enabled");
        }
        profiler
    }

    /// Mark a phase as complete and record its timestamp
    pub fn mark_phase(&mut self, phase: StartupPhase) {
        if !is_profiling_enabled() {
            return;
        }
        let elapsed = self.start_time.elapsed();
        self.phase_times.insert(phase, elapsed);

        // Print individual phase timing
        println!(
            "STARTUP_PROFILE: {} completed at {:.2}ms",
            phase.name(),
            elapsed.as_secs_f64() * 1000.0
        );
    }

    /// Get the duration since app start
    #[allow(dead_code)]
    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Get the total startup time (time to first frame)
    #[allow(dead_code)]
    pub fn total_startup_time(&self) -> Option<Duration> {
        self.phase_times.get(&StartupPhase::FirstFrameRendered).copied()
    }

    /// Check if first frame has been rendered
    #[allow(dead_code)]
    pub fn is_startup_complete(&self) -> bool {
        self.phase_times.contains_key(&StartupPhase::FirstFrameRendered)
    }

    /// Print the full startup profile summary
    pub fn print_summary(&mut self) {
        if !is_profiling_enabled() || self.output_printed {
            return;
        }
        self.output_printed = true;

        println!("\n=== STARTUP PROFILE SUMMARY ===");

        // Define the order of phases
        let phases = [
            StartupPhase::AppStart,
            StartupPhase::ArgParsing,
            StartupPhase::PlatformSetup,
            StartupPhase::MenuBarSetup,
            StartupPhase::EventLoopCreation,
            StartupPhase::AppStructInit,
            StartupPhase::WindowCreation,
            StartupPhase::MetalSetup,
            StartupPhase::GpuContextCreation,
            StartupPhase::SceneRendererCreation,
            StartupPhase::PdfLoading,
            StartupPhase::TextExtraction,
            StartupPhase::FirstPageRender,
            StartupPhase::FirstFrameRendered,
        ];

        let mut last_time = Duration::ZERO;

        for phase in phases {
            if let Some(time) = self.phase_times.get(&phase) {
                let delta = time.saturating_sub(last_time);
                println!(
                    "  {:30} {:>8.2}ms (delta: {:>6.2}ms)",
                    phase.name(),
                    time.as_secs_f64() * 1000.0,
                    delta.as_secs_f64() * 1000.0
                );
                last_time = *time;
            }
        }

        // Print total time
        if let Some(total) = self.total_startup_time() {
            println!("  {:30} {:>8.2}ms", "TOTAL", total.as_secs_f64() * 1000.0);

            // Check against targets
            let total_ms = total.as_secs_f64() * 1000.0;
            if total_ms < 200.0 {
                println!("\n  Target <200ms to first frame: PASS");
            } else {
                println!("\n  Target <200ms to first frame: FAIL ({:.0}ms)", total_ms);
            }

            // Check PDF visible time (if PDF was loaded)
            if let Some(pdf_time) = self.phase_times.get(&StartupPhase::FirstPageRender) {
                let pdf_ms = pdf_time.as_secs_f64() * 1000.0;
                if pdf_ms < 500.0 {
                    println!("  Target <500ms to first PDF page: PASS");
                } else {
                    println!("  Target <500ms to first PDF page: FAIL ({:.0}ms)", pdf_ms);
                }
            }
        }

        println!("===============================\n");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_profiler_creation() {
        let profiler = StartupProfiler::new();
        assert!(!profiler.is_startup_complete());
        assert!(profiler.total_startup_time().is_none());
    }

    #[test]
    #[serial]
    fn test_profiler_mark_phase_when_disabled() {
        // Ensure profiling is disabled
        PROFILING_ENABLED.store(false, Ordering::SeqCst);

        let mut profiler = StartupProfiler::new();
        profiler.mark_phase(StartupPhase::AppStart);

        // Phase should not be recorded when profiling is disabled
        assert!(!profiler.phase_times.contains_key(&StartupPhase::AppStart));
    }

    #[test]
    #[serial]
    fn test_profiler_mark_phase_when_enabled() {
        // Enable profiling for this test
        PROFILING_ENABLED.store(true, Ordering::SeqCst);

        let mut profiler = StartupProfiler::new();
        thread::sleep(Duration::from_millis(10));
        profiler.mark_phase(StartupPhase::AppStart);

        assert!(profiler.phase_times.contains_key(&StartupPhase::AppStart));
        let time = profiler.phase_times.get(&StartupPhase::AppStart).unwrap();
        assert!(time.as_millis() >= 10);

        // Reset flag
        PROFILING_ENABLED.store(false, Ordering::SeqCst);
    }

    #[test]
    fn test_profiler_elapsed() {
        let profiler = StartupProfiler::new();
        thread::sleep(Duration::from_millis(5));
        let elapsed = profiler.elapsed();
        assert!(elapsed.as_millis() >= 5);
    }

    #[test]
    #[serial]
    fn test_profiler_startup_complete() {
        PROFILING_ENABLED.store(true, Ordering::SeqCst);

        let mut profiler = StartupProfiler::new();
        assert!(!profiler.is_startup_complete());

        profiler.mark_phase(StartupPhase::FirstFrameRendered);
        assert!(profiler.is_startup_complete());
        assert!(profiler.total_startup_time().is_some());

        PROFILING_ENABLED.store(false, Ordering::SeqCst);
    }

    #[test]
    fn test_phase_names() {
        assert_eq!(StartupPhase::AppStart.name(), "Application Start");
        assert_eq!(StartupPhase::WindowCreation.name(), "Window Creation");
        assert_eq!(StartupPhase::FirstFrameRendered.name(), "First Frame Rendered");
    }

    #[test]
    #[serial]
    fn test_enable_profiling() {
        PROFILING_ENABLED.store(false, Ordering::SeqCst);
        assert!(!is_profiling_enabled());

        enable_profiling();
        assert!(is_profiling_enabled());

        // Reset
        PROFILING_ENABLED.store(false, Ordering::SeqCst);
    }
}
