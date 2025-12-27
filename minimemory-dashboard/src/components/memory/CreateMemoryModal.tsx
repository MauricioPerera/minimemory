import { useState } from 'react';
import { X, Plus, Brain } from 'lucide-react';
import { useRemember } from '../../api/hooks';

interface CreateMemoryModalProps {
  isOpen: boolean;
  onClose: () => void;
  namespace: string;
}

export function CreateMemoryModal({ isOpen, onClose, namespace }: CreateMemoryModalProps) {
  const [content, setContent] = useState('');
  const [type, setType] = useState<'episodic' | 'semantic' | 'working'>('episodic');
  const [importance, setImportance] = useState(0.5);
  const [metadataJson, setMetadataJson] = useState('');
  const [metadataError, setMetadataError] = useState('');

  const rememberMutation = useRemember();

  if (!isOpen) return null;

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setMetadataError('');

    let metadata: Record<string, unknown> | undefined;
    if (metadataJson.trim()) {
      try {
        metadata = JSON.parse(metadataJson);
      } catch {
        setMetadataError('Invalid JSON');
        return;
      }
    }

    try {
      await rememberMutation.mutateAsync({
        params: {
          content,
          type,
          importance,
          metadata,
        },
        namespace,
      });

      // Reset form
      setContent('');
      setType('episodic');
      setImportance(0.5);
      setMetadataJson('');
      onClose();
    } catch (error) {
      console.error('Failed to create memory:', error);
    }
  };

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4">
      <div className="bg-white dark:bg-gray-800 rounded-2xl shadow-xl max-w-lg w-full transition-colors">
        {/* Header */}
        <div className="flex items-center justify-between p-6 border-b border-gray-200 dark:border-gray-700">
          <div className="flex items-center gap-3">
            <div className="p-2 bg-indigo-100 dark:bg-indigo-900/30 rounded-lg">
              <Brain className="w-5 h-5 text-indigo-600 dark:text-indigo-400" />
            </div>
            <h2 className="text-lg font-semibold text-gray-900 dark:text-white">Create Memory</h2>
          </div>
          <button
            onClick={onClose}
            className="p-2 text-gray-400 hover:text-gray-600 dark:hover:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors"
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        {/* Form */}
        <form onSubmit={handleSubmit} className="p-6 space-y-4">
          {/* Content */}
          <div>
            <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
              Content *
            </label>
            <textarea
              value={content}
              onChange={(e) => setContent(e.target.value)}
              required
              rows={4}
              placeholder="Enter the memory content..."
              className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white placeholder-gray-400 dark:placeholder-gray-500 focus:ring-2 focus:ring-indigo-500 focus:border-indigo-500 transition-colors resize-none"
            />
          </div>

          {/* Type */}
          <div>
            <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
              Type
            </label>
            <select
              value={type}
              onChange={(e) => setType(e.target.value as 'episodic' | 'semantic' | 'working')}
              className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-indigo-500 transition-colors"
            >
              <option value="episodic">Episodic (events, experiences)</option>
              <option value="semantic">Semantic (facts, knowledge)</option>
              <option value="working">Working (temporary, task-related)</option>
            </select>
          </div>

          {/* Importance */}
          <div>
            <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
              Importance: {(importance * 100).toFixed(0)}%
            </label>
            <input
              type="range"
              min="0"
              max="1"
              step="0.1"
              value={importance}
              onChange={(e) => setImportance(parseFloat(e.target.value))}
              className="w-full h-2 bg-gray-200 dark:bg-gray-700 rounded-lg appearance-none cursor-pointer accent-indigo-600"
            />
            <div className="flex justify-between text-xs text-gray-500 dark:text-gray-400 mt-1">
              <span>Low</span>
              <span>Medium</span>
              <span>High</span>
            </div>
          </div>

          {/* Metadata */}
          <div>
            <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
              Metadata (JSON, optional)
            </label>
            <textarea
              value={metadataJson}
              onChange={(e) => {
                const value = e.target.value;
                setMetadataJson(value);
                // Real-time JSON validation
                if (value.trim()) {
                  try {
                    JSON.parse(value);
                    setMetadataError('');
                  } catch (err) {
                    setMetadataError(`Invalid JSON: ${err instanceof Error ? err.message : 'Parse error'}`);
                  }
                } else {
                  setMetadataError('');
                }
              }}
              rows={2}
              placeholder='{"key": "value"}'
              className={`w-full px-3 py-2 border rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white placeholder-gray-400 dark:placeholder-gray-500 focus:ring-2 focus:ring-indigo-500 focus:border-indigo-500 transition-colors resize-none font-mono text-sm ${
                metadataError ? 'border-red-500 focus:ring-red-500' : 'border-gray-300 dark:border-gray-600'
              }`}
            />
            {metadataError && (
              <p className="text-xs text-red-500 mt-1">{metadataError}</p>
            )}
            {metadataJson.trim() && !metadataError && (
              <p className="text-xs text-green-500 mt-1">Valid JSON</p>
            )}
          </div>

          {/* Error message */}
          {rememberMutation.isError && (
            <div className="p-3 bg-red-50 dark:bg-red-900/30 text-red-700 dark:text-red-400 rounded-lg text-sm">
              {rememberMutation.error?.message || 'Failed to create memory'}
            </div>
          )}

          {/* Success message */}
          {rememberMutation.isSuccess && (
            <div className="p-3 bg-green-50 dark:bg-green-900/30 text-green-700 dark:text-green-400 rounded-lg text-sm">
              Memory created successfully!
            </div>
          )}

          {/* Actions */}
          <div className="flex items-center justify-end gap-3 pt-4">
            <button
              type="button"
              onClick={onClose}
              className="px-4 py-2 text-gray-700 dark:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={!content.trim() || rememberMutation.isPending || !!metadataError}
              className="flex items-center gap-2 px-4 py-2 bg-indigo-600 text-white rounded-lg hover:bg-indigo-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
            >
              <Plus className="w-4 h-4" />
              {rememberMutation.isPending ? 'Creating...' : 'Create Memory'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
