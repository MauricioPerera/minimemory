import { Database, Brain, Clock, Sparkles } from 'lucide-react';
import { useStats } from '../../api/hooks';
import { formatRelativeTime } from '../../lib/utils';

interface StatsCardsProps {
  namespace: string;
}

export function StatsCards({ namespace }: StatsCardsProps) {
  const { data, isLoading, error } = useStats(namespace);

  if (isLoading) {
    return (
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        {[1, 2, 3, 4].map((i) => (
          <div key={i} className="bg-white dark:bg-gray-800 rounded-xl p-6 shadow-sm animate-pulse">
            <div className="h-4 bg-gray-200 dark:bg-gray-700 rounded w-24 mb-4"></div>
            <div className="h-8 bg-gray-200 dark:bg-gray-700 rounded w-16"></div>
          </div>
        ))}
      </div>
    );
  }

  if (error) {
    return (
      <div className="bg-red-50 dark:bg-red-900/20 text-red-700 dark:text-red-400 rounded-xl p-6">
        Error loading stats: {error.message}
      </div>
    );
  }

  const stats = data?.stats;
  if (!stats) return null;

  const cards = [
    {
      title: 'Total Memories',
      value: stats.total,
      icon: Database,
      color: 'text-indigo-600 dark:text-indigo-400',
      bgColor: 'bg-indigo-100 dark:bg-indigo-900/30',
    },
    {
      title: 'By Type',
      value: `${stats.byType.episodic}/${stats.byType.semantic}/${stats.byType.working}`,
      subtitle: 'E / S / W',
      icon: Brain,
      color: 'text-purple-600 dark:text-purple-400',
      bgColor: 'bg-purple-100 dark:bg-purple-900/30',
    },
    {
      title: 'Avg Importance',
      value: (stats.averageImportance * 100).toFixed(0) + '%',
      icon: Sparkles,
      color: 'text-amber-600 dark:text-amber-400',
      bgColor: 'bg-amber-100 dark:bg-amber-900/30',
    },
    {
      title: 'Last Memory',
      value: stats.newestMemory ? formatRelativeTime(stats.newestMemory) : 'None',
      icon: Clock,
      color: 'text-green-600 dark:text-green-400',
      bgColor: 'bg-green-100 dark:bg-green-900/30',
    },
  ];

  return (
    <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
      {cards.map((card) => (
        <div key={card.title} className="bg-white dark:bg-gray-800 rounded-xl p-6 shadow-sm transition-colors">
          <div className="flex items-center justify-between mb-4">
            <span className="text-sm font-medium text-gray-500 dark:text-gray-400">{card.title}</span>
            <div className={`p-2 rounded-lg ${card.bgColor}`}>
              <card.icon className={`w-5 h-5 ${card.color}`} />
            </div>
          </div>
          <div className="text-2xl font-bold text-gray-900 dark:text-white">{card.value}</div>
          {card.subtitle && (
            <div className="text-xs text-gray-400 dark:text-gray-500 mt-1">{card.subtitle}</div>
          )}
        </div>
      ))}
    </div>
  );
}
