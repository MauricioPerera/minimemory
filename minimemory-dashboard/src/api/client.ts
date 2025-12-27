// API Client for minimemory-service

import type {
  StatsResponse,
  RecallResponse,
  NamespacesResponse,
  HealthResponse,
  ExportResponse,
  Memory,
} from './types';

const API_URL = import.meta.env.VITE_API_URL || 'https://minimemory-service.rckflr.workers.dev';

class MinimemoryClient {
  private apiKey: string;

  constructor(apiKey: string) {
    this.apiKey = apiKey;
  }

  private async request<T>(
    endpoint: string,
    options: RequestInit = {},
    namespace?: string
  ): Promise<T> {
    const headers: Record<string, string> = {
      'Content-Type': 'application/json',
      'X-API-Key': this.apiKey,
      ...((options.headers as Record<string, string>) || {}),
    };

    if (namespace) {
      headers['X-Namespace'] = namespace;
    }

    const response = await fetch(`${API_URL}${endpoint}`, {
      ...options,
      headers,
    });

    if (!response.ok) {
      const error = await response.json().catch(() => ({ error: 'Unknown error' }));
      throw new Error(error.error || `API Error: ${response.status}`);
    }

    return response.json();
  }

  // Health
  async getHealth(): Promise<HealthResponse> {
    return this.request('/health');
  }

  // Stats
  async getStats(namespace?: string): Promise<StatsResponse> {
    return this.request('/api/v1/stats', {}, namespace);
  }

  // Namespaces
  async getNamespaces(): Promise<NamespacesResponse> {
    return this.request('/api/v1/namespaces');
  }

  async createNamespace(name: string, dimensions: number): Promise<{ success: boolean }> {
    return this.request('/api/v1/namespaces', {
      method: 'POST',
      body: JSON.stringify({ name, dimensions }),
    });
  }

  async deleteNamespace(name: string): Promise<{ success: boolean }> {
    return this.request(`/api/v1/namespaces/${encodeURIComponent(name)}`, {
      method: 'DELETE',
    });
  }

  // Memories
  async remember(
    params: {
      content: string;
      type?: 'episodic' | 'semantic' | 'working';
      importance?: number;
      metadata?: Record<string, unknown>;
    },
    namespace?: string
  ): Promise<{ success: boolean; id: string; persisted: boolean }> {
    return this.request(
      '/api/v1/remember',
      {
        method: 'POST',
        body: JSON.stringify(params),
      },
      namespace
    );
  }

  async recall(
    params: {
      keywords?: string;
      embedding?: number[];
      mode?: 'vector' | 'keyword' | 'hybrid';
      limit?: number;
      type?: string;
      minImportance?: number;
    },
    namespace?: string
  ): Promise<RecallResponse> {
    return this.request(
      '/api/v1/recall',
      {
        method: 'POST',
        body: JSON.stringify(params),
      },
      namespace
    );
  }

  async getMemory(id: string, namespace?: string): Promise<{ success: boolean; memory: Memory }> {
    return this.request(`/api/v1/memory/${encodeURIComponent(id)}`, {}, namespace);
  }

  async deleteMemory(id: string, namespace?: string): Promise<{ success: boolean }> {
    return this.request(
      `/api/v1/forget/${encodeURIComponent(id)}`,
      { method: 'DELETE' },
      namespace
    );
  }

  async clearNamespace(namespace: string): Promise<{ success: boolean }> {
    return this.request('/api/v1/clear', { method: 'DELETE' }, namespace);
  }

  // Export
  async exportMemories(namespace?: string): Promise<ExportResponse> {
    return this.request('/api/v1/export', { method: 'POST' }, namespace);
  }
}

// Singleton instance
let clientInstance: MinimemoryClient | null = null;

export function getClient(): MinimemoryClient {
  const apiKey = localStorage.getItem('minimemory_api_key') || 'mm_dev_key_12345';
  if (!clientInstance || (clientInstance as any).apiKey !== apiKey) {
    clientInstance = new MinimemoryClient(apiKey);
  }
  return clientInstance;
}

export function setApiKey(key: string) {
  localStorage.setItem('minimemory_api_key', key);
  clientInstance = null; // Force recreation
}

export function getApiKey(): string {
  return localStorage.getItem('minimemory_api_key') || 'mm_dev_key_12345';
}

export { MinimemoryClient };
