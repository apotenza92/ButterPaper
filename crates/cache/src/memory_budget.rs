//! Memory budget tracking for bounded memory usage
//!
//! This module provides unified memory budget tracking across all cache tiers
//! (RAM, GPU, Disk) to ensure memory usage stays bounded under all conditions.
//! It includes memory pressure detection and proactive eviction triggers.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use crate::{CacheStats, DiskCacheStats, GpuCacheStats};

/// Memory pressure level indicating cache health
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MemoryPressure {
    /// Memory usage is low (< 50% utilization)
    Low,
    /// Memory usage is moderate (50-75% utilization)
    Moderate,
    /// Memory usage is high (75-90% utilization)
    High,
    /// Memory usage is critical (> 90% utilization)
    Critical,
}

impl MemoryPressure {
    /// Get the memory pressure level from a utilization ratio (0.0 to 1.0)
    pub fn from_utilization(utilization: f64) -> Self {
        if utilization < 0.5 {
            MemoryPressure::Low
        } else if utilization < 0.75 {
            MemoryPressure::Moderate
        } else if utilization < 0.90 {
            MemoryPressure::High
        } else {
            MemoryPressure::Critical
        }
    }

    /// Returns true if memory pressure requires action (High or Critical)
    pub fn needs_eviction(&self) -> bool {
        matches!(self, MemoryPressure::High | MemoryPressure::Critical)
    }
}

/// Configuration for memory budget thresholds
#[derive(Debug, Clone, Copy)]
pub struct MemoryBudgetConfig {
    /// Total memory budget in bytes across all caches
    pub total_budget: usize,
    /// Warning threshold (0.0 to 1.0) - triggers warning when exceeded
    pub warning_threshold: f64,
    /// Critical threshold (0.0 to 1.0) - triggers aggressive eviction
    pub critical_threshold: f64,
    /// Target utilization after eviction (0.0 to 1.0)
    pub target_utilization: f64,
}

impl Default for MemoryBudgetConfig {
    fn default() -> Self {
        Self {
            // Default total budget: 768 MB (256 RAM + 512 GPU)
            total_budget: 768 * 1024 * 1024,
            warning_threshold: 0.85,
            critical_threshold: 0.95,
            target_utilization: 0.80,
        }
    }
}

impl MemoryBudgetConfig {
    /// Create a new memory budget configuration
    pub fn new(total_budget_mb: usize) -> Self {
        Self {
            total_budget: total_budget_mb * 1024 * 1024,
            ..Default::default()
        }
    }

