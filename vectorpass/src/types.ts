/**
 * VectorPass Type Definitions
 */

// Environment bindings
export interface Env {
    AI: any;
    USERS: KVNamespace;
    VECTORS: KVNamespace;
    RATE_LIMITS: KVNamespace;
    ENVIRONMENT: string;
    FREE_TIER_MAX_VECTORS: string;
    FREE_TIER_SEARCHES_PER_DAY: string;
    STARTER_MAX_VECTORS: string;
    STARTER_SEARCHES_PER_DAY: string;
    PRO_MAX_VECTORS: string;
    PRO_SEARCHES_PER_DAY: string;
}

// Subscription tiers
export type Tier = 'free' | 'starter' | 'pro' | 'business';

// User data stored in KV
export interface User {
    id: string;
    email: string;
    apiKey: string;
    tier: Tier;
    createdAt: string;
    stripeCustomerId?: string;
    stripeSubscriptionId?: string;
    referralCode?: string;
    referredBy?: string;
    referralCount: number;
}

// Tier limits
export interface TierLimits {
    maxVectors: number;
    searchesPerDay: number;
    maxDatabases: number;
    batchSize: number;
}

export const TIER_LIMITS: Record<Tier, TierLimits> = {
    free: {
        maxVectors: 1000,
        searchesPerDay: 100,
        maxDatabases: 1,
        batchSize: 10
    },
    starter: {
        maxVectors: 50000,
        searchesPerDay: 10000,
        maxDatabases: 5,
        batchSize: 100
    },
    pro: {
        maxVectors: 500000,
        searchesPerDay: 100000,
        maxDatabases: 20,
        batchSize: 500
    },
    business: {
        maxVectors: 5000000,
        searchesPerDay: 1000000,
        maxDatabases: 100,
        batchSize: 1000
    }
};

// Rate limit tracking
export interface RateLimitData {
    count: number;
    resetAt: number;  // Unix timestamp
}

// Vector database metadata
export interface DatabaseInfo {
    id: string;
    name: string;
    userId: string;
    vectorCount: number;
    dimensions: number;
    createdAt: string;
    updatedAt: string;
}

// API request/response types
export interface IndexRequest {
    id: string;
    text: string;
    metadata?: Record<string, any>;
}

export interface BatchIndexRequest {
    items: IndexRequest[];
}

export interface SearchRequest {
    query: string;
    k?: number;
    filter?: Record<string, any>;
}

export interface SearchResult {
    id: string;
    distance: number;
    metadata?: Record<string, any>;
}

export interface KeywordSearchRequest {
    query: string;
    k?: number;
}

export interface KeywordResult {
    id: string;
    score: number;
}

export interface ApiResponse<T = any> {
    success: boolean;
    data?: T;
    error?: string;
}

export interface StatsResponse {
    vectorCount: number;
    dimensions: number;
    tier: Tier;
    limits: TierLimits;
    usage: {
        searchesToday: number;
        vectorsUsed: number;
    };
}

// API Key format: vp_live_xxxx or vp_test_xxxx
export function generateApiKey(isTest: boolean = false): string {
    const prefix = isTest ? 'vp_test_' : 'vp_live_';
    const chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
    let key = '';
    for (let i = 0; i < 32; i++) {
        key += chars.charAt(Math.floor(Math.random() * chars.length));
    }
    return prefix + key;
}

// Generate referral code
export function generateReferralCode(): string {
    const chars = 'ABCDEFGHJKLMNPQRSTUVWXYZ23456789';  // Avoid ambiguous chars
    let code = '';
    for (let i = 0; i < 8; i++) {
        code += chars.charAt(Math.floor(Math.random() * chars.length));
    }
    return code;
}
