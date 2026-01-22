import { useState, useEffect, useRef, useCallback, KeyboardEvent } from 'react';

interface SearchBarProps {
  isOpen: boolean;
  onClose: () => void;
  onSearch: (query: string) => void;
  onNext: () => void;
  onPrev: () => void;
  matchCount: number;
  currentMatch: number;
}

export function SearchBar({
  isOpen,
  onClose,
  onSearch,
  onNext,
  onPrev,
  matchCount,
  currentMatch,
}: SearchBarProps) {
  const [query, setQuery] = useState('');
  const inputRef = useRef<HTMLInputElement>(null);

  // Auto-focus when opened
  useEffect(() => {
    if (isOpen && inputRef.current) {
      inputRef.current.focus();
    }
  }, [isOpen]);

  // Debounced search
  useEffect(() => {
    if (!isOpen) return;
    const timer = setTimeout(() => onSearch(query), 300);
    return () => clearTimeout(timer);
  }, [query, isOpen, onSearch]);

  const handleKeyDown = useCallback((e: KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Escape') {
      onClose();
    } else if (e.key === 'Enter') {
      if (e.shiftKey) {
        onPrev();
      } else {
        onNext();
      }
    }
  }, [onClose, onNext, onPrev]);

  if (!isOpen) return null;

  return (
    <div className="fixed top-0 left-0 right-0 h-12 bg-white dark:bg-gray-800 border-b border-gray-300 dark:border-gray-600 shadow-md flex items-center px-4 gap-2 z-50">
      <input
        ref={inputRef}
        type="text"
        className="flex-1 px-3 py-1 rounded border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-blue-500"
        placeholder="Search..."
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        onKeyDown={handleKeyDown}
        aria-label="Search text"
      />
      <span className="text-sm text-gray-600 dark:text-gray-300 min-w-[60px] text-center">
        {matchCount > 0 ? `${currentMatch} of ${matchCount}` : '0 matches'}
      </span>
      <button
        className="px-2 py-1 rounded hover:bg-gray-200 dark:hover:bg-gray-700 text-gray-700 dark:text-gray-300"
        onClick={onPrev}
        disabled={matchCount === 0}
        aria-label="Previous match"
      >
        &lt;
      </button>
      <button
        className="px-2 py-1 rounded hover:bg-gray-200 dark:hover:bg-gray-700 text-gray-700 dark:text-gray-300"
        onClick={onNext}
        disabled={matchCount === 0}
        aria-label="Next match"
      >
        &gt;
      </button>
      <button
        className="px-2 py-1 rounded hover:bg-gray-200 dark:hover:bg-gray-700 text-gray-700 dark:text-gray-300"
        onClick={onClose}
        aria-label="Close search"
      >
        &times;
      </button>
    </div>
  );
}