    /// Set the warning threshold (0.0 to 1.0)
    pub fn with_warning_threshold(mut self, threshold: f64) -> Self {
        self.warning_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    /// Set the critical threshold (0.0 to 1.0)
    pub fn with_critical_threshold(mut self, threshold: f64) -> Self {
        self.critical_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    /// Set the target utilization after eviction (0.0 to 1.0)
    pub fn with_target_utilization(mut self, target: f64) -> Self {
        self.target_utilization = target.clamp(0.0, 1.0);
        self
    }

    /// Get the warning threshold in bytes
    pub fn warning_bytes(&self) -> usize {
        (self.total_budget as f64 * self.warning_threshold) as usize
    }

    /// Get the critical threshold in bytes
    pub fn critical_bytes(&self) -> usize {
        (self.total_budget as f64 * self.critical_threshold) as usize
    }

    /// Get the target bytes after eviction
    pub fn target_bytes(&self) -> usize {
        (self.total_budget as f64 * self.target_utilization) as usize
    }
}

/// Memory budget tracker for bounded memory usage
///
/// Tracks total memory usage across RAM and GPU caches (disk cache is not
/// counted as it uses persistent storage). Provides memory pressure detection
/// and eviction recommendations.
///
/// # Example
///
/// ```
/// use pdf_editor_cache::memory_budget::{MemoryBudget, MemoryBudgetConfig};
///
/// // Create a budget with 500MB total limit
/// let config = MemoryBudgetConfig::new(500);
/// let budget = MemoryBudget::new(config);
///
/// // Check if we can allocate memory
/// if budget.can_allocate(10 * 1024 * 1024) {
///     // Safe to allocate 10MB
///     budget.record_allocation(10 * 1024 * 1024);
/// }
///
/// // Check memory pressure
/// let pressure = budget.pressure();
/// if pressure.needs_eviction() {
///     // Should trigger cache eviction
/// }
/// ```
#[derive(Debug)]
pub struct MemoryBudget {
    config: MemoryBudgetConfig,
    /// Current total memory usage (RAM + GPU)
    current_usage: AtomicUsize,
}

impl MemoryBudget {
    /// Create a new memory budget with the given configuration
    pub fn new(config: MemoryBudgetConfig) -> Self {
        Self {
            config,
            current_usage: AtomicUsize::new(0),
        }
    }

    /// Create a memory budget with default configuration
    pub fn with_default_config() -> Self {
        Self::new(MemoryBudgetConfig::default())
    }

    /// Create a memory budget with a total limit in megabytes
    pub fn with_limit_mb(total_mb: usize) -> Self {
        Self::new(MemoryBudgetConfig::new(total_mb))
    }

    /// Get the current memory usage in bytes
    pub fn current_usage(&self) -> usize {
        self.current_usage.load(Ordering::Relaxed)
    }

    /// Get the total budget in bytes
    pub fn total_budget(&self) -> usize {
        self.config.total_budget
    }

    /// Get the available memory in bytes
    pub fn available(&self) -> usize {
        let current = self.current_usage();
        self.config.total_budget.saturating_sub(current)
    }

    /// Get the current utilization ratio (0.0 to 1.0)
    pub fn utilization(&self) -> f64 {
        if self.config.total_budget == 0 {
            0.0
        } else {
            self.current_usage() as f64 / self.config.total_budget as f64
        }
    }

    /// Get the current memory pressure level
    pub fn pressure(&self) -> MemoryPressure {
        MemoryPressure::from_utilization(self.utilization())
    }

    /// Check if the given allocation can fit within the budget
    ///
    /// Returns true if allocating `bytes` would not exceed the total budget.
    pub fn can_allocate(&self, bytes: usize) -> bool {
        let current = self.current_usage();
        current.saturating_add(bytes) <= self.config.total_budget
    }

    /// Check if the given allocation would trigger memory pressure
    ///
    /// Returns true if allocating `bytes` would exceed the warning threshold.
    pub fn would_trigger_pressure(&self, bytes: usize) -> bool {
        let current = self.current_usage();
        current.saturating_add(bytes) > self.config.warning_bytes()
    }

    /// Record a memory allocation
    pub fn record_allocation(&self, bytes: usize) {
        self.current_usage.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Record a memory deallocation
    pub fn record_deallocation(&self, bytes: usize) {
        self.current_usage
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
                Some(current.saturating_sub(bytes))
            })
            .ok();
    }

    /// Set the current usage directly (for synchronization with cache stats)
    pub fn set_usage(&self, bytes: usize) {
        self.current_usage.store(bytes, Ordering::Relaxed);
    }

    /// Calculate how many bytes need to be evicted to reach target utilization
    ///
    /// Returns 0 if current usage is already below target.
    pub fn bytes_to_evict(&self) -> usize {
        let current = self.current_usage();
        let target = self.config.target_bytes();
        current.saturating_sub(target)
    }

    /// Check if eviction is needed based on current pressure
    pub fn needs_eviction(&self) -> bool {
        self.pressure().needs_eviction()
    }

    /// Get the configuration
    pub fn config(&self) -> &MemoryBudgetConfig {
        &self.config
    }

    /// Update the total budget
    pub fn set_total_budget(&mut self, bytes: usize) {
        self.config.total_budget = bytes;
    }
}

impl Default for MemoryBudget {
    fn default() -> Self {
        Self::with_default_config()
    }
}

impl Clone for MemoryBudget {
    fn clone(&self) -> Self {
        Self {
            config: self.config,
            current_usage: AtomicUsize::new(self.current_usage.load(Ordering::Relaxed)),
        }
    }
}

/// Aggregated statistics from all cache tiers
#[derive(Debug, Clone, Default)]
pub struct AggregatedCacheStats {
    /// RAM cache statistics
    pub ram: CacheStats,
    /// GPU cache statistics
    pub gpu: GpuCacheStats,
    /// Disk cache statistics
    pub disk: DiskCacheStats,
}

impl AggregatedCacheStats {
    /// Get total memory usage (RAM + GPU) in bytes
    pub fn total_memory_used(&self) -> usize {
        self.ram.memory_used + self.gpu.vram_used
    }

