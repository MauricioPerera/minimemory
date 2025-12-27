import { useState } from 'react';
import { Link, useLocation } from 'react-router-dom';
import { Brain, Settings, ChevronDown, Moon, Sun, LogOut, User, BookOpen, Key } from 'lucide-react';
import { useNamespaces } from '../../api/hooks';
import { useAuth } from '../../contexts/AuthContext';
import { cn } from '../../lib/utils';

interface HeaderProps {
  namespace: string;
  onNamespaceChange: (ns: string) => void;
  onSettingsClick: () => void;
  theme: 'light' | 'dark';
  onThemeToggle: () => void;
}

export function Header({ namespace, onNamespaceChange, onSettingsClick, theme, onThemeToggle }: HeaderProps) {
  const { data: namespacesData } = useNamespaces();
  const { user, tenants, activeTenantId, setActiveTenant, logout } = useAuth();
  const location = useLocation();
  const [isOpen, setIsOpen] = useState(false);
  const [isTenantOpen, setIsTenantOpen] = useState(false);
  const [isUserOpen, setIsUserOpen] = useState(false);

  const namespaces = namespacesData?.namespaces || [];
  const activeTenant = tenants.find(t => t.id === activeTenantId);

  const handleLogout = async () => {
    await logout();
  };

  return (
    <header className="bg-white dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700 px-6 py-4 transition-colors">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-6">
          <div className="flex items-center gap-3">
            <Brain className="w-8 h-8 text-indigo-600 dark:text-indigo-400" />
            <h1 className="text-xl font-bold text-gray-900 dark:text-white">minimemory</h1>
          </div>

          {/* Navigation */}
          <nav className="flex items-center gap-1">
            <Link
              to="/"
              className={cn(
                'px-3 py-2 rounded-lg text-sm font-medium transition-colors',
                location.pathname === '/'
                  ? 'bg-indigo-100 dark:bg-indigo-900/30 text-indigo-700 dark:text-indigo-300'
                  : 'text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white hover:bg-gray-100 dark:hover:bg-gray-700'
              )}
            >
              Dashboard
            </Link>
            <Link
              to="/tokens"
              className={cn(
                'flex items-center gap-1.5 px-3 py-2 rounded-lg text-sm font-medium transition-colors',
                location.pathname === '/tokens'
                  ? 'bg-indigo-100 dark:bg-indigo-900/30 text-indigo-700 dark:text-indigo-300'
                  : 'text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white hover:bg-gray-100 dark:hover:bg-gray-700'
              )}
            >
              <Key className="w-4 h-4" />
              Tokens
            </Link>
            <Link
              to="/docs"
              className={cn(
                'flex items-center gap-1.5 px-3 py-2 rounded-lg text-sm font-medium transition-colors',
                location.pathname === '/docs'
                  ? 'bg-indigo-100 dark:bg-indigo-900/30 text-indigo-700 dark:text-indigo-300'
                  : 'text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white hover:bg-gray-100 dark:hover:bg-gray-700'
              )}
            >
              <BookOpen className="w-4 h-4" />
              Docs
            </Link>
          </nav>
        </div>

        <div className="flex items-center gap-4">
          {/* Tenant Selector */}
          {tenants.length > 1 && (
            <div className="relative">
              <button
                onClick={() => setIsTenantOpen(!isTenantOpen)}
                className="flex items-center gap-2 px-3 py-2 bg-indigo-50 dark:bg-indigo-900/30 rounded-lg hover:bg-indigo-100 dark:hover:bg-indigo-900/50 transition-colors"
              >
                <span className="text-sm font-medium text-indigo-700 dark:text-indigo-300">
                  {activeTenant?.name || 'Select Tenant'}
                </span>
                <ChevronDown className={cn('w-4 h-4 text-indigo-500 transition-transform', isTenantOpen && 'rotate-180')} />
              </button>

              {isTenantOpen && (
                <div className="absolute right-0 mt-2 w-56 bg-white dark:bg-gray-800 rounded-lg shadow-lg border border-gray-200 dark:border-gray-700 z-20">
                  {tenants.map((tenant) => (
                    <button
                      key={tenant.id}
                      onClick={() => {
                        setActiveTenant(tenant.id);
                        setIsTenantOpen(false);
                      }}
                      className={cn(
                        'w-full px-4 py-2 text-left text-sm hover:bg-gray-100 dark:hover:bg-gray-700 first:rounded-t-lg last:rounded-b-lg transition-colors',
                        tenant.id === activeTenantId && 'bg-indigo-50 dark:bg-indigo-900/30'
                      )}
                    >
                      <div className="font-medium text-gray-900 dark:text-white">{tenant.name}</div>
                      <div className="text-xs text-gray-500 dark:text-gray-400 capitalize">{tenant.role}</div>
                    </button>
                  ))}
                </div>
              )}
            </div>
          )}

          {/* Namespace Selector */}
          <div className="relative">
            <button
              onClick={() => setIsOpen(!isOpen)}
              className="flex items-center gap-2 px-4 py-2 bg-gray-100 dark:bg-gray-700 rounded-lg hover:bg-gray-200 dark:hover:bg-gray-600 transition-colors"
            >
              <span className="text-sm font-medium text-gray-700 dark:text-gray-200">{namespace}</span>
              <ChevronDown className={cn('w-4 h-4 text-gray-500 dark:text-gray-400 transition-transform', isOpen && 'rotate-180')} />
            </button>

            {isOpen && (
              <div className="absolute right-0 mt-2 w-48 bg-white dark:bg-gray-800 rounded-lg shadow-lg border border-gray-200 dark:border-gray-700 z-10">
                {namespaces.map((ns) => (
                  <button
                    key={ns.name}
                    onClick={() => {
                      onNamespaceChange(ns.name);
                      setIsOpen(false);
                    }}
                    className={cn(
                      'w-full px-4 py-2 text-left text-sm hover:bg-gray-100 dark:hover:bg-gray-700 first:rounded-t-lg last:rounded-b-lg transition-colors',
                      ns.name === namespace && 'bg-indigo-50 dark:bg-indigo-900/30 text-indigo-700 dark:text-indigo-300'
                    )}
                  >
                    <div className="font-medium text-gray-900 dark:text-white">{ns.name}</div>
                    <div className="text-xs text-gray-500 dark:text-gray-400">{ns.dimensions}D</div>
                  </button>
                ))}
              </div>
            )}
          </div>

          {/* Theme Toggle */}
          <button
            onClick={onThemeToggle}
            className="p-2 text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors"
            title={theme === 'light' ? 'Switch to dark mode' : 'Switch to light mode'}
          >
            {theme === 'light' ? <Moon className="w-5 h-5" /> : <Sun className="w-5 h-5" />}
          </button>

          {/* Settings Button */}
          <button
            onClick={onSettingsClick}
            className="p-2 text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors"
          >
            <Settings className="w-5 h-5" />
          </button>

          {/* User Menu */}
          <div className="relative">
            <button
              onClick={() => setIsUserOpen(!isUserOpen)}
              className="flex items-center gap-2 p-2 text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors"
            >
              <User className="w-5 h-5" />
            </button>

            {isUserOpen && (
              <div className="absolute right-0 mt-2 w-56 bg-white dark:bg-gray-800 rounded-lg shadow-lg border border-gray-200 dark:border-gray-700 z-20">
                <div className="px-4 py-3 border-b border-gray-200 dark:border-gray-700">
                  <div className="font-medium text-gray-900 dark:text-white truncate">
                    {user?.name || 'User'}
                  </div>
                  <div className="text-sm text-gray-500 dark:text-gray-400 truncate">
                    {user?.email}
                  </div>
                </div>
                <button
                  onClick={handleLogout}
                  className="w-full px-4 py-2 text-left text-sm text-red-600 dark:text-red-400 hover:bg-red-50 dark:hover:bg-red-900/20 rounded-b-lg flex items-center gap-2 transition-colors"
                >
                  <LogOut className="w-4 h-4" />
                  Sign out
                </button>
              </div>
            )}
          </div>
        </div>
      </div>
    </header>
  );
}
