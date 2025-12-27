import { useState } from 'react';
import { X, Key, Download, Trash2, AlertTriangle } from 'lucide-react';
import { getApiKey, setApiKey } from '../api/client';
import { useExportMemories, useClearNamespace } from '../api/hooks';
import { downloadJson } from '../lib/utils';

interface SettingsModalProps {
  isOpen: boolean;
  onClose: () => void;
  namespace: string;
}

export function SettingsModal({ isOpen, onClose, namespace }: SettingsModalProps) {
  const [apiKeyInput, setApiKeyInput] = useState(getApiKey());
  const [showClearConfirm, setShowClearConfirm] = useState(false);

  const exportMutation = useExportMemories();
  const clearMutation = useClearNamespace();

  if (!isOpen) return null;

  const handleSaveApiKey = () => {
    setApiKey(apiKeyInput);
    window.location.reload(); // Refresh to use new key
  };

  const handleExport = async () => {
    const data = await exportMutation.mutateAsync(namespace);
    downloadJson(data.data, `minimemory-${namespace}-${Date.now()}.json`);
  };

  const handleClear = async () => {
    await clearMutation.mutateAsync(namespace);
    setShowClearConfirm(false);
  };

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4">
      <div className="bg-white dark:bg-gray-800 rounded-2xl shadow-xl max-w-md w-full transition-colors">
        {/* Header */}
        <div className="flex items-center justify-between p-6 border-b border-gray-200 dark:border-gray-700">
          <h2 className="text-lg font-semibold text-gray-900 dark:text-white">Settings</h2>
          <button
            onClick={onClose}
            className="p-2 text-gray-400 hover:text-gray-600 dark:hover:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors"
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        {/* Content */}
        <div className="p-6 space-y-6">
          {/* API Key */}
          <div>
            <label className="flex items-center gap-2 text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
              <Key className="w-4 h-4" />
              API Key
            </label>
            <div className="flex gap-2">
              <input
                type="password"
                value={apiKeyInput}
                onChange={(e) => setApiKeyInput(e.target.value)}
                className="flex-1 px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-indigo-500 transition-colors"
                placeholder="mm_..."
              />
              <button
                onClick={handleSaveApiKey}
                className="px-4 py-2 bg-indigo-600 text-white rounded-lg hover:bg-indigo-700 transition-colors"
              >
                Save
              </button>
            </div>
            <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">
              Stored in localStorage. Default: mm_dev_key_12345
            </p>
          </div>

          {/* Current Namespace */}
          <div>
            <label className="text-sm font-medium text-gray-700 dark:text-gray-300 mb-2 block">
              Current Namespace
            </label>
            <div className="px-3 py-2 bg-gray-100 dark:bg-gray-700 rounded-lg font-mono text-sm text-gray-900 dark:text-gray-100">
              {namespace}
            </div>
          </div>

          {/* Export */}
          <div>
            <button
              onClick={handleExport}
              disabled={exportMutation.isPending}
              className="flex items-center gap-2 w-full px-4 py-3 bg-gray-100 dark:bg-gray-700 text-gray-700 dark:text-gray-200 rounded-lg hover:bg-gray-200 dark:hover:bg-gray-600 transition-colors disabled:opacity-50"
            >
              <Download className="w-5 h-5" />
              {exportMutation.isPending ? 'Exporting...' : 'Export Memories'}
            </button>
          </div>

          {/* Clear */}
          <div className="pt-4 border-t border-gray-200 dark:border-gray-700">
            {!showClearConfirm ? (
              <button
                onClick={() => setShowClearConfirm(true)}
                className="flex items-center gap-2 w-full px-4 py-3 bg-red-50 dark:bg-red-900/30 text-red-600 dark:text-red-400 rounded-lg hover:bg-red-100 dark:hover:bg-red-900/50 transition-colors"
              >
                <Trash2 className="w-5 h-5" />
                Clear All Memories
              </button>
            ) : (
              <div className="bg-red-50 dark:bg-red-900/30 rounded-lg p-4">
                <div className="flex items-center gap-2 text-red-700 dark:text-red-400 mb-3">
                  <AlertTriangle className="w-5 h-5" />
                  <span className="font-medium">Are you sure?</span>
                </div>
                <p className="text-sm text-red-600 dark:text-red-400 mb-4">
                  This will permanently delete all memories in namespace "{namespace}".
                </p>
                <div className="flex gap-2">
                  <button
                    onClick={() => setShowClearConfirm(false)}
                    className="flex-1 px-3 py-2 bg-white dark:bg-gray-700 text-gray-700 dark:text-gray-200 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-600 transition-colors"
                  >
                    Cancel
                  </button>
                  <button
                    onClick={handleClear}
                    disabled={clearMutation.isPending}
                    className="flex-1 px-3 py-2 bg-red-600 text-white rounded-lg hover:bg-red-700 transition-colors disabled:opacity-50"
                  >
                    {clearMutation.isPending ? 'Clearing...' : 'Clear All'}
                  </button>
                </div>
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
