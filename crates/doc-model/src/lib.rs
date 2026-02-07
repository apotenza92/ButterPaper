use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViewMode {
    Continuous,
    SinglePage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ZoomMode {
    Percent,
    FitPage,
    FitWidth,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZoomAction {
    ActualSize100,
    FitPage,
    FitWidth,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReaderState {
    pub view_mode: ViewMode,
    pub zoom_mode: ZoomMode,
    pub zoom_percent: u16,
}

impl Default for ReaderState {
    fn default() -> Self {
        Self { view_mode: ViewMode::Continuous, zoom_mode: ZoomMode::FitPage, zoom_percent: 100 }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageSize {
    pub width_pt: u32,
    pub height_pt: u32,
}

impl Default for PageSize {
    fn default() -> Self {
        Self { width_pt: 612, height_pt: 792 }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct DocumentId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TabId(pub u64);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentState {
    pub id: DocumentId,
    pub title: String,
    pub path: PathBuf,
    pub page_count: u32,
    pub first_page_size: PageSize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TabContent {
    Welcome,
    Document { document_id: DocumentId },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TabState {
    pub id: TabId,
    pub title: String,
    pub content: TabContent,
    pub reader: ReaderState,
    pub current_page: u32,
}

impl TabState {
    pub fn new_welcome(id: TabId) -> Self {
        Self {
            id,
            title: "Welcome".to_owned(),
            content: TabContent::Welcome,
            reader: ReaderState::default(),
            current_page: 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Preferences {
    pub prefer_tabs: bool,
    pub show_tab_bar: bool,
    pub allow_window_merge: bool,
}

impl Default for Preferences {
    fn default() -> Self {
        Self { prefer_tabs: true, show_tab_bar: true, allow_window_merge: true }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionState {
    pub tabs: Vec<TabState>,
    pub active_tab: Option<TabId>,
    pub documents: BTreeMap<DocumentId, DocumentState>,
    pub preferences: Preferences,
    next_document_id: u64,
    next_tab_id: u64,
}

impl Default for SessionState {
    fn default() -> Self {
        let welcome = TabState::new_welcome(TabId(1));

        Self {
            tabs: vec![welcome],
            active_tab: Some(TabId(1)),
            documents: BTreeMap::new(),
            preferences: Preferences::default(),
            next_document_id: 1,
            next_tab_id: 1,
        }
    }
}

impl SessionState {
    pub fn active_tab_mut(&mut self) -> Option<&mut TabState> {
        let active = self.active_tab?;
        self.tabs.iter_mut().find(|tab| tab.id == active)
    }

    pub fn active_tab(&self) -> Option<&TabState> {
        let active = self.active_tab?;
        self.tabs.iter().find(|tab| tab.id == active)
    }

    pub fn active_document(&self) -> Option<&DocumentState> {
        let tab = self.active_tab()?;
        let TabContent::Document { document_id } = tab.content else {
            return None;
        };

        self.documents.get(&document_id)
    }

    pub fn new_tab_id(&mut self) -> TabId {
        self.next_tab_id += 1;
        TabId(self.next_tab_id)
    }

    pub fn new_document_id(&mut self) -> DocumentId {
        self.next_document_id += 1;
        DocumentId(self.next_document_id)
    }

    pub fn is_welcome_only(&self) -> bool {
        matches!(self.tabs.as_slice(), [tab] if matches!(tab.content, TabContent::Welcome))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionAction {
    OpenDocument { path: PathBuf, title: String, page_count: u32, first_page_size: PageSize },
    NewWelcomeTab,
    CloseTab { tab_id: TabId },
    ActivateTab { tab_id: TabId },
    SetViewMode { tab_id: TabId, mode: ViewMode },
    SetZoomMode { tab_id: TabId, mode: ZoomMode },
    SetZoomPercent { tab_id: TabId, zoom_percent: u16 },
    SetCurrentPage { tab_id: TabId, page: u32 },
    NextPage { tab_id: TabId },
    PreviousPage { tab_id: TabId },
}

pub fn apply_zoom_action(state: &mut ReaderState, action: ZoomAction) {
    match action {
        ZoomAction::ActualSize100 => {
            state.zoom_mode = ZoomMode::Percent;
            state.zoom_percent = 100;
        }
        ZoomAction::FitPage => state.zoom_mode = ZoomMode::FitPage,
        ZoomAction::FitWidth => state.zoom_mode = ZoomMode::FitWidth,
    }
}

pub fn apply_session_action(state: &mut SessionState, action: SessionAction) {
    match action {
        SessionAction::OpenDocument { path, title, page_count, first_page_size } => {
            let document_id = state.new_document_id();
            let document = DocumentState {
                id: document_id,
                title: title.clone(),
                path,
                page_count,
                first_page_size,
            };
            state.documents.insert(document_id, document);

            if state.is_welcome_only() {
                if let Some(welcome) = state.tabs.first_mut() {
                    welcome.title = title;
                    welcome.content = TabContent::Document { document_id };
                    welcome.current_page = 1;
                }
                state.active_tab = state.tabs.first().map(|tab| tab.id);
            } else {
                let tab_id = state.new_tab_id();
                state.tabs.push(TabState {
                    id: tab_id,
                    title,
                    content: TabContent::Document { document_id },
                    reader: ReaderState::default(),
                    current_page: 1,
                });
                state.active_tab = Some(tab_id);
            }
        }
        SessionAction::NewWelcomeTab => {
            let tab_id = state.new_tab_id();
            state.tabs.push(TabState::new_welcome(tab_id));
            state.active_tab = Some(tab_id);
        }
        SessionAction::CloseTab { tab_id } => {
            let Some(index) = state.tabs.iter().position(|tab| tab.id == tab_id) else {
                return;
            };

            state.tabs.remove(index);

            if state.tabs.is_empty() {
                state.tabs.push(TabState::new_welcome(TabId(1)));
                state.active_tab = Some(TabId(1));
                state.documents.clear();
                state.next_document_id = 1;
                state.next_tab_id = 1;
                return;
            }

            let fallback_index = index.saturating_sub(1).min(state.tabs.len() - 1);
            state.active_tab = Some(state.tabs[fallback_index].id);

            let referenced_documents: BTreeMap<DocumentId, ()> = state
                .tabs
                .iter()
                .filter_map(|tab| match tab.content {
                    TabContent::Document { document_id } => Some((document_id, ())),
                    TabContent::Welcome => None,
                })
                .collect();

            state.documents.retain(|id, _| referenced_documents.contains_key(id));
        }
        SessionAction::ActivateTab { tab_id } => {
            if state.tabs.iter().any(|tab| tab.id == tab_id) {
                state.active_tab = Some(tab_id);
            }
        }
        SessionAction::SetViewMode { tab_id, mode } => {
            if let Some(tab) = state.tabs.iter_mut().find(|tab| tab.id == tab_id) {
                tab.reader.view_mode = mode;
            }
        }
        SessionAction::SetZoomMode { tab_id, mode } => {
            if let Some(tab) = state.tabs.iter_mut().find(|tab| tab.id == tab_id) {
                tab.reader.zoom_mode = mode;
            }
        }
        SessionAction::SetZoomPercent { tab_id, zoom_percent } => {
            if let Some(tab) = state.tabs.iter_mut().find(|tab| tab.id == tab_id) {
                tab.reader.zoom_mode = ZoomMode::Percent;
                tab.reader.zoom_percent = zoom_percent.clamp(10, 1600);
            }
        }
        SessionAction::SetCurrentPage { tab_id, page } => {
            if let Some(index) = state.tabs.iter().position(|tab| tab.id == tab_id) {
                let page_count = tab_page_count_by_index(state, index).unwrap_or(1);
                state.tabs[index].current_page = page.max(1).min(page_count.max(1));
            }
        }
        SessionAction::NextPage { tab_id } => {
            if let Some(index) = state.tabs.iter().position(|tab| tab.id == tab_id) {
                let page_count = tab_page_count_by_index(state, index).unwrap_or(1);
                state.tabs[index].current_page =
                    (state.tabs[index].current_page + 1).min(page_count);
            }
        }
        SessionAction::PreviousPage { tab_id } => {
            if let Some(index) = state.tabs.iter().position(|tab| tab.id == tab_id) {
                state.tabs[index].current_page =
                    state.tabs[index].current_page.saturating_sub(1).max(1);
            }
        }
    }
}

fn tab_page_count_by_index(state: &SessionState, tab_index: usize) -> Option<u32> {
    let tab = state.tabs.get(tab_index)?;
    let TabContent::Document { document_id } = tab.content else {
        return None;
    };

    state.documents.get(&document_id).map(|document| document.page_count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actual_size_forces_manual_percent_at_100_in_both_modes() {
        let mut continuous = ReaderState {
            view_mode: ViewMode::Continuous,
            zoom_mode: ZoomMode::FitPage,
            zoom_percent: 143,
        };
        apply_zoom_action(&mut continuous, ZoomAction::ActualSize100);
        assert_eq!(continuous.zoom_mode, ZoomMode::Percent);
        assert_eq!(continuous.zoom_percent, 100);

        let mut single = ReaderState {
            view_mode: ViewMode::SinglePage,
            zoom_mode: ZoomMode::FitWidth,
            zoom_percent: 66,
        };
        apply_zoom_action(&mut single, ZoomAction::ActualSize100);
        assert_eq!(single.zoom_mode, ZoomMode::Percent);
        assert_eq!(single.zoom_percent, 100);
    }

    #[test]
    fn open_first_document_replaces_welcome_tab() {
        let mut state = SessionState::default();
        apply_session_action(
            &mut state,
            SessionAction::OpenDocument {
                path: PathBuf::from("/tmp/test.pdf"),
                title: "test.pdf".to_owned(),
                page_count: 4,
                first_page_size: PageSize::default(),
            },
        );

        assert_eq!(state.tabs.len(), 1);
        assert!(matches!(state.tabs[0].content, TabContent::Document { .. }));
        assert_eq!(state.active_tab, Some(state.tabs[0].id));
    }

    #[test]
    fn opening_second_document_creates_new_tab_and_activates_it() {
        let mut state = SessionState::default();
        for i in 0..2 {
            apply_session_action(
                &mut state,
                SessionAction::OpenDocument {
                    path: PathBuf::from(format!("/tmp/test-{i}.pdf")),
                    title: format!("test-{i}.pdf"),
                    page_count: 2,
                    first_page_size: PageSize::default(),
                },
            );
        }

        assert_eq!(state.tabs.len(), 2);
        let active = state.active_tab.expect("active tab expected");
        assert_eq!(active, state.tabs[1].id);
    }

    #[test]
    fn closing_last_tab_creates_welcome_tab() {
        let mut state = SessionState::default();

        let tab_id = state.tabs[0].id;
        apply_session_action(&mut state, SessionAction::CloseTab { tab_id });

        assert_eq!(state.tabs.len(), 1);
        assert!(matches!(state.tabs[0].content, TabContent::Welcome));
    }

    #[test]
    fn zoom_percent_is_clamped() {
        let mut state = SessionState::default();
        let tab_id = state.active_tab.expect("active tab expected");

        apply_session_action(&mut state, SessionAction::SetZoomPercent { tab_id, zoom_percent: 1 });
        assert_eq!(state.tabs[0].reader.zoom_percent, 10);

        apply_session_action(
            &mut state,
            SessionAction::SetZoomPercent { tab_id, zoom_percent: 9999 },
        );
        assert_eq!(state.tabs[0].reader.zoom_percent, 1600);
    }

    #[test]
    fn next_page_is_clamped_to_document_bounds() {
        let mut state = SessionState::default();
        apply_session_action(
            &mut state,
            SessionAction::OpenDocument {
                path: PathBuf::from("/tmp/test.pdf"),
                title: "test.pdf".to_owned(),
                page_count: 2,
                first_page_size: PageSize::default(),
            },
        );

        let tab_id = state.active_tab.expect("active tab expected");
        apply_session_action(&mut state, SessionAction::NextPage { tab_id });
        apply_session_action(&mut state, SessionAction::NextPage { tab_id });
        apply_session_action(&mut state, SessionAction::NextPage { tab_id });

        let tab = state.active_tab().expect("active tab expected");
        assert_eq!(tab.current_page, 2);
    }

    #[test]
    fn set_current_page_is_clamped_to_document_bounds() {
        let mut state = SessionState::default();
        apply_session_action(
            &mut state,
            SessionAction::OpenDocument {
                path: PathBuf::from("/tmp/test.pdf"),
                title: "test.pdf".to_owned(),
                page_count: 3,
                first_page_size: PageSize::default(),
            },
        );

        let tab_id = state.active_tab.expect("active tab expected");
        apply_session_action(&mut state, SessionAction::SetCurrentPage { tab_id, page: 100 });

        let tab = state.active_tab().expect("active tab expected");
        assert_eq!(tab.current_page, 3);
    }
}
