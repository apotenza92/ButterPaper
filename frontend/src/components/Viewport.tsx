import { useState, useEffect, useRef, useCallback } from 'react';
import { renderPage, getPageDimensions } from '../lib/tauri';
import { FitMode, Tool } from '../hooks/usePdfState';

interface ViewportProps {
  filePath: string | null;
  currentPage: number;
  zoomPercent: number;
  fitMode: FitMode;
  activeTool: Tool;
}

interface PageDimensions {
  width: number;
  height: number;
}

function calculateRenderSize(
  pageDimensions: PageDimensions,
  viewportSize: { width: number; height: number },
  fitMode: FitMode,
  zoomPercent: number
): { width: number; height: number } {
  const padding = 32; // 16px each side
  const availableWidth = viewportSize.width - padding;
  const availableHeight = viewportSize.height - padding;
  const pageAspect = pageDimensions.width / pageDimensions.height;

  let baseWidth: number;
  let baseHeight: number;

  switch (fitMode) {
    case 'fit-page':
      // Fit entire page within viewport
      if (availableWidth / availableHeight > pageAspect) {
        baseHeight = availableHeight;
        baseWidth = baseHeight * pageAspect;
      } else {
        baseWidth = availableWidth;
        baseHeight = baseWidth / pageAspect;
      }
      break;
    case 'fit-width':
      baseWidth = availableWidth;
      baseHeight = baseWidth / pageAspect;
      break;
    case 'actual-size':
      // 72 DPI: 1 PDF point = 1 pixel
      baseWidth = pageDimensions.width;
      baseHeight = pageDimensions.height;
      break;
  }

  // Apply zoom
  const scale = zoomPercent / 100;
  return {
    width: Math.round(baseWidth * scale),
    height: Math.round(baseHeight * scale),
  };
}

export function Viewport({
  filePath,
  currentPage,
  zoomPercent,
  fitMode,
  activeTool,
}: ViewportProps) {
  const [pageImage, setPageImage] = useState<string | null>(null);
  const [pageDimensions, setPageDimensions] = useState<PageDimensions | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [viewportSize, setViewportSize] = useState({ width: 800, height: 600 });
  const containerRef = useRef<HTMLDivElement>(null);
  const scrollRef = useRef<HTMLDivElement>(null);

  // For hand tool panning
  const [isDragging, setIsDragging] = useState(false);
  const [dragStart, setDragStart] = useState({ x: 0, y: 0 });
  const [scrollStart, setScrollStart] = useState({ x: 0, y: 0 });

  // Measure viewport size
  useEffect(() => {
    const updateSize = () => {
      if (containerRef.current) {
        setViewportSize({
          width: containerRef.current.clientWidth,
          height: containerRef.current.clientHeight,
        });
      }
    };

    updateSize();
    window.addEventListener('resize', updateSize);
    return () => window.removeEventListener('resize', updateSize);
  }, []);

  // Fetch page dimensions when page changes
  useEffect(() => {
    if (!filePath) {
      setPageDimensions(null);
      return;
    }

    const fetchDimensions = async () => {
      try {
        const response = await getPageDimensions(currentPage);
        if (response.success) {
          setPageDimensions({ width: response.width, height: response.height });
        }
      } catch (err) {
        console.error('Failed to get page dimensions:', err);
      }
    };

    fetchDimensions();
  }, [filePath, currentPage]);

  // Render page when dimensions, viewport, zoom, or fit mode change
  useEffect(() => {
    if (!filePath || !pageDimensions) {
      setPageImage(null);
      return;
    }

    const renderSize = calculateRenderSize(pageDimensions, viewportSize, fitMode, zoomPercent);

    if (renderSize.width <= 0 || renderSize.height <= 0) return;

    const fetchPage = async () => {
      setIsLoading(true);
      try {
        const response = await renderPage(currentPage, renderSize.width, renderSize.height);
        if (response.success) {
          setPageImage(response.image_base64);
        }
      } catch (err) {
        console.error('Failed to render page:', err);
      } finally {
        setIsLoading(false);
      }
    };

    fetchPage();
  }, [filePath, currentPage, pageDimensions, viewportSize, fitMode, zoomPercent]);

  // Hand tool drag handlers
  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    if (activeTool !== 'hand' || !scrollRef.current) return;
    setIsDragging(true);
    setDragStart({ x: e.clientX, y: e.clientY });
    setScrollStart({
      x: scrollRef.current.scrollLeft,
      y: scrollRef.current.scrollTop,
    });
    e.preventDefault();
  }, [activeTool]);

  const handleMouseMove = useCallback((e: React.MouseEvent) => {
    if (!isDragging || !scrollRef.current) return;
    const dx = e.clientX - dragStart.x;
    const dy = e.clientY - dragStart.y;
    scrollRef.current.scrollLeft = scrollStart.x - dx;
    scrollRef.current.scrollTop = scrollStart.y - dy;
  }, [isDragging, dragStart, scrollStart]);

  const handleMouseUp = useCallback(() => {
    setIsDragging(false);
  }, []);

  const handleMouseLeave = useCallback(() => {
    setIsDragging(false);
  }, []);

  // Empty state
  if (!filePath) {
    return (
      <main
        ref={containerRef}
        className="flex-1 bg-gray-200 dark:bg-gray-700 flex items-center justify-center"
      >
        <span className="text-gray-500 dark:text-gray-400 text-lg">Open a PDF to begin</span>
      </main>
    );
  }

  const renderSize = pageDimensions
    ? calculateRenderSize(pageDimensions, viewportSize, fitMode, zoomPercent)
    : null;

  const needsScroll = renderSize && (renderSize.width > viewportSize.width - 32 || renderSize.height > viewportSize.height - 32);

  return (
    <main
      ref={containerRef}
      className="flex-1 bg-gray-200 dark:bg-gray-700 overflow-hidden"
    >
      <div
        ref={scrollRef}
        className={`w-full h-full ${needsScroll ? 'overflow-auto' : 'overflow-hidden flex items-center justify-center'}`}
        style={{ cursor: activeTool === 'hand' ? (isDragging ? 'grabbing' : 'grab') : 'default' }}
        onMouseDown={handleMouseDown}
        onMouseMove={handleMouseMove}
        onMouseUp={handleMouseUp}
        onMouseLeave={handleMouseLeave}
      >
        {isLoading && !pageImage ? (
          <div className="flex items-center justify-center w-full h-full">
            <span className="text-gray-500 dark:text-gray-400">Loading...</span>
          </div>
        ) : pageImage ? (
          <div className={`${needsScroll ? 'p-4' : ''}`}>
            <div className="shadow-lg bg-white">
              <img
                src={`data:image/png;base64,${pageImage}`}
                alt={`Page ${currentPage + 1}`}
                style={{
                  width: renderSize?.width,
                  height: renderSize?.height,
                }}
                draggable={false}
              />
            </div>
          </div>
        ) : null}
      </div>
    </main>
  );
}
