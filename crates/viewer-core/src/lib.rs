use std::collections::{HashMap, VecDeque};
use std::hash::Hash;
use std::ops::RangeInclusive;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Continuous,
    SinglePage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZoomMode {
    Percent,
    FitPage,
    FitWidth,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ViewportState {
    pub mode: ViewMode,
    pub zoom_mode: ZoomMode,
    pub zoom_percent: u16,
    pub viewport_width_px: f32,
    pub viewport_height_px: f32,
    pub dpr: f32,
    pub scroll_offset_px: f32,
    pub page_heights_px: Vec<f32>,
    pub page_spacing_px: f32,
}

impl Default for ViewportState {
    fn default() -> Self {
        Self {
            mode: ViewMode::Continuous,
            zoom_mode: ZoomMode::FitPage,
            zoom_percent: 100,
            viewport_width_px: 1280.0,
            viewport_height_px: 800.0,
            dpr: 1.0,
            scroll_offset_px: 0.0,
            page_heights_px: vec![1000.0],
            page_spacing_px: 16.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RenderKind {
    Page,
    Thumbnail,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RenderPriority {
    Visible,
    Thumbnail,
    Prefetch,
}

impl RenderPriority {
    fn rank(self) -> u8 {
        match self {
            Self::Visible => 0,
            Self::Thumbnail => 1,
            Self::Prefetch => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RenderJobKey {
    pub document_id: u64,
    pub page_index: u32,
    pub zoom_percent: u16,
    pub kind: RenderKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderJob {
    pub key: RenderJobKey,
    pub priority: RenderPriority,
    pub generation: u64,
}

#[derive(Debug, Default)]
pub struct RenderQueue {
    generation: u64,
    pending: HashMap<RenderJobKey, (RenderPriority, u64)>,
    order: VecDeque<RenderJobKey>,
}

impl RenderQueue {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn begin_generation(&mut self) -> u64 {
        self.generation += 1;
        self.pending.clear();
        self.order.clear();
        self.generation
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn enqueue(&mut self, key: RenderJobKey, priority: RenderPriority) {
        match self.pending.get_mut(&key) {
            Some((existing_priority, _)) => {
                if priority.rank() < existing_priority.rank() {
                    *existing_priority = priority;
                }
            }
            None => {
                let generation = self.generation;
                self.pending.insert(key, (priority, generation));
                self.order.push_back(key);
            }
        }
    }

    pub fn pop_next(&mut self) -> Option<RenderJob> {
        let mut best: Option<(RenderJobKey, RenderPriority, u64)> = None;

        for key in &self.order {
            let Some((priority, generation)) = self.pending.get(key).copied() else {
                continue;
            };

            match best {
                Some((_, best_priority, _)) if priority.rank() >= best_priority.rank() => {}
                _ => best = Some((*key, priority, generation)),
            }

            if matches!(best, Some((_, RenderPriority::Visible, _))) {
                break;
            }
        }

        let (key, priority, generation) = best?;
        self.pending.remove(&key);

        if let Some(index) = self.order.iter().position(|candidate| *candidate == key) {
            let _ = self.order.remove(index);
        }

        Some(RenderJob { key, priority, generation })
    }

    pub fn len(&self) -> usize {
        self.pending.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }
}

#[derive(Debug, Clone)]
pub struct LruCache<K, V>
where
    K: Eq + Hash + Clone,
{
    capacity: usize,
    map: HashMap<K, V>,
    order: VecDeque<K>,
}

impl<K, V> LruCache<K, V>
where
    K: Eq + Hash + Clone,
{
    pub fn new(capacity: usize) -> Self {
        Self { capacity: capacity.max(1), map: HashMap::new(), order: VecDeque::new() }
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn contains_key(&self, key: &K) -> bool {
        self.map.contains_key(key)
    }

    pub fn peek(&self, key: &K) -> Option<&V> {
        self.map.get(key)
    }

    pub fn get(&mut self, key: &K) -> Option<&V> {
        if self.map.contains_key(key) {
            self.touch(key);
        }

        self.map.get(key)
    }

    pub fn insert(&mut self, key: K, value: V) {
        let existed = self.map.insert(key.clone(), value).is_some();

        if existed {
            self.touch(&key);
            return;
        }

        self.order.push_back(key);

        while self.map.len() > self.capacity {
            if let Some(oldest) = self.order.pop_front() {
                self.map.remove(&oldest);
            }
        }
    }

    fn touch(&mut self, key: &K) {
        if let Some(index) = self.order.iter().position(|existing| existing == key) {
            let Some(found) = self.order.remove(index) else {
                return;
            };
            self.order.push_back(found);
        }
    }
}

pub fn prefetch_page_indices(current_page_index: u32, page_count: u32, radius: u32) -> Vec<u32> {
    if page_count == 0 {
        return Vec::new();
    }

    let max = page_count.saturating_sub(1);
    let mut pages = Vec::new();

    for offset in 1..=radius {
        if let Some(lower) = current_page_index.checked_sub(offset) {
            pages.push(lower.min(max));
        }

        let upper = current_page_index.saturating_add(offset);
        if upper <= max {
            pages.push(upper);
        }
    }

    pages
}

pub fn fit_width_percent(viewport_width_px: f32, page_width_px: f32, dpr: f32) -> u16 {
    if viewport_width_px <= 0.0 || page_width_px <= 0.0 || dpr <= 0.0 {
        return 100;
    }

    ((viewport_width_px / (page_width_px * dpr)) * 100.0).round().clamp(10.0, 1600.0) as u16
}

pub fn fit_page_percent(
    viewport_width_px: f32,
    viewport_height_px: f32,
    page_width_px: f32,
    page_height_px: f32,
    dpr: f32,
) -> u16 {
    if viewport_width_px <= 0.0
        || viewport_height_px <= 0.0
        || page_width_px <= 0.0
        || page_height_px <= 0.0
        || dpr <= 0.0
    {
        return 100;
    }

    let width = viewport_width_px / (page_width_px * dpr);
    let height = viewport_height_px / (page_height_px * dpr);

    (width.min(height) * 100.0).round().clamp(10.0, 1600.0) as u16
}

pub fn visible_pages(state: &ViewportState) -> RangeInclusive<u32> {
    if state.page_heights_px.is_empty() {
        return 0..=0;
    }

    let start = page_at_offset(state.scroll_offset_px.max(0.0), state);
    let end = page_at_offset((state.scroll_offset_px + state.viewport_height_px).max(0.0), state);

    start..=end
}

pub fn current_page_from_viewport(state: &ViewportState) -> u32 {
    if state.page_heights_px.is_empty() {
        return 0;
    }

    let center_offset = (state.scroll_offset_px + state.viewport_height_px / 2.0).max(0.0);
    page_at_offset(center_offset, state)
}

pub fn switch_mode_preserving_page(
    state: &mut ViewportState,
    target_mode: ViewMode,
    current_page: u32,
) {
    state.mode = target_mode;

    if state.page_heights_px.is_empty() {
        state.scroll_offset_px = 0.0;
        return;
    }

    let target_page = current_page.min((state.page_heights_px.len() as u32).saturating_sub(1));
    state.scroll_offset_px = page_start_offset(target_page, state);
}

fn page_at_offset(offset: f32, state: &ViewportState) -> u32 {
    let mut cursor = 0.0;

    for (index, page_height) in state.page_heights_px.iter().enumerate() {
        let page_end = cursor + page_height;
        if offset <= page_end {
            return index as u32;
        }

        cursor = page_end + state.page_spacing_px;
    }

    state.page_heights_px.len().saturating_sub(1) as u32
}

fn page_start_offset(page_index: u32, state: &ViewportState) -> f32 {
    let mut cursor = 0.0;

    for (index, page_height) in state.page_heights_px.iter().enumerate() {
        if index as u32 == page_index {
            return cursor;
        }
        cursor += page_height + state.page_spacing_px;
    }

    cursor
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fit_width_respects_expected_scale() {
        let percent = fit_width_percent(1000.0, 500.0, 1.0);
        assert_eq!(percent, 200);

        let clamped = fit_width_percent(100_000.0, 100.0, 1.0);
        assert_eq!(clamped, 1600);
    }

    #[test]
    fn fit_page_uses_smallest_dimension_ratio() {
        let percent = fit_page_percent(1000.0, 800.0, 500.0, 2000.0, 1.0);
        assert_eq!(percent, 40);
    }

    #[test]
    fn visible_range_tracks_scroll_window() {
        let state = ViewportState {
            viewport_height_px: 900.0,
            scroll_offset_px: 1100.0,
            page_heights_px: vec![1000.0, 1000.0, 1000.0],
            page_spacing_px: 100.0,
            ..ViewportState::default()
        };

        assert_eq!(visible_pages(&state), 1..=1);

        let shifted = ViewportState { scroll_offset_px: 1500.0, ..state };

        assert_eq!(visible_pages(&shifted), 1..=2);
    }

    #[test]
    fn current_page_uses_viewport_center() {
        let state = ViewportState {
            viewport_height_px: 1000.0,
            scroll_offset_px: 1200.0,
            page_heights_px: vec![1000.0, 1000.0, 1000.0],
            page_spacing_px: 100.0,
            ..ViewportState::default()
        };

        assert_eq!(current_page_from_viewport(&state), 1);
    }

    #[test]
    fn mode_switch_preserves_requested_page_offset() {
        let mut state = ViewportState {
            page_heights_px: vec![1000.0, 1000.0, 1000.0],
            page_spacing_px: 100.0,
            ..ViewportState::default()
        };

        switch_mode_preserving_page(&mut state, ViewMode::SinglePage, 2);
        assert_eq!(state.mode, ViewMode::SinglePage);
        assert_eq!(state.scroll_offset_px, 2200.0);
    }

    #[test]
    fn render_queue_prioritizes_visible_before_prefetch() {
        let mut queue = RenderQueue::new();
        queue.begin_generation();

        queue.enqueue(
            RenderJobKey {
                document_id: 1,
                page_index: 4,
                zoom_percent: 100,
                kind: RenderKind::Page,
            },
            RenderPriority::Prefetch,
        );

        queue.enqueue(
            RenderJobKey {
                document_id: 1,
                page_index: 1,
                zoom_percent: 100,
                kind: RenderKind::Page,
            },
            RenderPriority::Visible,
        );

        let first = queue.pop_next().expect("first render job expected");
        assert_eq!(first.priority, RenderPriority::Visible);
        assert_eq!(first.key.page_index, 1);

        let second = queue.pop_next().expect("second render job expected");
        assert_eq!(second.priority, RenderPriority::Prefetch);
        assert_eq!(second.key.page_index, 4);
    }

    #[test]
    fn render_queue_upgrades_priority_for_existing_jobs() {
        let mut queue = RenderQueue::new();
        queue.begin_generation();

        let key = RenderJobKey {
            document_id: 2,
            page_index: 8,
            zoom_percent: 125,
            kind: RenderKind::Page,
        };

        queue.enqueue(key, RenderPriority::Prefetch);
        queue.enqueue(key, RenderPriority::Visible);

        let job = queue.pop_next().expect("render job expected");
        assert_eq!(job.key, key);
        assert_eq!(job.priority, RenderPriority::Visible);
        assert!(queue.is_empty());
    }

    #[test]
    fn lru_cache_evicts_oldest_entry() {
        let mut cache = LruCache::new(2);

        cache.insert(1_u32, "one");
        cache.insert(2_u32, "two");
        cache.insert(3_u32, "three");

        assert!(!cache.contains_key(&1));
        assert!(cache.contains_key(&2));
        assert!(cache.contains_key(&3));
    }

    #[test]
    fn lru_cache_refreshes_recently_accessed_entry() {
        let mut cache = LruCache::new(2);

        cache.insert(1_u32, "one");
        cache.insert(2_u32, "two");

        let _ = cache.get(&1);
        cache.insert(3_u32, "three");

        assert!(cache.contains_key(&1));
        assert!(!cache.contains_key(&2));
        assert!(cache.contains_key(&3));
    }

    #[test]
    fn prefetch_neighbors_are_symmetric_and_bounded() {
        let pages = prefetch_page_indices(5, 10, 2);
        assert_eq!(pages, vec![4, 6, 3, 7]);

        let edge = prefetch_page_indices(0, 3, 3);
        assert_eq!(edge, vec![1, 2]);
    }
}
