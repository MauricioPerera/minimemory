/**
 * Rate Limiting Middleware
 */
import { Context, Next } from 'hono';
export interface RateLimitConfig {
    /** Default requests per window */
    defaultLimit?: number;
    /** Default window in seconds */
    defaultWindow?: number;
    /** Key generator function */
    keyGenerator?: (c: Context) => string;
    /** Skip rate limiting for certain conditions */
    skip?: (c: Context) => boolean;
    /** Custom error handler */
    onLimit?: (c: Context, info: RateLimitInfo) => Response;
}
export interface RateLimitInfo {
    limit: number;
    remaining: number;
    reset: number;
    retryAfter: number;
}
/**
 * In-memory rate limiter
 * For production, use Redis or Cloudflare Rate Limiting
 * Note: Workers-compatible - no setInterval in global scope
 */
export declare class RateLimiter {
    private entries;
    private lastCleanup;
    private cleanupInterval;
    constructor();
    /**
     * Check and increment rate limit
     */
    check(key: string, limit: number, windowSeconds: number): RateLimitInfo;
    /**
     * Clean up expired entries
     */
    private cleanup;
    /**
     * Reset a specific key
     */
    reset(key: string): void;
    /**
     * Get current stats
     */
    stats(): {
        activeKeys: number;
    };
    /**
     * Start periodic cleanup (for Node.js environments)
     * Not needed for Workers - they are stateless
     */
    startCleanup(): void;
    /**
     * Destroy the rate limiter
     */
    destroy(): void;
}
export declare const defaultRateLimiter: RateLimiter;
/**
 * Create rate limiting middleware
 */
export declare function createRateLimitMiddleware(config?: RateLimitConfig): (c: Context, next: Next) => Promise<Response | undefined>;
/**
 * Create a stricter rate limit for specific routes
 */
export declare function strictRateLimit(limit: number, windowSeconds: number): (c: Context, next: Next) => Promise<(Response & import("hono").TypedResponse<{
    error: string;
    message: string;
    retryAfter: number;
}, 429, "json">) | undefined>;
//# sourceMappingURL=rateLimit.d.ts.map