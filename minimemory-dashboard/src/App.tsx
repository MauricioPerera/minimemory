import { useState } from 'react';
import { BrowserRouter, Routes, Route } from 'react-router-dom';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { AuthProvider } from './contexts/AuthContext';
import { AuthGuard, GuestGuard } from './components/auth/AuthGuard';
import { Layout } from './components/layout/Layout';
import { Dashboard } from './pages/Dashboard';
import { Documentation } from './pages/Documentation';
import { AgentTokens } from './pages/AgentTokens';
import { Login } from './pages/Login';
import { Register } from './pages/Register';
import { SettingsModal } from './components/SettingsModal';
import { useTheme } from './hooks/useTheme';

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 5000,
      retry: 1,
    },
  },
});

function AuthenticatedApp() {
  const [namespace, setNamespace] = useState('default');
  const [showSettings, setShowSettings] = useState(false);
  const { theme, toggleTheme } = useTheme();

  return (
    <Layout
      namespace={namespace}
      onNamespaceChange={setNamespace}
      onSettingsClick={() => setShowSettings(true)}
      theme={theme}
      onThemeToggle={toggleTheme}
    >
      <Routes>
        <Route path="/" element={<Dashboard namespace={namespace} />} />
        <Route path="/tokens" element={<AgentTokens />} />
        <Route path="/docs" element={<Documentation />} />
      </Routes>

      <SettingsModal
        isOpen={showSettings}
        onClose={() => setShowSettings(false)}
        namespace={namespace}
      />
    </Layout>
  );
}

function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <AuthProvider>
        <BrowserRouter>
          <Routes>
            {/* Public routes */}
            <Route
              path="/login"
              element={
                <GuestGuard>
                  <Login />
                </GuestGuard>
              }
            />
            <Route
              path="/register"
              element={
                <GuestGuard>
                  <Register />
                </GuestGuard>
              }
            />

            {/* Protected routes */}
            <Route
              path="/*"
              element={
                <AuthGuard>
                  <AuthenticatedApp />
                </AuthGuard>
              }
            />
          </Routes>
        </BrowserRouter>
      </AuthProvider>
    </QueryClientProvider>
  );
}

export default App;
