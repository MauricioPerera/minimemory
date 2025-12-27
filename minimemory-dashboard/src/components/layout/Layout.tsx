import type { ReactNode } from 'react';
import { Header } from './Header';
import { useHealth } from '../../api/hooks';

interface LayoutProps {
  children: ReactNode;
  namespace: string;
  onNamespaceChange: (ns: string) => void;
  onSettingsClick: () => void;
  theme: 'light' | 'dark';
  onThemeToggle: () => void;
}

export function Layout({ children, namespace, onNamespaceChange, onSettingsClick, theme, onThemeToggle }: LayoutProps) {
  const { data: health } = useHealth();

  return (
    <div className="min-h-screen bg-gray-50 dark:bg-gray-900 transition-colors">
      <Header
        namespace={namespace}
        onNamespaceChange={onNamespaceChange}
        onSettingsClick={onSettingsClick}
        theme={theme}
        onThemeToggle={onThemeToggle}
      />

      <main className="p-6">
        {children}
      </main>

      {/* Status Bar */}
      <footer className="fixed bottom-0 left-0 right-0 bg-white dark:bg-gray-800 border-t border-gray-200 dark:border-gray-700 px-6 py-2 transition-colors">
        <div className="flex items-center justify-between text-sm">
          <div className="flex items-center gap-4">
            <span className="flex items-center gap-2">
              <span
                className={`w-2 h-2 rounded-full ${
                  health?.status === 'healthy' ? 'bg-green-500' : 'bg-red-500'
                }`}
              />
              <span className="text-gray-600 dark:text-gray-300">
                {health?.status === 'healthy' ? 'Connected' : 'Disconnected'}
              </span>
            </span>
            {health?.storage && (
              <span className="text-gray-500 dark:text-gray-400">Storage: {health.storage}</span>
            )}
          </div>
          <span className="text-gray-400 dark:text-gray-500">minimemory-dashboard v1.0.0</span>
        </div>
      </footer>
    </div>
  );
}
