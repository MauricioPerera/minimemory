import { Trash2, Eye, Clock, Sparkles } from 'lucide-react';
import type { RecallResult } from '../../api/types';
import { truncate, formatRelativeTime, cn } from '../../lib/utils';

interface MemoryListProps {
  memories: RecallResult[];
  onView: (memory: RecallResult) => void;
  onDelete: (id: string) => void;
  isDeleting?: string;
}

const typeColors = {
  episodic: 'bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-300',
  semantic: 'bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-300',
  working: 'bg-amber-100 text-amber-700 dark:bg-amber-900/30 dark:text-amber-300',
};

export function MemoryList({ memories, onView, onDelete, isDeleting }: MemoryListProps) {
  if (memories.length === 0) {
    return (
      <div className="bg-white dark:bg-gray-800 rounded-xl p-12 text-center shadow-sm transition-colors">
        <div className="text-gray-400 dark:text-gray-500 text-lg">No memories found</div>
        <div className="text-gray-400 dark:text-gray-500 text-sm mt-2">Try a different search query</div>
      </div>
    );
  }

  return (
    <div className="bg-white dark:bg-gray-800 rounded-xl shadow-sm overflow-hidden transition-colors">
      <table className="w-full">
        <thead className="bg-gray-50 dark:bg-gray-900/50 border-b border-gray-200 dark:border-gray-700">
          <tr>
            <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">Content</th>
            <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">Type</th>
            <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">Score</th>
            <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">Importance</th>
            <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">Created</th>
            <th className="px-6 py-3 text-right text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">Actions</th>
          </tr>
        </thead>
        <tbody className="divide-y divide-gray-200 dark:divide-gray-700">
          {memories.map((memory) => (
            <tr key={memory.id} className="hover:bg-gray-50 dark:hover:bg-gray-700/50 transition-colors">
              <td className="px-6 py-4">
                <div className="text-sm text-gray-900 dark:text-white font-medium">
                  {truncate(memory.content, 60)}
                </div>
                <div className="text-xs text-gray-500 dark:text-gray-400 mt-1 font-mono">
                  {memory.id}
                </div>
              </td>
              <td className="px-6 py-4">
                <span className={cn('px-2 py-1 text-xs font-medium rounded-full', typeColors[memory.type as keyof typeof typeColors] || 'bg-gray-100 text-gray-700 dark:bg-gray-700 dark:text-gray-300')}>
                  {memory.type}
                </span>
              </td>
              <td className="px-6 py-4">
                <div className="flex flex-col text-sm text-gray-900 dark:text-white">
                  <span className="font-medium">{memory.score.toFixed(4)}</span>
                  {memory.keywordScore !== undefined && (
                    <span className="text-xs text-gray-500 dark:text-gray-400">KW: {memory.keywordScore.toFixed(2)}</span>
                  )}
                  {memory.vectorSimilarity !== undefined && (
                    <span className="text-xs text-gray-500 dark:text-gray-400">Vec: {memory.vectorSimilarity.toFixed(4)}</span>
                  )}
                </div>
              </td>
              <td className="px-6 py-4">
                <div className="flex items-center gap-2">
                  <Sparkles className="w-4 h-4 text-amber-500" />
                  <span className="text-sm font-medium text-gray-900 dark:text-white">{(memory.importance * 100).toFixed(0)}%</span>
                </div>
              </td>
              <td className="px-6 py-4">
                <div className="flex items-center gap-2 text-sm text-gray-500 dark:text-gray-400">
                  <Clock className="w-4 h-4" />
                  {formatRelativeTime(memory.createdAt)}
                </div>
              </td>
              <td className="px-6 py-4 text-right">
                <div className="flex items-center justify-end gap-2">
                  <button
                    onClick={() => onView(memory)}
                    className="p-2 text-gray-400 hover:text-indigo-600 dark:hover:text-indigo-400 hover:bg-indigo-50 dark:hover:bg-indigo-900/30 rounded-lg transition-colors"
                    title="View"
                  >
                    <Eye className="w-4 h-4" />
                  </button>
                  <button
                    onClick={() => onDelete(memory.id)}
                    disabled={isDeleting === memory.id}
                    className="p-2 text-gray-400 hover:text-red-600 dark:hover:text-red-400 hover:bg-red-50 dark:hover:bg-red-900/30 rounded-lg transition-colors disabled:opacity-50"
                    title="Delete"
                  >
                    <Trash2 className="w-4 h-4" />
                  </button>
                </div>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
