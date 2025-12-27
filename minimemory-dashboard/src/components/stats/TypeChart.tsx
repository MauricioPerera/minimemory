import { PieChart, Pie, Cell, ResponsiveContainer, Legend, Tooltip } from 'recharts';
import { useStats } from '../../api/hooks';

interface TypeChartProps {
  namespace: string;
}

const COLORS = {
  episodic: '#6366f1', // indigo
  semantic: '#22c55e', // green
  working: '#f59e0b',  // amber
};

const DARK_COLORS = {
  episodic: '#818cf8', // indigo-400
  semantic: '#4ade80', // green-400
  working: '#fbbf24',  // amber-400
};

export function TypeChart({ namespace }: TypeChartProps) {
  const { data, isLoading, error } = useStats(namespace);

  if (isLoading) {
    return (
      <div className="bg-white dark:bg-gray-800 rounded-xl p-6 shadow-sm transition-colors">
        <div className="h-4 bg-gray-200 dark:bg-gray-700 rounded w-32 mb-4 animate-pulse"></div>
        <div className="h-64 bg-gray-100 dark:bg-gray-700 rounded animate-pulse"></div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="bg-white dark:bg-gray-800 rounded-xl p-6 shadow-sm transition-colors">
        <div className="text-red-500 dark:text-red-400">Error loading chart</div>
      </div>
    );
  }

  const stats = data?.stats;
  if (!stats || stats.total === 0) {
    return (
      <div className="bg-white dark:bg-gray-800 rounded-xl p-6 shadow-sm transition-colors">
        <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">Memory Types</h3>
        <div className="h-64 flex items-center justify-center text-gray-400 dark:text-gray-500">
          No memories yet
        </div>
      </div>
    );
  }

  const isDark = document.documentElement.classList.contains('dark');
  const colors = isDark ? DARK_COLORS : COLORS;

  const chartData = [
    { name: 'Episodic', value: stats.byType.episodic, color: colors.episodic },
    { name: 'Semantic', value: stats.byType.semantic, color: colors.semantic },
    { name: 'Working', value: stats.byType.working, color: colors.working },
  ].filter(d => d.value > 0);

  return (
    <div className="bg-white dark:bg-gray-800 rounded-xl p-6 shadow-sm transition-colors">
      <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">Memory Types</h3>
      <div className="h-64 min-h-[256px]">
        <ResponsiveContainer width="100%" height="100%" minWidth={100} minHeight={100}>
          <PieChart>
            <Pie
              data={chartData}
              cx="50%"
              cy="50%"
              innerRadius={60}
              outerRadius={90}
              paddingAngle={2}
              dataKey="value"
              label={({ name, percent }) => `${name} ${((percent ?? 0) * 100).toFixed(0)}%`}
              labelLine={false}
            >
              {chartData.map((entry, index) => (
                <Cell key={`cell-${index}`} fill={entry.color} />
              ))}
            </Pie>
            <Tooltip
              contentStyle={{
                backgroundColor: isDark ? '#1f2937' : '#fff',
                border: `1px solid ${isDark ? '#374151' : '#e5e7eb'}`,
                borderRadius: '8px',
                color: isDark ? '#fff' : '#111827',
              }}
              formatter={(value) => [`${value} memories`, '']}
            />
            <Legend
              verticalAlign="bottom"
              height={36}
              formatter={(value) => (
                <span className="text-gray-700 dark:text-gray-300">{value}</span>
              )}
            />
          </PieChart>
        </ResponsiveContainer>
      </div>
    </div>
  );
}
