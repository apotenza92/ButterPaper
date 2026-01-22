import { useState, useEffect, useRef, useCallback } from 'react';
import { renderThumbnail } from '../lib/tauri';

interface SidebarProps {
  pageCount: number;
  currentPage: number;
  filePath: string | null;
  onPageSelect: (page: number) => void;
}

const MAX_CACHED_THUMBNAILS = 20;

export function Sidebar({ pageCount, currentPage, filePath, onPageSelect }: SidebarProps) {
  const [thumbnails, setThumbnails] = useState<Map<number, string>>(new Map());
  const [loadingPages, setLoadingPages] = useState<Set<number>>(new Set());
  const observerRef = useRef<IntersectionObserver | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const thumbnailRefs = useRef<Map<number, HTMLDivElement>>(new Map());
  const currentFileRef = useRef<string | null>(null);

  // Clear thumbnails when file changes
  useEffect(() => {
    if (filePath !== currentFileRef.current) {
      currentFileRef.current = filePath;
      setThumbnails(new Map());
      setLoadingPages(new Set());
    }
  }, [filePath]);

  const loadThumbnail = useCallback(async (page: number) => {
    if (thumbnails.has(page) || loadingPages.has(page)) return;

    setLoadingPages(prev => new Set(prev).add(page));

    try {
      const response = await renderThumbnail(page);
      if (response.success) {
        setThumbnails(prev => {
          const newMap = new Map(prev);
          // Evict oldest if cache is full
          if (newMap.size >= MAX_CACHED_THUMBNAILS) {
            const firstKey = newMap.keys().next().value;
            if (firstKey !== undefined) {
              newMap.delete(firstKey);
            }
          }
          newMap.set(page, response.image_base64);
          return newMap;
        });
      }
    } catch (err) {
      console.error(`Failed to load thumbnail for page ${page}:`, err);
    } finally {
      setLoadingPages(prev => {
        const newSet = new Set(prev);
        newSet.delete(page);
        return newSet;
      });
    }
  }, [thumbnails, loadingPages]);

  // Setup Intersection Observer for lazy loading
  useEffect(() => {
    if (observerRef.current) {
      observerRef.current.disconnect();
    }

    observerRef.current = new IntersectionObserver(
      (entries) => {
        entries.forEach(entry => {
          if (entry.isIntersecting) {
            const page = Number(entry.target.getAttribute('data-page'));
            if (!isNaN(page)) {
              loadThumbnail(page);
            }
          }
        });
      },
      { rootMargin: '100px' }
    );

    // Observe all thumbnail placeholders
    thumbnailRefs.current.forEach((el) => {
      if (observerRef.current) {
        observerRef.current.observe(el);
      }
    });

    return () => {
      if (observerRef.current) {
        observerRef.current.disconnect();
      }
    };
  }, [pageCount, loadThumbnail]);

  // Auto-scroll to current page
  useEffect(() => {
    const currentThumbnail = thumbnailRefs.current.get(currentPage);
    if (currentThumbnail && containerRef.current) {
      currentThumbnail.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
    }
  }, [currentPage]);

  const setThumbnailRef = useCallback((page: number, el: HTMLDivElement | null) => {
    if (el) {
      thumbnailRefs.current.set(page, el);
      if (observerRef.current) {
        observerRef.current.observe(el);
      }
    } else {
      thumbnailRefs.current.delete(page);
    }
  }, []);

  if (pageCount === 0) {
    return (
      <aside className="w-32 bg-gray-100 dark:bg-gray-900 border-r border-gray-300 dark:border-gray-700 overflow-y-auto flex items-center justify-center">
        <span className="text-gray-500 dark:text-gray-400 text-sm text-center px-2">No pages</span>
      </aside>
    );
  }

  return (
    <aside
      ref={containerRef}
      className="w-32 bg-gray-100 dark:bg-gray-900 border-r border-gray-300 dark:border-gray-700 overflow-y-auto"
    >
      {Array.from({ length: pageCount }, (_, index) => {
        const isSelected = index === currentPage;
        const thumbnailData = thumbnails.get(index);
        const isLoading = loadingPages.has(index);

        return (
          <div
            key={index}
            ref={(el) => setThumbnailRef(index, el)}
            data-page={index}
            className={`p-2 cursor-pointer hover:bg-gray-200 dark:hover:bg-gray-800`}
            onClick={() => onPageSelect(index)}
          >
            <div
              className={`border-2 ${isSelected ? 'border-blue-500' : 'border-transparent'} bg-white dark:bg-gray-700`}
            >
              {thumbnailData ? (
                <img
                  src={`data:image/png;base64,${thumbnailData}`}
                  alt={`Page ${index + 1}`}
                  className="w-full h-auto"
                />
              ) : (
                <div className="w-full aspect-[3/4] bg-gray-300 dark:bg-gray-600 flex items-center justify-center">
                  {isLoading ? (
                    <span className="text-gray-500 dark:text-gray-400 text-xs">Loading...</span>
                  ) : (
                    <span className="text-gray-400 dark:text-gray-500 text-xs">&nbsp;</span>
                  )}
                </div>
              )}
            </div>
            <div className="text-center text-xs mt-1 text-gray-700 dark:text-gray-300">
              {index + 1}
            </div>
          </div>
        );
      })}
    </aside>
  );
}
