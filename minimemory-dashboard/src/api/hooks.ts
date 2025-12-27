// React Query hooks for minimemory API

import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { getClient } from './client';
import { authFetch } from './auth';
import type {
  AgentToken,
  AgentTokenListResponse,
  AgentTokenStats,
  CreateAgentTokenParams,
  UpdateAgentTokenParams,
} from './types';

// Health
export function useHealth() {
  return useQuery({
    queryKey: ['health'],
    queryFn: () => getClient().getHealth(),
    refetchInterval: 30000, // Every 30 seconds
  });
}

// Stats
export function useStats(namespace?: string) {
  return useQuery({
    queryKey: ['stats', namespace],
    queryFn: () => getClient().getStats(namespace),
    refetchInterval: 10000, // Every 10 seconds
  });
}

// Namespaces
export function useNamespaces() {
  return useQuery({
    queryKey: ['namespaces'],
    queryFn: () => getClient().getNamespaces(),
  });
}

export function useCreateNamespace() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ name, dimensions }: { name: string; dimensions: number }) =>
      getClient().createNamespace(name, dimensions),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['namespaces'] });
    },
  });
}

export function useDeleteNamespace() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (name: string) => getClient().deleteNamespace(name),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['namespaces'] });
    },
  });
}

// Memories
export function useRecall(
  params: {
    keywords?: string;
    mode?: 'vector' | 'keyword' | 'hybrid';
    limit?: number;
    type?: string;
    minImportance?: number;
  },
  namespace?: string,
  enabled = true
) {
  return useQuery({
    queryKey: ['recall', params, namespace],
    queryFn: () => getClient().recall(params, namespace),
    enabled: enabled && (!!params.keywords || !!params.type),
  });
}

export function useRemember() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({
      params,
      namespace,
    }: {
      params: {
        content: string;
        type?: 'episodic' | 'semantic' | 'working';
        importance?: number;
        metadata?: Record<string, unknown>;
      };
      namespace?: string;
    }) => getClient().remember(params, namespace),
    onSuccess: (_, { namespace }) => {
      queryClient.invalidateQueries({ queryKey: ['stats', namespace] });
    },
  });
}

export function useSearchMemories() {
  return useMutation({
    mutationFn: ({
      params,
      namespace,
    }: {
      params: {
        keywords?: string;
        mode?: 'vector' | 'keyword' | 'hybrid';
        limit?: number;
        type?: string;
      };
      namespace?: string;
    }) => getClient().recall(params, namespace),
  });
}

export function useDeleteMemory() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, namespace }: { id: string; namespace?: string }) =>
      getClient().deleteMemory(id, namespace),
    onSuccess: (_, { namespace }) => {
      queryClient.invalidateQueries({ queryKey: ['stats', namespace] });
      queryClient.invalidateQueries({ queryKey: ['recall'] });
    },
  });
}

export function useClearNamespace() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (namespace: string) => getClient().clearNamespace(namespace),
    onSuccess: (_, namespace) => {
      queryClient.invalidateQueries({ queryKey: ['stats', namespace] });
      queryClient.invalidateQueries({ queryKey: ['recall'] });
    },
  });
}

// Export
export function useExportMemories() {
  return useMutation({
    mutationFn: (namespace?: string) => getClient().exportMemories(namespace),
  });
}

// ============ Agent Token Hooks ============

async function fetchAgentTokens(
  options?: { active?: boolean; limit?: number; offset?: number }
): Promise<AgentTokenListResponse> {
  const params = new URLSearchParams();
  if (options?.active !== undefined) params.set('active', String(options.active));
  if (options?.limit) params.set('limit', String(options.limit));
  if (options?.offset) params.set('offset', String(options.offset));

  const query = params.toString();
  const response = await authFetch(`/api/v1/agent-tokens${query ? `?${query}` : ''}`);

  if (!response.ok) {
    const error = await response.json().catch(() => ({}));
    throw new Error(error.error || 'Failed to fetch agent tokens');
  }

  return response.json();
}

async function fetchAgentTokenStats(): Promise<{ stats: AgentTokenStats }> {
  const response = await authFetch('/api/v1/agent-tokens/stats');

  if (!response.ok) {
    const error = await response.json().catch(() => ({}));
    throw new Error(error.error || 'Failed to fetch agent token stats');
  }

  return response.json();
}

export function useAgentTokens(options?: { active?: boolean; limit?: number; offset?: number }) {
  return useQuery({
    queryKey: ['agent-tokens', options],
    queryFn: () => fetchAgentTokens(options),
  });
}

export function useAgentTokenStats() {
  return useQuery({
    queryKey: ['agent-token-stats'],
    queryFn: fetchAgentTokenStats,
  });
}

export function useCreateAgentToken() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (params: CreateAgentTokenParams): Promise<{ token: AgentToken; message: string }> => {
      const response = await authFetch('/api/v1/agent-tokens', {
        method: 'POST',
        body: JSON.stringify(params),
      });

      if (!response.ok) {
        const error = await response.json().catch(() => ({}));
        throw new Error(error.error || 'Failed to create agent token');
      }

      return response.json();
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['agent-tokens'] });
      queryClient.invalidateQueries({ queryKey: ['agent-token-stats'] });
    },
  });
}

export function useUpdateAgentToken() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async ({
      id,
      updates,
    }: {
      id: string;
      updates: UpdateAgentTokenParams;
    }): Promise<{ token: AgentToken }> => {
      const response = await authFetch(`/api/v1/agent-tokens/${id}`, {
        method: 'PATCH',
        body: JSON.stringify(updates),
      });

      if (!response.ok) {
        const error = await response.json().catch(() => ({}));
        throw new Error(error.error || 'Failed to update agent token');
      }

      return response.json();
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['agent-tokens'] });
    },
  });
}

export function useDeleteAgentToken() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (id: string): Promise<{ success: boolean }> => {
      const response = await authFetch(`/api/v1/agent-tokens/${id}`, {
        method: 'DELETE',
      });

      if (!response.ok) {
        const error = await response.json().catch(() => ({}));
        throw new Error(error.error || 'Failed to delete agent token');
      }

      return response.json();
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['agent-tokens'] });
      queryClient.invalidateQueries({ queryKey: ['agent-token-stats'] });
    },
  });
}

export function useToggleAgentToken() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (id: string): Promise<{ token: AgentToken; message: string }> => {
      const response = await authFetch(`/api/v1/agent-tokens/${id}/toggle`, {
        method: 'POST',
      });

      if (!response.ok) {
        const error = await response.json().catch(() => ({}));
        throw new Error(error.error || 'Failed to toggle agent token');
      }

      return response.json();
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['agent-tokens'] });
      queryClient.invalidateQueries({ queryKey: ['agent-token-stats'] });
    },
  });
}
