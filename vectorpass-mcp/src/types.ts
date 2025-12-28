/**
 * VectorPass MCP Server - Type Definitions
 */

// Environment bindings
export interface Env {
  // KV Namespaces
  OAUTH_CODES: KVNamespace;
  OAUTH_TOKENS: KVNamespace;
  OAUTH_SESSIONS: KVNamespace;
  USERS: KVNamespace;

  // Environment variables
  VECTORPASS_API_URL: string;
  MCP_SERVER_NAME: string;
  MCP_SERVER_VERSION: string;
  ALLOWED_REDIRECT_URIS: string;

  // Secrets
  EMAIL_API_KEY: string;
  JWT_SECRET: string;

  // AI binding for email (optional, reuse from VectorPass)
  AI?: any;
}

// OAuth Types
export interface OAuthSession {
  email?: string;
  userId?: string;
  clientId: string;
  redirectUri: string;
  state: string;
  codeChallenge: string;
  codeChallengeMethod: string;
  verified: boolean;
  createdAt: number;
}

export interface OAuthCode {
  userId: string;
  apiKey: string;
  clientId: string;
  redirectUri: string;
  codeChallenge: string;
  codeChallengeMethod: string;
  expiresAt: number;
}

export interface OAuthToken {
  userId: string;
  apiKey: string;
  email: string;
  tier: string;
  expiresAt: number;
}

export interface RefreshToken {
  userId: string;
  expiresAt: number;
}

// Verification code (same structure as VectorPass)
export interface VerificationData {
  code: string;
  attempts: number;
  createdAt: number;
}

// User from VectorPass (simplified)
export interface User {
  id: string;
  email: string;
  apiKey: string;
  tier: 'free' | 'starter' | 'pro' | 'business';
  createdAt: string;
  referralCode?: string;
}

// Token response
export interface TokenResponse {
  access_token: string;
  token_type: 'Bearer';
  expires_in: number;
  refresh_token?: string;
}

// OAuth error response
export interface OAuthError {
  error: string;
  error_description?: string;
}

// MCP Tool definitions
export interface MCPTool {
  name: string;
  description: string;
  inputSchema: {
    type: 'object';
    properties: Record<string, any>;
    required?: string[];
  };
}

// VectorPass API response
export interface VectorPassResponse<T = any> {
  success: boolean;
  data?: T;
  error?: string;
}
