import { useEffect, useCallback } from 'react';

interface ErrorDialogProps {
  isOpen: boolean;
  message: string;
  onClose: () => void;
}

export function ErrorDialog({ isOpen, message, onClose }: ErrorDialogProps) {
  // Handle escape key
  useEffect(() => {
    if (!isOpen) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        onClose();
      }
    };

    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [isOpen, onClose]);

  const handleBackdropClick = useCallback((e: React.MouseEvent) => {
    if (e.target === e.currentTarget) {
      onClose();
    }
  }, [onClose]);

  if (!isOpen) return null;

  return (
    <div
      className="fixed inset-0 bg-black/50 flex items-center justify-center z-50"
      onClick={handleBackdropClick}
    >
      <div className="bg-white dark:bg-gray-800 rounded-lg shadow-xl p-6 max-w-md">
        <h2 className="text-lg font-semibold text-red-600">Error</h2>
        <p className="mt-2 text-gray-700 dark:text-gray-300">{message}</p>
        <button
          className="mt-4 px-4 py-2 bg-gray-800 text-white rounded hover:bg-gray-700"
          onClick={onClose}
          aria-label="Close error dialog"
        >
          OK
        </button>
      </div>
    </div>
  );
}
