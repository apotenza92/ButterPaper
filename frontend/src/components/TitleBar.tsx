import { getCurrentWindow } from '@tauri-apps/api/window';

export function TitleBar() {
  const appWindow = getCurrentWindow();

  const handleMinimize = () => {
    appWindow.minimize();
  };

  const handleMaximize = () => {
    appWindow.toggleMaximize();
  };

  const handleClose = () => {
    appWindow.close();
  };

  return (
    <div
      data-tauri-drag-region
      className="h-8 bg-gray-900 flex items-center justify-between select-none"
    >
      {/* App title */}
      <div data-tauri-drag-region className="flex items-center px-3 h-full">
        <span className="text-white text-sm font-medium">PDF Editor</span>
      </div>

      {/* Window controls */}
      <div className="flex h-full">
        <button
          onClick={handleMinimize}
          className="w-12 h-full flex items-center justify-center text-gray-400 hover:bg-gray-700 hover:text-white transition-colors"
          aria-label="Minimize"
        >
          <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M20 12H4" />
          </svg>
        </button>
        <button
          onClick={handleMaximize}
          className="w-12 h-full flex items-center justify-center text-gray-400 hover:bg-gray-700 hover:text-white transition-colors"
          aria-label="Maximize"
        >
          <svg className="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <rect x="4" y="4" width="16" height="16" strokeWidth={2} />
          </svg>
        </button>
        <button
          onClick={handleClose}
          className="w-12 h-full flex items-center justify-center text-gray-400 hover:bg-red-600 hover:text-white transition-colors"
          aria-label="Close"
        >
          <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      </div>
    </div>
  );
}
