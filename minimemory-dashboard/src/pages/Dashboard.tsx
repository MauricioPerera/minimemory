import { useState } from 'react';
import { Plus } from 'lucide-react';
import { StatsCards } from '../components/stats/StatsCards';
import { TypeChart } from '../components/stats/TypeChart';
import { ImportanceChart } from '../components/stats/ImportanceChart';
import { SearchBar } from '../components/memory/SearchBar';
import { MemoryList } from '../components/memory/MemoryList';
import { MemoryModal } from '../components/memory/MemoryModal';
import { CreateMemoryModal } from '../components/memory/CreateMemoryModal';
import { useSearchMemories, useDeleteMemory } from '../api/hooks';
import type { RecallResult } from '../api/types';

interface DashboardProps {
  namespace: string;
}

export function Dashboard({ namespace }: DashboardProps) {
  const [selectedMemory, setSelectedMemory] = useState<RecallResult | null>(null);
  const [showCreateModal, setShowCreateModal] = useState(false);

  const searchMutation = useSearchMemories();
  const deleteMutation = useDeleteMemory();

  const handleSearch = (params: {
    keywords: string;
    mode: 'keyword' | 'hybrid';
    type?: string;
    limit: number;
  }) => {
    searchMutation.mutate({ params, namespace });
  };

  const handleDelete = (id: string) => {
    deleteMutation.mutate({ id, namespace });
  };

  return (
    <div className="space-y-6 pb-16">
      {/* Header with Create Button */}
      <div className="flex items-center justify-between">
        <h2 className="text-2xl font-bold text-gray-900 dark:text-white">Dashboard</h2>
        <button
          onClick={() => setShowCreateModal(true)}
          className="flex items-center gap-2 px-4 py-2 bg-indigo-600 text-white rounded-lg hover:bg-indigo-700 transition-colors"
        >
          <Plus className="w-4 h-4" />
          Create Memory
        </button>
      </div>

      {/* Stats */}
      <StatsCards namespace={namespace} />

      {/* Charts */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <TypeChart namespace={namespace} />
        <ImportanceChart memories={searchMutation.data?.results || []} />
      </div>

      {/* Search */}
      <SearchBar onSearch={handleSearch} isLoading={searchMutation.isPending} />

      {/* Results */}
      {searchMutation.data && (
        <div>
          <div className="flex items-center justify-between mb-4">
            <h2 className="text-lg font-semibold text-gray-900 dark:text-white">
              Results ({searchMutation.data.count})
            </h2>
          </div>
          <MemoryList
            memories={searchMutation.data.results}
            onView={setSelectedMemory}
            onDelete={handleDelete}
            isDeleting={deleteMutation.isPending ? deleteMutation.variables?.id : undefined}
          />
        </div>
      )}

      {/* Empty State */}
      {!searchMutation.data && !searchMutation.isPending && (
        <div className="bg-white dark:bg-gray-800 rounded-xl p-12 text-center shadow-sm transition-colors">
          <div className="text-gray-400 dark:text-gray-500 text-lg mb-2">Search your memories</div>
          <div className="text-gray-400 dark:text-gray-500 text-sm">
            Enter keywords above to find relevant memories
          </div>
        </div>
      )}

      {/* Memory Detail Modal */}
      <MemoryModal
        memory={selectedMemory}
        onClose={() => setSelectedMemory(null)}
        onDelete={handleDelete}
      />

      {/* Create Memory Modal */}
      <CreateMemoryModal
        isOpen={showCreateModal}
        onClose={() => setShowCreateModal(false)}
        namespace={namespace}
      />
    </div>
  );
}
