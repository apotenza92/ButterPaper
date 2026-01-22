import { useEffect, useCallback, useState } from 'react';
import { usePdfState } from './hooks/usePdfState';
import { Toolbar } from './components/Toolbar';
import { Sidebar } from './components/Sidebar';
import { Viewport } from './components/Viewport';
import { SearchBar } from './components/SearchBar';
import { ErrorDialog } from './components/ErrorDialog';

function App() {
  const {
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
  } = usePdfState();

  // Search state (UI-only for Spec 03, actual search in Spec 04)
  const [searchMatchCount] = useState(0);
  const [searchCurrentMatch] = useState(0);

  const handleSearch = useCallback((_query: string) => {
    // Search implementation will be added in Spec 04
  }, []);

  const handleSearchNext = useCallback(() => {
    // Search implementation will be added in Spec 04
  }, []);

  const handleSearchPrev = useCallback(() => {
    // Search implementation will be added in Spec 04
  }, []);

  // Keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      const isMeta = e.metaKey || e.ctrlKey;

      // Ctrl/Cmd+F: Open search
      if (isMeta && e.key === 'f') {
        e.preventDefault();
        openSearch();
        return;
      }

      // Ctrl/Cmd+O: Open file
      if (isMeta && e.key === 'o') {
        e.preventDefault();
        openFile();
        return;
      }

      // Escape: Close dialogs/search
      if (e.key === 'Escape') {
        if (state.isSearchOpen) {
          closeSearch();
        }
        return;
      }

      // Only handle the following if no PDF is open
      if (!state.filePath) return;

      // Left/Right arrow: Navigate pages
      if (e.key === 'ArrowLeft') {
        prevPage();
        return;
      }
      if (e.key === 'ArrowRight') {
        nextPage();
        return;
      }

      // Ctrl/Cmd++: Zoom in
      if (isMeta && (e.key === '=' || e.key === '+')) {
        e.preventDefault();
        setZoom(Math.min(500, state.zoomPercent + 25));
        return;
      }

      // Ctrl/Cmd+-: Zoom out
      if (isMeta && e.key === '-') {
        e.preventDefault();
        setZoom(Math.max(10, state.zoomPercent - 25));
        return;
      }

      // Ctrl/Cmd+0: Reset zoom to 100%
      if (isMeta && e.key === '0') {
        e.preventDefault();
        setZoom(100);
        return;
      }
    };

    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [
    state.isSearchOpen,
    state.filePath,
    state.zoomPercent,
    openSearch,
    closeSearch,
    openFile,
    prevPage,
    nextPage,
    setZoom,
  ]);

  return (
    <div className="flex flex-col h-screen">
      {/* SearchBar (fixed overlay at top) */}
      <SearchBar
        isOpen={state.isSearchOpen}
        onClose={closeSearch}
        onSearch={handleSearch}
        onNext={handleSearchNext}
        onPrev={handleSearchPrev}
        matchCount={searchMatchCount}
        currentMatch={searchCurrentMatch}
      />

      {/* Toolbar */}
      <Toolbar
        currentPage={state.currentPage}
        pageCount={state.pageCount}
        zoomPercent={state.zoomPercent}
        fitMode={state.fitMode}
        activeTool={state.activeTool}
        onPageChange={goToPage}
        onZoomChange={setZoom}
        onFitModeChange={setFitMode}
        onToolChange={setTool}
        onOpenFile={openFile}
        onSearchOpen={openSearch}
      />

      {/* Main content area */}
      <div className="flex flex-1 overflow-hidden">
        {/* Sidebar */}
        <Sidebar
          pageCount={state.pageCount}
          currentPage={state.currentPage}
          filePath={state.filePath}
          onPageSelect={goToPage}
        />

        {/* Viewport */}
        <Viewport
          filePath={state.filePath}
          currentPage={state.currentPage}
          zoomPercent={state.zoomPercent}
          fitMode={state.fitMode}
          activeTool={state.activeTool}
        />
      </div>

      {/* Error Dialog */}
      <ErrorDialog
        isOpen={state.error !== null}
        message={state.error ?? ''}
        onClose={clearError}
      />
    </div>
  );
}

export default App;
