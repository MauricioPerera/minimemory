// Authentication API for minimemory-service

const API_URL = import.meta.env.VITE_API_URL || 'https://minimemory-service.rckflr.workers.dev';

export interface User {
  id: string;
  email: string;
  name: string | null;
}

export interface TenantInfo {
  id: string;
  name: string;
  role: 'owner' | 'admin' | 'member' | 'viewer';
}

export interface AuthTokens {
  accessToken: string;
  refreshToken: string;
}

export interface LoginResponse {
  success: boolean;
  accessToken: string;
  refreshToken: string;
  user: User;
  tenants: TenantInfo[];
}

export interface RegisterResponse extends LoginResponse {}

export interface RefreshResponse {
  success: boolean;
  accessToken: string;
  refreshToken: string;
}

export interface MeResponse {
  user: User & {
    createdAt: number;
    lastLogin: number | null;
  };
  tenants: Array<TenantInfo & {
    plan: string;
    maxMemories: number;
    maxNamespaces: number;
    createdAt: number;
  }>;
}

// Token storage keys
const ACCESS_TOKEN_KEY = 'minimemory_access_token';
const REFRESH_TOKEN_KEY = 'minimemory_refresh_token';
const USER_KEY = 'minimemory_user';
const TENANTS_KEY = 'minimemory_tenants';
const ACTIVE_TENANT_KEY = 'minimemory_active_tenant';

/**
 * Store authentication data
 */
export function storeAuth(data: LoginResponse): void {
  localStorage.setItem(ACCESS_TOKEN_KEY, data.accessToken);
  localStorage.setItem(REFRESH_TOKEN_KEY, data.refreshToken);
  localStorage.setItem(USER_KEY, JSON.stringify(data.user));
  localStorage.setItem(TENANTS_KEY, JSON.stringify(data.tenants));

  // Set first tenant as active if not set
  if (!localStorage.getItem(ACTIVE_TENANT_KEY) && data.tenants.length > 0) {
    localStorage.setItem(ACTIVE_TENANT_KEY, data.tenants[0].id);
  }
}

/**
 * Clear authentication data
 */
export function clearAuth(): void {
  localStorage.removeItem(ACCESS_TOKEN_KEY);
  localStorage.removeItem(REFRESH_TOKEN_KEY);
  localStorage.removeItem(USER_KEY);
  localStorage.removeItem(TENANTS_KEY);
  localStorage.removeItem(ACTIVE_TENANT_KEY);
}

/**
 * Get stored access token
 */
export function getAccessToken(): string | null {
  return localStorage.getItem(ACCESS_TOKEN_KEY);
}

/**
 * Get stored refresh token
 */
export function getRefreshToken(): string | null {
  return localStorage.getItem(REFRESH_TOKEN_KEY);
}

/**
 * Get stored user
 */
export function getStoredUser(): User | null {
  const data = localStorage.getItem(USER_KEY);
  return data ? JSON.parse(data) : null;
}

/**
 * Get stored tenants
 */
export function getStoredTenants(): TenantInfo[] {
  const data = localStorage.getItem(TENANTS_KEY);
  return data ? JSON.parse(data) : [];
}

/**
 * Get active tenant ID
 */
export function getActiveTenantId(): string | null {
  return localStorage.getItem(ACTIVE_TENANT_KEY);
}

/**
 * Set active tenant
 */
export function setActiveTenantId(tenantId: string): void {
  localStorage.setItem(ACTIVE_TENANT_KEY, tenantId);
}

/**
 * Check if user is logged in
 */
export function isLoggedIn(): boolean {
  return !!getAccessToken() && !!getRefreshToken();
}

/**
 * Register a new user
 */
export async function register(
  email: string,
  password: string,
  name?: string
): Promise<RegisterResponse> {
  const response = await fetch(`${API_URL}/api/v1/auth/register`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ email, password, name }),
  });

  const data = await response.json();

  if (!response.ok) {
    throw new Error(data.error || 'Registration failed');
  }

  storeAuth(data);
  return data;
}

/**
 * Login user
 */
export async function login(email: string, password: string): Promise<LoginResponse> {
  const response = await fetch(`${API_URL}/api/v1/auth/login`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ email, password }),
  });

  const data = await response.json();

  if (!response.ok) {
    throw new Error(data.error || 'Login failed');
  }

  storeAuth(data);
  return data;
}

/**
 * Refresh access token
 */
export async function refreshAccessToken(): Promise<RefreshResponse | null> {
  const refreshToken = getRefreshToken();

  if (!refreshToken) {
    return null;
  }

  try {
    const response = await fetch(`${API_URL}/api/v1/auth/refresh`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ refreshToken }),
    });

    if (!response.ok) {
      clearAuth();
      return null;
    }

    const data = await response.json();

    // Update stored tokens
    localStorage.setItem(ACCESS_TOKEN_KEY, data.accessToken);
    localStorage.setItem(REFRESH_TOKEN_KEY, data.refreshToken);

    return data;
  } catch {
    clearAuth();
    return null;
  }
}

/**
 * Logout user
 */
export async function logout(): Promise<void> {
  const refreshToken = getRefreshToken();

  if (refreshToken) {
    try {
      await fetch(`${API_URL}/api/v1/auth/logout`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ refreshToken }),
      });
    } catch {
      // Ignore errors on logout
    }
  }

  clearAuth();
}

/**
 * Get current user profile
 */
export async function getMe(): Promise<MeResponse> {
  const accessToken = getAccessToken();

  if (!accessToken) {
    throw new Error('Not authenticated');
  }

  const response = await fetch(`${API_URL}/api/v1/auth/me`, {
    headers: { Authorization: `Bearer ${accessToken}` },
  });

  if (!response.ok) {
    if (response.status === 401) {
      // Try to refresh token
      const refreshed = await refreshAccessToken();
      if (refreshed) {
        return getMe();
      }
      throw new Error('Session expired');
    }
    const data = await response.json().catch(() => ({}));
    throw new Error(data.error || 'Failed to get profile');
  }

  return response.json();
}

/**
 * Authenticated fetch wrapper
 * Automatically adds Authorization header and handles token refresh
 */
export async function authFetch(
  endpoint: string,
  options: RequestInit = {}
): Promise<Response> {
  let accessToken = getAccessToken();

  if (!accessToken) {
    throw new Error('Not authenticated');
  }

  const headers = new Headers(options.headers);
  headers.set('Authorization', `Bearer ${accessToken}`);
  headers.set('Content-Type', 'application/json');

  // Add tenant header if active
  const tenantId = getActiveTenantId();
  if (tenantId) {
    headers.set('X-Tenant-Id', tenantId);
  }

  let response = await fetch(`${API_URL}${endpoint}`, {
    ...options,
    headers,
  });

  // If 401, try to refresh token and retry
  if (response.status === 401) {
    const refreshed = await refreshAccessToken();
    if (refreshed) {
      headers.set('Authorization', `Bearer ${refreshed.accessToken}`);
      response = await fetch(`${API_URL}${endpoint}`, {
        ...options,
        headers,
      });
    }
  }

  return response;
}
