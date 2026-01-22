import { useState, useCallback, useEffect, useRef } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import { openPdf, navigatePage, setZoom as ipcSetZoom } from '../lib/tauri';

export type Tool = 'select' | 'hand' | 'text' | 'highlight' | 'comment';
export type FitMode = 'fit-page' | 'fit-width' | 'actual-size';
export type ViewMode = 'continuous' | 'single-page';

export interface PdfState {
  filePath: string | null;
  pageCount: number;
  currentPage: number;        // 0-indexed
  zoomPercent: number;        // 100 = 100%
  fitMode: FitMode;
  viewMode: ViewMode;
  activeTool: Tool;
  isSearchOpen: boolean;
  error: string | null;
}

export interface UsePdfStateReturn {
  state: PdfState;
  openFile: () => Promise<void>;
  goToPage: (page: number) => Promise<void>;
  nextPage: () => Promise<void>;
  prevPage: () => Promise<void>;
  setZoom: (percent: number) => Promise<void>;
  setFitMode: (mode: FitMode) => void;
  setViewMode: (mode: ViewMode) => void;
  setTool: (tool: Tool) => void;
  openSearch: () => void;
  closeSearch: () => void;
  clearError: () => void;
}

// LocalStorage key for state persistence
const STORAGE_KEY = 'pdf-editor-state';

interface PersistedState {
  lastFilePath?: string;
  lastPage?: number;
  zoomPercent?: number;
  fitMode?: FitMode;
  viewMode?: ViewMode;
}

function loadPersistedState(): PersistedState {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored) {
      return JSON.parse(stored);
    }
  } catch {
    // Ignore parse errors
  }
  return {};
}

function savePersistedState(state: PersistedState): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
  } catch {
    // Ignore storage errors
  }
}

const persisted = loadPersistedState();

const initialState: PdfState = {
  filePath: null,
  pageCount: 0,
  currentPage: 0,
  zoomPercent: persisted.zoomPercent ?? 100,
  fitMode: persisted.fitMode ?? 'fit-page',
  viewMode: persisted.viewMode ?? 'continuous',
  activeTool: 'select',
  isSearchOpen: false,
  error: null,
};

export function usePdfState(): UsePdfStateReturn {
  const [state, setState] = useState<PdfState>(initialState);

  const openFile = useCallback(async () => {
    try {
      const selected = await open({
        filters: [{ name: 'PDF', extensions: ['pdf'] }],
        multiple: false,
      });
      if (selected) {
        const response = await openPdf(selected as string);
        if (response.success) {
          setState(prev => ({
            ...prev,
            filePath: selected as string,
            pageCount: response.page_count,
            currentPage: 0,
            error: null,
          }));
        } else {
          setState(prev => ({ ...prev, error: response.error ?? 'Failed to open PDF' }));
        }
      }
    } catch (err) {
      setState(prev => ({ ...prev, error: String(err) }));
    }
  }, []);

  const goToPage = useCallback(async (page: number) => {
    if (page < 0 || page >= state.pageCount) return;
    try {
      const response = await navigatePage(page);
      if (response.success) {
        setState(prev => ({ ...prev, currentPage: response.current_page, error: null }));
      } else {
        setState(prev => ({ ...prev, error: response.error ?? 'Navigation failed' }));
      }
    } catch (err) {
      setState(prev => ({ ...prev, error: String(err) }));
    }
  }, [state.pageCount]);

  const nextPage = useCallback(async () => {
    if (state.currentPage < state.pageCount - 1) {
      await goToPage(state.currentPage + 1);
    }
  }, [state.currentPage, state.pageCount, goToPage]);

  const prevPage = useCallback(async () => {
    if (state.currentPage > 0) {
      await goToPage(state.currentPage - 1);
    }
  }, [state.currentPage, goToPage]);

  const setZoom = useCallback(async (percent: number) => {
    const clampedPercent = Math.min(500, Math.max(10, percent));
    try {
      const response = await ipcSetZoom(clampedPercent);
      if (response.success) {
        setState(prev => ({ ...prev, zoomPercent: response.zoom_percent, error: null }));
      } else {
        setState(prev => ({ ...prev, error: response.error ?? 'Zoom failed' }));
      }
    } catch (err) {
      setState(prev => ({ ...prev, error: String(err) }));
    }
  }, []);

  const setFitMode = useCallback((mode: FitMode) => {
    setState(prev => ({ ...prev, fitMode: mode }));
  }, []);

  const setViewMode = useCallback((mode: ViewMode) => {
    setState(prev => ({ ...prev, viewMode: mode }));
  }, []);

  const setTool = useCallback((tool: Tool) => {
    setState(prev => ({ ...prev, activeTool: tool }));
  }, []);

  const openSearch = useCallback(() => {
    setState(prev => ({ ...prev, isSearchOpen: true }));
  }, []);

  const closeSearch = useCallback(() => {
    setState(prev => ({ ...prev, isSearchOpen: false }));
  }, []);

  const clearError = useCallback(() => {
    setState(prev => ({ ...prev, error: null }));
  }, []);

  // Persist state changes to localStorage
  const isInitialMount = useRef(true);
  useEffect(() => {
    // Skip saving on initial mount
    if (isInitialMount.current) {
      isInitialMount.current = false;
      return;
    }

    savePersistedState({
      lastFilePath: state.filePath ?? undefined,
      lastPage: state.currentPage,
      zoomPercent: state.zoomPercent,
      fitMode: state.fitMode,
      viewMode: state.viewMode,
    });
  }, [state.filePath, state.currentPage, state.zoomPercent, state.fitMode, state.viewMode]);

  // Restore last opened file on mount
  const hasRestoredRef = useRef(false);
  useEffect(() => {
    if (hasRestoredRef.current) return;
    hasRestoredRef.current = true;

    const persisted = loadPersistedState();
    if (persisted.lastFilePath) {
      // Attempt to restore last file
      openPdf(persisted.lastFilePath).then(response => {
        if (response.success) {
          const restoredPage = Math.min(
            persisted.lastPage ?? 0,
            response.page_count - 1
          );
          setState(prev => ({
            ...prev,
            filePath: persisted.lastFilePath!,
            pageCount: response.page_count,
            currentPage: Math.max(0, restoredPage),
            error: null,
          }));
          // Navigate to restored page via IPC
          if (restoredPage > 0) {
            navigatePage(restoredPage).catch(() => {});
          }
        }
      }).catch(() => {
        // Silently fail if file no longer exists
      });
    }
  }, []);

  return {
    state,
    openFile,
    goToPage,
    nextPage,
    prevPage,
    setZoom,
    setFitMode,
    setViewMode,
    setTool,
    openSearch,
    closeSearch,
    clearError,
  };
}
