import { useState, useCallback } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import { openPdf, navigatePage, setZoom as ipcSetZoom } from '../lib/tauri';

export type Tool = 'select' | 'hand' | 'text' | 'highlight' | 'comment';
export type FitMode = 'fit-page' | 'fit-width' | 'actual-size';

export interface PdfState {
  filePath: string | null;
  pageCount: number;
  currentPage: number;        // 0-indexed
  zoomPercent: number;        // 100 = 100%
  fitMode: FitMode;
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
  setTool: (tool: Tool) => void;
  openSearch: () => void;
  closeSearch: () => void;
  clearError: () => void;
}

const initialState: PdfState = {
  filePath: null,
  pageCount: 0,
  currentPage: 0,
  zoomPercent: 100,
  fitMode: 'fit-page',
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

  return {
    state,
    openFile,
    goToPage,
    nextPage,
    prevPage,
    setZoom,
    setFitMode,
    setTool,
    openSearch,
    closeSearch,
    clearError,
  };
}
