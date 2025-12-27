import { createContext, useContext, useState, useEffect, useCallback, type ReactNode } from 'react';
import type { User, TenantInfo } from '../api/auth';
import {
  login as apiLogin,
  register as apiRegister,
  logout as apiLogout,
  getMe,
  isLoggedIn,
  getStoredUser,
  getStoredTenants,
  getActiveTenantId,
  setActiveTenantId,
  clearAuth,
} from '../api/auth';

interface AuthContextType {
  user: User | null;
  tenants: TenantInfo[];
  activeTenantId: string | null;
  isAuthenticated: boolean;
  isLoading: boolean;
  login: (email: string, password: string) => Promise<void>;
  register: (email: string, password: string, name?: string) => Promise<void>;
  logout: () => Promise<void>;
  setActiveTenant: (tenantId: string) => void;
  refreshUser: () => Promise<void>;
}

const AuthContext = createContext<AuthContextType | null>(null);

export function AuthProvider({ children }: { children: ReactNode }) {
  const [user, setUser] = useState<User | null>(null);
  const [tenants, setTenants] = useState<TenantInfo[]>([]);
  const [activeTenantId, setActiveTenantIdState] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(true);

  // Initialize auth state from localStorage
  useEffect(() => {
    const initAuth = async () => {
      if (isLoggedIn()) {
        const storedUser = getStoredUser();
        const storedTenants = getStoredTenants();
        const storedActiveTenant = getActiveTenantId();

        if (storedUser) {
          setUser(storedUser);
          setTenants(storedTenants);
          setActiveTenantIdState(storedActiveTenant);

          // Refresh user data in background
          try {
            const data = await getMe();
            setUser(data.user);
            setTenants(data.tenants);
          } catch {
            // Token expired, clear auth
            clearAuth();
            setUser(null);
            setTenants([]);
            setActiveTenantIdState(null);
          }
        }
      }
      setIsLoading(false);
    };

    initAuth();
  }, []);

  const login = useCallback(async (email: string, password: string) => {
    const data = await apiLogin(email, password);
    setUser(data.user);
    setTenants(data.tenants);
    if (data.tenants.length > 0) {
      setActiveTenantIdState(data.tenants[0].id);
    }
  }, []);

  const register = useCallback(async (email: string, password: string, name?: string) => {
    const data = await apiRegister(email, password, name);
    setUser(data.user);
    setTenants(data.tenants);
    if (data.tenants.length > 0) {
      setActiveTenantIdState(data.tenants[0].id);
    }
  }, []);

  const logout = useCallback(async () => {
    await apiLogout();
    setUser(null);
    setTenants([]);
    setActiveTenantIdState(null);
  }, []);

  const setActiveTenant = useCallback((tenantId: string) => {
    setActiveTenantId(tenantId);
    setActiveTenantIdState(tenantId);
  }, []);

  const refreshUser = useCallback(async () => {
    try {
      const data = await getMe();
      setUser(data.user);
      setTenants(data.tenants);
    } catch {
      clearAuth();
      setUser(null);
      setTenants([]);
      setActiveTenantIdState(null);
    }
  }, []);

  const value: AuthContextType = {
    user,
    tenants,
    activeTenantId,
    isAuthenticated: !!user,
    isLoading,
    login,
    register,
    logout,
    setActiveTenant,
    refreshUser,
  };

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}

export function useAuth() {
  const context = useContext(AuthContext);
  if (!context) {
    throw new Error('useAuth must be used within an AuthProvider');
  }
  return context;
}