    /// Get total memory limit (RAM + GPU) in bytes
    pub fn total_memory_limit(&self) -> usize {
        self.ram.memory_limit + self.gpu.vram_limit
    }

    /// Get overall memory utilization (0.0 to 1.0)
    pub fn memory_utilization(&self) -> f64 {
        let limit = self.total_memory_limit();
        if limit == 0 {
            0.0
        } else {
            self.total_memory_used() as f64 / limit as f64
        }
    }

    /// Get overall hit rate (weighted average of RAM and GPU)
    pub fn overall_hit_rate(&self) -> f64 {
        let total_hits = self.ram.hits + self.gpu.hits;
        let total_misses = self.ram.misses + self.gpu.misses;
        let total = total_hits + total_misses;
        if total == 0 {
            0.0
        } else {
            total_hits as f64 / total as f64
        }
    }

    /// Get total evictions across all caches
    pub fn total_evictions(&self) -> u64 {
        self.ram.evictions + self.gpu.evictions + self.disk.evictions
    }

    /// Get total tile/texture count
    pub fn total_cached_items(&self) -> usize {
        self.ram.tile_count + self.gpu.texture_count + self.disk.tile_count
    }
}

/// Cache monitor for aggregating statistics across all cache tiers
///
/// Provides a unified view of cache health and memory usage across
/// RAM, GPU, and disk caches.
///
/// # Example
///
/// ```
/// use pdf_editor_cache::{RamTileCache, GpuTextureCache, DiskTileCache};
/// use pdf_editor_cache::memory_budget::CacheMonitor;
///
/// let ram_cache = RamTileCache::with_mb_limit(256);
/// let gpu_cache = GpuTextureCache::with_mb_limit(512);
/// // let disk_cache = DiskTileCache::new(...);
///
/// let monitor = CacheMonitor::new();
/// let stats = monitor.aggregate_stats(&ram_cache, &gpu_cache, None);
///
/// println!("Total memory: {} MB", stats.total_memory_used() / (1024 * 1024));
/// println!("Utilization: {:.1}%", stats.memory_utilization() * 100.0);
/// ```
#[derive(Debug, Clone)]
pub struct CacheMonitor {
    /// Memory budget for tracking overall usage
    budget: Arc<MemoryBudget>,
}

impl CacheMonitor {
    /// Create a new cache monitor with default budget
    pub fn new() -> Self {
        Self {
            budget: Arc::new(MemoryBudget::default()),
        }
    }

    /// Create a cache monitor with a custom memory budget
    pub fn with_budget(budget: MemoryBudget) -> Self {
        Self {
            budget: Arc::new(budget),
        }
    }

    /// Create a cache monitor with a total memory limit in MB
    pub fn with_limit_mb(total_mb: usize) -> Self {
        Self {
            budget: Arc::new(MemoryBudget::with_limit_mb(total_mb)),
        }
    }

    /// Get a reference to the memory budget
    pub fn budget(&self) -> &MemoryBudget {
        &self.budget
    }

