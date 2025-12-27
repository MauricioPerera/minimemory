import { X, Copy, Trash2 } from 'lucide-react';
import type { RecallResult } from '../../api/types';
import { formatDate, cn } from '../../lib/utils';

interface MemoryModalProps {
  memory: RecallResult | null;
  onClose: () => void;
  onDelete: (id: string) => void;
}

const typeColors = {
  episodic: 'bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-300',
  semantic: 'bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-300',
  working: 'bg-amber-100 text-amber-700 dark:bg-amber-900/30 dark:text-amber-300',
};

export function MemoryModal({ memory, onClose, onDelete }: MemoryModalProps) {
  if (!memory) return null;

  const copyToClipboard = (text: string) => {
    navigator.clipboard.writeText(text);
  };

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4">
      <div className="bg-white dark:bg-gray-800 rounded-2xl shadow-xl max-w-2xl w-full max-h-[90vh] overflow-hidden transition-colors">
        {/* Header */}
        <div className="flex items-center justify-between p-6 border-b border-gray-200 dark:border-gray-700">
          <div className="flex items-center gap-3">
            <h2 className="text-lg font-semibold text-gray-900 dark:text-white">Memory Details</h2>
            <span className={cn('px-2 py-1 text-xs font-medium rounded-full', typeColors[memory.type as keyof typeof typeColors] || 'bg-gray-100 text-gray-700 dark:bg-gray-700 dark:text-gray-300')}>
              {memory.type}
            </span>
          </div>
          <button
            onClick={onClose}
            className="p-2 text-gray-400 hover:text-gray-600 dark:hover:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors"
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        {/* Content */}
        <div className="p-6 overflow-y-auto max-h-[60vh]">
          {/* ID */}
          <div className="mb-6">
            <label className="block text-sm font-medium text-gray-500 dark:text-gray-400 mb-1">ID</label>
            <div className="flex items-center gap-2">
              <code className="text-sm font-mono bg-gray-100 dark:bg-gray-700 text-gray-900 dark:text-gray-100 px-3 py-2 rounded-lg flex-1">
                {memory.id}
              </code>
              <button
                onClick={() => copyToClipboard(memory.id)}
                className="p-2 text-gray-400 hover:text-gray-600 dark:hover:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors"
              >
                <Copy className="w-4 h-4" />
              </button>
            </div>
          </div>

          {/* Content */}
          <div className="mb-6">
            <label className="block text-sm font-medium text-gray-500 dark:text-gray-400 mb-1">Content</label>
            <div className="bg-gray-50 dark:bg-gray-700 rounded-lg p-4 text-gray-900 dark:text-white whitespace-pre-wrap">
              {memory.content}
            </div>
          </div>

          {/* Scores */}
          <div className="grid grid-cols-3 gap-4 mb-6">
            <div className="bg-gray-50 dark:bg-gray-700 rounded-lg p-4">
              <label className="block text-sm font-medium text-gray-500 dark:text-gray-400 mb-1">Score</label>
              <div className="text-2xl font-bold text-gray-900 dark:text-white">{memory.score.toFixed(4)}</div>
            </div>
            <div className="bg-gray-50 dark:bg-gray-700 rounded-lg p-4">
              <label className="block text-sm font-medium text-gray-500 dark:text-gray-400 mb-1">Importance</label>
              <div className="text-2xl font-bold text-gray-900 dark:text-white">{(memory.importance * 100).toFixed(0)}%</div>
            </div>
            <div className="bg-gray-50 dark:bg-gray-700 rounded-lg p-4">
              <label className="block text-sm font-medium text-gray-500 dark:text-gray-400 mb-1">Created</label>
              <div className="text-sm font-medium text-gray-900 dark:text-white">{formatDate(memory.createdAt)}</div>
            </div>
          </div>

          {/* Score breakdown */}
          {(memory.vectorSimilarity !== undefined || memory.keywordScore !== undefined) && (
            <div className="mb-6">
              <label className="block text-sm font-medium text-gray-500 dark:text-gray-400 mb-2">Score Breakdown</label>
              <div className="grid grid-cols-2 gap-4">
                {memory.vectorSimilarity !== undefined && (
                  <div className="bg-indigo-50 dark:bg-indigo-900/30 rounded-lg p-3">
                    <span className="text-sm text-indigo-700 dark:text-indigo-300">Vector Similarity</span>
                    <div className="text-lg font-bold text-indigo-900 dark:text-indigo-100">{memory.vectorSimilarity.toFixed(4)}</div>
                  </div>
                )}
                {memory.keywordScore !== undefined && (
                  <div className="bg-green-50 dark:bg-green-900/30 rounded-lg p-3">
                    <span className="text-sm text-green-700 dark:text-green-300">Keyword Score</span>
                    <div className="text-lg font-bold text-green-900 dark:text-green-100">{memory.keywordScore.toFixed(2)}</div>
                  </div>
                )}
              </div>
            </div>
          )}

          {/* Metadata */}
          {memory.metadata && Object.keys(memory.metadata).length > 0 && (
            <div>
              <label className="block text-sm font-medium text-gray-500 dark:text-gray-400 mb-1">Metadata</label>
              <pre className="bg-gray-50 dark:bg-gray-700 rounded-lg p-4 text-sm text-gray-900 dark:text-gray-100 overflow-x-auto">
                {JSON.stringify(memory.metadata, null, 2)}
              </pre>
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="flex items-center justify-end gap-3 p-6 border-t border-gray-200 dark:border-gray-700">
          <button
            onClick={() => {
              onDelete(memory.id);
              onClose();
            }}
            className="flex items-center gap-2 px-4 py-2 text-red-600 dark:text-red-400 hover:bg-red-50 dark:hover:bg-red-900/30 rounded-lg transition-colors"
          >
            <Trash2 className="w-4 h-4" />
            Delete
          </button>
          <button
            onClick={onClose}
            className="px-4 py-2 bg-gray-100 dark:bg-gray-700 text-gray-700 dark:text-gray-200 rounded-lg hover:bg-gray-200 dark:hover:bg-gray-600 transition-colors"
          >
            Close
          </button>
        </div>
      </div>
    </div>
  );
}
