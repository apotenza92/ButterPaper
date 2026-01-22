import { useState, useCallback, KeyboardEvent } from 'react';
import { FitMode, Tool } from '../hooks/usePdfState';

interface ToolbarProps {
  currentPage: number;
  pageCount: number;
  zoomPercent: number;
  fitMode: FitMode;
  activeTool: Tool;
  onPageChange: (page: number) => void;
  onZoomChange: (percent: number) => void;
  onFitModeChange: (mode: FitMode) => void;
  onToolChange: (tool: Tool) => void;
  onOpenFile: () => void;
  onSearchOpen: () => void;
}

const ZOOM_STEP = 25;
const MIN_ZOOM = 10;
const MAX_ZOOM = 500;

export function Toolbar({
  currentPage,
  pageCount,
  zoomPercent,
  fitMode,
  activeTool,
  onPageChange,
  onZoomChange,
  onFitModeChange,
  onToolChange,
  onOpenFile,
  onSearchOpen,
}: ToolbarProps) {
  const [pageInput, setPageInput] = useState<string>(String(currentPage + 1));
  const hasPdf = pageCount > 0;

  // Sync page input when currentPage changes externally
  const displayPage = String(currentPage + 1);
  if (pageInput !== displayPage && !document.activeElement?.matches('input[data-page-input]')) {
    setPageInput(displayPage);
  }

  const handlePageInputChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    setPageInput(e.target.value);
  }, []);

  const handlePageInputKeyDown = useCallback((e: KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Enter') {
      const parsed = parseInt(pageInput, 10);
      if (!isNaN(parsed) && parsed >= 1 && parsed <= pageCount) {
        onPageChange(parsed - 1); // Convert to 0-indexed
      } else {
        setPageInput(String(currentPage + 1)); // Reset to current page
      }
    }
  }, [pageInput, pageCount, currentPage, onPageChange]);

  const handlePageInputBlur = useCallback(() => {
    setPageInput(String(currentPage + 1)); // Reset on blur
  }, [currentPage]);

  const handleZoomIn = useCallback(() => {
    if (zoomPercent < MAX_ZOOM) {
      onZoomChange(Math.min(MAX_ZOOM, zoomPercent + ZOOM_STEP));
    }
  }, [zoomPercent, onZoomChange]);

  const handleZoomOut = useCallback(() => {
    if (zoomPercent > MIN_ZOOM) {
      onZoomChange(Math.max(MIN_ZOOM, zoomPercent - ZOOM_STEP));
    }
  }, [zoomPercent, onZoomChange]);

  const buttonClass = (disabled: boolean) =>
    `px-3 py-1 rounded hover:bg-gray-700 ${disabled ? 'opacity-50 cursor-not-allowed' : ''}`;

  const toolButtonClass = (tool: Tool) =>
    `px-3 py-1 rounded hover:bg-gray-700 ${activeTool === tool ? 'bg-gray-600' : ''}`;

  return (
    <header className="h-12 bg-gray-800 text-white flex items-center px-4 gap-1">
      {/* File section */}
      <button
        className="px-3 py-1 rounded hover:bg-gray-700"
        onClick={onOpenFile}
        aria-label="Open PDF file"
      >
        Open
      </button>

      <div className="border-l border-gray-600 mx-2 h-6" />

      {/* Navigation section */}
      <button
        className={buttonClass(!hasPdf || currentPage <= 0)}
        onClick={() => onPageChange(currentPage - 1)}
        disabled={!hasPdf || currentPage <= 0}
        aria-label="Previous page"
      >
        &lt;
      </button>
      <span className="flex items-center gap-1">
        <span className="text-sm">Page</span>
        <input
          data-page-input
          type="text"
          className="w-12 px-2 py-1 rounded bg-gray-700 text-white text-center text-sm"
          value={pageInput}
          onChange={handlePageInputChange}
          onKeyDown={handlePageInputKeyDown}
          onBlur={handlePageInputBlur}
          disabled={!hasPdf}
          aria-label="Current page number"
        />
        <span className="text-sm">of {pageCount || 0}</span>
      </span>
      <button
        className={buttonClass(!hasPdf || currentPage >= pageCount - 1)}
        onClick={() => onPageChange(currentPage + 1)}
        disabled={!hasPdf || currentPage >= pageCount - 1}
        aria-label="Next page"
      >
        &gt;
      </button>

      <div className="border-l border-gray-600 mx-2 h-6" />

      {/* Zoom section */}
      <select
        className="px-2 py-1 rounded bg-gray-700 text-white text-sm"
        value={fitMode}
        onChange={(e) => onFitModeChange(e.target.value as FitMode)}
        disabled={!hasPdf}
        aria-label="Fit mode"
      >
        <option value="fit-page">Fit Page</option>
        <option value="fit-width">Fit Width</option>
        <option value="actual-size">Actual Size</option>
      </select>
      <button
        className={buttonClass(!hasPdf || zoomPercent <= MIN_ZOOM)}
        onClick={handleZoomOut}
        disabled={!hasPdf || zoomPercent <= MIN_ZOOM}
        aria-label="Zoom out"
      >
        &minus;
      </button>
      <button
        className={buttonClass(!hasPdf || zoomPercent >= MAX_ZOOM)}
        onClick={handleZoomIn}
        disabled={!hasPdf || zoomPercent >= MAX_ZOOM}
        aria-label="Zoom in"
      >
        +
      </button>

      <div className="border-l border-gray-600 mx-2 h-6" />

      {/* Tools section */}
      <button
        className={toolButtonClass('select')}
        onClick={() => onToolChange('select')}
        disabled={!hasPdf}
        aria-label="Select tool"
      >
        Select
      </button>
      <button
        className={toolButtonClass('hand')}
        onClick={() => onToolChange('hand')}
        disabled={!hasPdf}
        aria-label="Hand tool"
      >
        Hand
      </button>
      <button
        className={toolButtonClass('text')}
        onClick={() => onToolChange('text')}
        disabled={!hasPdf}
        aria-label="Text tool"
      >
        Text
      </button>
      <button
        className={toolButtonClass('highlight')}
        onClick={() => onToolChange('highlight')}
        disabled={!hasPdf}
        aria-label="Highlight tool"
      >
        Highlight
      </button>
      <button
        className={toolButtonClass('comment')}
        onClick={() => onToolChange('comment')}
        disabled={!hasPdf}
        aria-label="Comment tool"
      >
        Comment
      </button>

      <div className="border-l border-gray-600 mx-2 h-6" />

      {/* Search section */}
      <button
        className={buttonClass(!hasPdf)}
        onClick={onSearchOpen}
        disabled={!hasPdf}
        aria-label="Open search"
      >
        Search
      </button>
    </header>
  );
}