    /// Aggregate statistics from all cache tiers
    pub fn aggregate_stats(
        &self,
        ram_cache: &crate::RamTileCache,
        gpu_cache: &crate::GpuTextureCache,
        disk_cache: Option<&crate::DiskTileCache>,
    ) -> AggregatedCacheStats {
        let ram = ram_cache.stats();
        let gpu = gpu_cache.stats();
        let disk = disk_cache.map(|c| c.stats()).unwrap_or_default();

        // Update budget with current usage
        let total_usage = ram.memory_used + gpu.vram_used;
        self.budget.set_usage(total_usage);

        AggregatedCacheStats { ram, gpu, disk }
    }

    /// Get the current memory pressure level
    pub fn pressure(&self) -> MemoryPressure {
        self.budget.pressure()
    }

    /// Check if any cache tier needs eviction
    pub fn needs_eviction(&self) -> bool {
        self.budget.needs_eviction()
    }

    /// Calculate recommended eviction amounts for each cache tier
    ///
    /// Returns (ram_bytes_to_evict, gpu_bytes_to_evict)
    pub fn recommend_eviction(&self, stats: &AggregatedCacheStats) -> (usize, usize) {
        let total_to_evict = self.budget.bytes_to_evict();

        if total_to_evict == 0 {
            return (0, 0);
        }

        // Distribute eviction proportionally based on current usage
        let ram_ratio = if stats.total_memory_used() == 0 {
            0.5
        } else {
            stats.ram.memory_used as f64 / stats.total_memory_used() as f64
        };

        let ram_evict = (total_to_evict as f64 * ram_ratio) as usize;
        let gpu_evict = total_to_evict - ram_evict;

        (ram_evict, gpu_evict)
    }
}

impl Default for CacheMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of a bounded memory check
#[derive(Debug, Clone, Copy)]
pub struct MemoryCheckResult {
    /// Whether the allocation can proceed
    pub can_proceed: bool,
    /// Current memory pressure level
    pub pressure: MemoryPressure,
    /// Bytes that need to be evicted first (0 if can_proceed is true)
    pub bytes_to_evict_first: usize,
}

impl MemoryBudget {
    /// Check if an allocation can proceed and what eviction is needed
    pub fn check_allocation(&self, bytes: usize) -> MemoryCheckResult {
        let current = self.current_usage();
        let after_allocation = current.saturating_add(bytes);
        let pressure = MemoryPressure::from_utilization(
            after_allocation as f64 / self.config.total_budget as f64,
        );

        if after_allocation <= self.config.total_budget {
            MemoryCheckResult {
                can_proceed: true,
                pressure,
                bytes_to_evict_first: 0,
            }
        } else {
            let bytes_to_evict = after_allocation - self.config.target_bytes();
            MemoryCheckResult {
                can_proceed: false,
                pressure: MemoryPressure::Critical,
                bytes_to_evict_first: bytes_to_evict,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_pressure_levels() {
        assert_eq!(
            MemoryPressure::from_utilization(0.3),
            MemoryPressure::Low
        );
        assert_eq!(
            MemoryPressure::from_utilization(0.6),
            MemoryPressure::Moderate
        );
        assert_eq!(
            MemoryPressure::from_utilization(0.8),
            MemoryPressure::High
        );
        assert_eq!(
            MemoryPressure::from_utilization(0.95),
            MemoryPressure::Critical
        );
    }

    #[test]
    fn test_pressure_needs_eviction() {
        assert!(!MemoryPressure::Low.needs_eviction());
        assert!(!MemoryPressure::Moderate.needs_eviction());
        assert!(MemoryPressure::High.needs_eviction());
        assert!(MemoryPressure::Critical.needs_eviction());
    }

    #[test]
    fn test_memory_budget_config() {
        let config = MemoryBudgetConfig::new(500);
        assert_eq!(config.total_budget, 500 * 1024 * 1024);
        assert_eq!(config.warning_threshold, 0.85);
        assert_eq!(config.critical_threshold, 0.95);

        let config = config
            .with_warning_threshold(0.8)
            .with_critical_threshold(0.95)
            .with_target_utilization(0.6);

        assert_eq!(config.warning_threshold, 0.8);
        assert_eq!(config.critical_threshold, 0.95);
        assert_eq!(config.target_utilization, 0.6);
    }

    #[test]
    fn test_memory_budget_basic() {
        let budget = MemoryBudget::with_limit_mb(100);

        assert_eq!(budget.total_budget(), 100 * 1024 * 1024);
        assert_eq!(budget.current_usage(), 0);
        assert_eq!(budget.available(), 100 * 1024 * 1024);
        assert_eq!(budget.utilization(), 0.0);
        assert_eq!(budget.pressure(), MemoryPressure::Low);
    }

    #[test]
    fn test_memory_budget_allocation() {
        let budget = MemoryBudget::with_limit_mb(100);
        let mb = 1024 * 1024;

        // Allocate 50MB
        assert!(budget.can_allocate(50 * mb));
        budget.record_allocation(50 * mb);

        assert_eq!(budget.current_usage(), 50 * mb);
        assert_eq!(budget.available(), 50 * mb);
        assert_eq!(budget.utilization(), 0.5);
        assert_eq!(budget.pressure(), MemoryPressure::Moderate);

        // Deallocate 20MB
        budget.record_deallocation(20 * mb);
        assert_eq!(budget.current_usage(), 30 * mb);
        assert_eq!(budget.available(), 70 * mb);
    }

    #[test]
    fn test_memory_budget_pressure_detection() {
        let budget = MemoryBudget::with_limit_mb(100);
        let mb = 1024 * 1024;

        // Low pressure
        budget.set_usage(30 * mb);
        assert_eq!(budget.pressure(), MemoryPressure::Low);
        assert!(!budget.needs_eviction());

        // Moderate pressure
        budget.set_usage(60 * mb);
        assert_eq!(budget.pressure(), MemoryPressure::Moderate);
        assert!(!budget.needs_eviction());

        // High pressure
        budget.set_usage(80 * mb);
        assert_eq!(budget.pressure(), MemoryPressure::High);
        assert!(budget.needs_eviction());

        // Critical pressure
        budget.set_usage(95 * mb);
        assert_eq!(budget.pressure(), MemoryPressure::Critical);
        assert!(budget.needs_eviction());
    }

    #[test]
    fn test_memory_budget_eviction_calculation() {
        let config = MemoryBudgetConfig::new(100).with_target_utilization(0.70);
        let budget = MemoryBudget::new(config);
        let mb = 1024 * 1024;

        // At 90% usage, should evict to 70%
        budget.set_usage(90 * mb);
        let to_evict = budget.bytes_to_evict();
        assert_eq!(to_evict, 20 * mb); // 90MB - 70MB = 20MB

        // At 50% usage, no eviction needed
        budget.set_usage(50 * mb);
        assert_eq!(budget.bytes_to_evict(), 0);
    }

    #[test]
    fn test_memory_budget_check_allocation() {
        let budget = MemoryBudget::with_limit_mb(100);
        let mb = 1024 * 1024;

        budget.set_usage(50 * mb);

        // Can allocate 30MB (total 80MB < 100MB limit)
        let result = budget.check_allocation(30 * mb);
        assert!(result.can_proceed);
        assert_eq!(result.bytes_to_evict_first, 0);

        // Cannot allocate 60MB (total 110MB > 100MB limit)
        let result = budget.check_allocation(60 * mb);
        assert!(!result.can_proceed);
        assert_eq!(result.pressure, MemoryPressure::Critical);
        assert!(result.bytes_to_evict_first > 0);
    }

    #[test]
    fn test_aggregated_stats() {
        let mut stats = AggregatedCacheStats::default();

        stats.ram.memory_used = 100 * 1024 * 1024;
        stats.ram.memory_limit = 256 * 1024 * 1024;
        stats.ram.hits = 80;
        stats.ram.misses = 20;
        stats.ram.evictions = 10;
        stats.ram.tile_count = 50;

        stats.gpu.vram_used = 200 * 1024 * 1024;
        stats.gpu.vram_limit = 512 * 1024 * 1024;
        stats.gpu.hits = 120;
        stats.gpu.misses = 30;
        stats.gpu.evictions = 5;
        stats.gpu.texture_count = 80;

        stats.disk.evictions = 2;
        stats.disk.tile_count = 200;

        assert_eq!(stats.total_memory_used(), 300 * 1024 * 1024);
        assert_eq!(stats.total_memory_limit(), 768 * 1024 * 1024);
        assert!((stats.memory_utilization() - 0.390625).abs() < 0.001);
        assert_eq!(stats.overall_hit_rate(), 0.8); // 200/250
        assert_eq!(stats.total_evictions(), 17);
        assert_eq!(stats.total_cached_items(), 330);
    }

    #[test]
    fn test_cache_monitor_recommend_eviction() {
        let monitor = CacheMonitor::with_limit_mb(500);
        let mb = 1024 * 1024;

        // Set up stats with 90% usage
        let mut stats = AggregatedCacheStats::default();
        stats.ram.memory_used = 200 * mb;
        stats.gpu.vram_used = 250 * mb;

        // Update monitor with this usage
        monitor.budget.set_usage(450 * mb);

        // Get eviction recommendation
        let (ram_evict, gpu_evict) = monitor.recommend_eviction(&stats);

        // Should recommend evicting proportionally
        assert!(ram_evict > 0);
        assert!(gpu_evict > 0);

        // RAM has ~44% of usage, GPU has ~56%
        // So GPU eviction should be larger
        assert!(gpu_evict >= ram_evict);
    }

    #[test]
    fn test_memory_budget_thread_safety() {
        use std::sync::Arc;
        use std::thread;

        let budget = Arc::new(MemoryBudget::with_limit_mb(100));
        let mb = 1024 * 1024;

        let handles: Vec<_> = (0..4)
            .map(|_| {
                let budget_clone = Arc::clone(&budget);
                thread::spawn(move || {
                    for _ in 0..100 {
                        budget_clone.record_allocation(mb);
                        budget_clone.record_deallocation(mb);
                    }
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        // After all allocations and deallocations, usage should be 0
        assert_eq!(budget.current_usage(), 0);
    }

    // ============================================================================
    // Large PDF Memory Bounded Tests (Phase 4.2)
    // ============================================================================

    #[test]
    fn test_memory_bounded_for_500_page_pdf() {
        let budget = MemoryBudget::with_limit_mb(100);
        let tile_size = 256 * 256 * 4; // 256KB per tile
        let tiles_per_page = 12;

        // Simulate loading tiles for 500 pages
        for page in 0..500 {
            for _tile in 0..tiles_per_page {
                // Check if we can allocate
                let check = budget.check_allocation(tile_size);

                if check.can_proceed {
                    budget.record_allocation(tile_size);
                } else {
                    // Would need to evict first - simulate eviction
                    let to_evict = check.bytes_to_evict_first;
                    budget.record_deallocation(to_evict.min(budget.current_usage()));
                    budget.record_allocation(tile_size);
                }
            }

            // Memory should never exceed budget
            assert!(
                budget.current_usage() <= budget.total_budget(),
                "Memory exceeded at page {}: {} > {}",
                page,
                budget.current_usage(),
                budget.total_budget()
            );
        }

        // Final check: memory is bounded
        assert!(budget.current_usage() <= budget.total_budget());
    }

    #[test]
    fn test_memory_bounded_for_100mb_pdf() {
        // 100MB PDF typically has 1000+ pages
        let budget = MemoryBudget::with_limit_mb(256);
        let tile_size = 256 * 256 * 4;
        let tiles_per_page = 12;

        // Simulate navigation through 1000 pages
        for page in 0..1000 {
            for _tile in 0..tiles_per_page {
                let check = budget.check_allocation(tile_size);

                if check.can_proceed {
                    budget.record_allocation(tile_size);
                } else {
                    // Simulate eviction to target level
                    let target = budget.config().target_bytes();
                    if budget.current_usage() > target {
                        budget.record_deallocation(budget.current_usage() - target);
                    }
                    budget.record_allocation(tile_size);
                }
            }

            // Verify bounded at each page
            assert!(
                budget.current_usage() <= budget.total_budget(),
                "Memory exceeded at page {}: {} > {}",
                page,
                budget.current_usage(),
                budget.total_budget()
            );
        }
    }

    #[test]
    fn test_memory_pressure_triggers_eviction() {
        let config = MemoryBudgetConfig::new(100)
            .with_warning_threshold(0.85)
            .with_critical_threshold(0.95);
        let budget = MemoryBudget::new(config);
        let mb = 1024 * 1024;

        // Fill to 70% - no eviction needed (below High threshold 75%)
        budget.set_usage(70 * mb);
        assert!(!budget.needs_eviction());

        // Fill to 80% - high pressure (75-90%), needs eviction
        budget.set_usage(80 * mb);
        assert!(budget.needs_eviction());
        assert_eq!(budget.pressure(), MemoryPressure::High);

        // Fill to 97% - critical pressure
        budget.set_usage(97 * mb);
        assert!(budget.needs_eviction());
        assert_eq!(budget.pressure(), MemoryPressure::Critical);
    }

    #[test]
    fn test_eviction_brings_usage_to_target() {
        let config = MemoryBudgetConfig::new(100).with_target_utilization(0.70);
        let budget = MemoryBudget::new(config);
        let mb = 1024 * 1024;

        // Fill to 95%
        budget.set_usage(95 * mb);

        // Calculate eviction needed
        let to_evict = budget.bytes_to_evict();
        assert_eq!(to_evict, 25 * mb); // 95 - 70 = 25MB

        // Simulate eviction
        budget.record_deallocation(to_evict);

        // Should be at target
        assert_eq!(budget.current_usage(), 70 * mb);
        assert_eq!(budget.pressure(), MemoryPressure::Moderate);
    }

    #[test]
    fn test_concurrent_memory_tracking() {
        use std::sync::Arc;
        use std::thread;

        let budget = Arc::new(MemoryBudget::with_limit_mb(500));
        let tile_size = 256 * 256 * 4;

        // Simulate 4 threads loading tiles concurrently
        let handles: Vec<_> = (0..4)
            .map(|thread_id| {
                let budget_clone = Arc::clone(&budget);
                thread::spawn(move || {
                    for i in 0..250 {
                        // Each thread loads 250 tiles
                        let check = budget_clone.check_allocation(tile_size);

                        if check.can_proceed {
                            budget_clone.record_allocation(tile_size);
                        } else {
                            // Simulate eviction - free 10% of budget
                            let to_free = budget_clone.total_budget() / 10;
                            budget_clone.record_deallocation(to_free);
                            budget_clone.record_allocation(tile_size);
                        }

                        // Verify bounded
                        assert!(
                            budget_clone.current_usage() <= budget_clone.total_budget() + tile_size,
                            "Thread {} iteration {}: Memory exceeded",
                            thread_id,
                            i
                        );
                    }
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        // Memory should be bounded (with some tolerance for concurrent updates)
        let usage = budget.current_usage();
        let limit = budget.total_budget();
        assert!(
            usage <= limit + 4 * 256 * 256 * 4, // Allow for 4 in-flight tiles
            "Final memory exceeded: {} > {}",
            usage,
            limit
        );
    }
}
