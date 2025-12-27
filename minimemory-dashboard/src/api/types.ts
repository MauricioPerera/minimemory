// API Types for minimemory-service

export interface Memory {
  id: string;
  type: 'episodic' | 'semantic' | 'working';
  content: string;
  importance: number;
  metadata?: Record<string, unknown>;
  createdAt: number;
}

export interface MemoryStats {
  total: number;
  byType: {
    episodic: number;
    semantic: number;
    working: number;
  };
  averageImportance: number;
  oldestMemory?: number;
  newestMemory?: number;
}

export interface StatsResponse {
  success: boolean;
  namespace: string;
  stats: MemoryStats;
  source: 'memory' | 'd1';
}

export interface RecallResult {
  id: string;
  type: string;
  content: string;
  score: number;
  vectorSimilarity?: number;
  keywordScore?: number;
  importance: number;
  metadata?: Record<string, unknown>;
  createdAt: number;
}

export interface RecallResponse {
  success: boolean;
  count: number;
  results: RecallResult[];
}

export interface Namespace {
  name: string;
  dimensions: number;
}

export interface NamespacesResponse {
  success: boolean;
  namespaces: Namespace[];
  count: number;
  storage: string;
}

export interface HealthResponse {
  status: string;
  timestamp: string;
  storage: string;
}

export interface ExportResponse {
  success: boolean;
  namespace: string;
  data: {
    memories: Memory[];
  };
  source: string;
}

export interface ApiError {
  error: string;
  message?: string;
}

// Agent Token Types
export type AgentPermission = 'read' | 'write';

export interface AgentToken {
  id: string;
  userId: string;
  tenantId?: string;
  name: string;
  description?: string;
  allowedMemories: string[];
  permissions: AgentPermission[];
  isActive: boolean;
  lastUsedAt?: number;
  useCount: number;
  expiresAt?: number;
  createdAt: number;
  updatedAt: number;
}

export interface AgentTokenListResponse {
  tokens: AgentToken[];
  total: number;
  hasMore: boolean;
}

export interface AgentTokenStats {
  total: number;
  active: number;
  inactive: number;
  expired: number;
  totalUseCount: number;
}

export interface CreateAgentTokenParams {
  name: string;
  description?: string;
  allowedMemories?: string[];
  permissions?: AgentPermission[];
  expiresAt?: number;
}

export interface UpdateAgentTokenParams {
  name?: string;
  description?: string;
  allowedMemories?: string[];
  permissions?: AgentPermission[];
  isActive?: boolean;
  expiresAt?: number | null;
}
