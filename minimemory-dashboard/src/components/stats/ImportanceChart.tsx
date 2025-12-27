import { BarChart, Bar, XAxis, YAxis, ResponsiveContainer, Tooltip, Cell } from 'recharts';

interface ImportanceChartProps {
  memories: Array<{ importance: number }>;
}

export function ImportanceChart({ memories }: ImportanceChartProps) {
  if (!memories || memories.length === 0) {
    return (
      <div className="bg-white dark:bg-gray-800 rounded-xl p-6 shadow-sm transition-colors">
        <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">Importance Distribution</h3>
        <div className="h-48 flex items-center justify-center text-gray-400 dark:text-gray-500">
          Search memories to see distribution
        </div>
      </div>
    );
  }

  // Create buckets for importance distribution
  const buckets = [
    { range: '0-20%', min: 0, max: 0.2, count: 0 },
    { range: '21-40%', min: 0.2, max: 0.4, count: 0 },
    { range: '41-60%', min: 0.4, max: 0.6, count: 0 },
    { range: '61-80%', min: 0.6, max: 0.8, count: 0 },
    { range: '81-100%', min: 0.8, max: 1.0, count: 0 },
  ];

  memories.forEach(m => {
    for (const bucket of buckets) {
      if (m.importance >= bucket.min && m.importance <= bucket.max) {
        bucket.count++;
        break;
      }
    }
  });

  const isDark = document.documentElement.classList.contains('dark');

  const getColor = (index: number) => {
    const colors = isDark
      ? ['#6366f1', '#818cf8', '#a5b4fc', '#c7d2fe', '#e0e7ff']
      : ['#4f46e5', '#6366f1', '#818cf8', '#a5b4fc', '#c7d2fe'];
    return colors[index];
  };

  return (
    <div className="bg-white dark:bg-gray-800 rounded-xl p-6 shadow-sm transition-colors">
      <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">
        Importance Distribution
        <span className="text-sm font-normal text-gray-500 dark:text-gray-400 ml-2">
          ({memories.length} memories)
        </span>
      </h3>
      <div className="h-48 min-h-[192px]">
        <ResponsiveContainer width="100%" height="100%" minWidth={100} minHeight={100}>
          <BarChart data={buckets} margin={{ top: 10, right: 10, left: -20, bottom: 0 }}>
            <XAxis
              dataKey="range"
              tick={{ fill: isDark ? '#9ca3af' : '#6b7280', fontSize: 12 }}
              axisLine={{ stroke: isDark ? '#374151' : '#e5e7eb' }}
              tickLine={false}
            />
            <YAxis
              tick={{ fill: isDark ? '#9ca3af' : '#6b7280', fontSize: 12 }}
              axisLine={{ stroke: isDark ? '#374151' : '#e5e7eb' }}
              tickLine={false}
              allowDecimals={false}
            />
            <Tooltip
              contentStyle={{
                backgroundColor: isDark ? '#1f2937' : '#fff',
                border: `1px solid ${isDark ? '#374151' : '#e5e7eb'}`,
                borderRadius: '8px',
                color: isDark ? '#fff' : '#111827',
              }}
              formatter={(value) => [`${value} memories`, 'Count']}
            />
            <Bar dataKey="count" radius={[4, 4, 0, 0]}>
              {buckets.map((_, index) => (
                <Cell key={`cell-${index}`} fill={getColor(index)} />
              ))}
            </Bar>
          </BarChart>
        </ResponsiveContainer>
      </div>
    </div>
  );
}
