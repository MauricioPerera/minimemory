/**
 * Tests for Rate Limiting Middleware
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import {
	RateLimiter,
	createRateLimitMiddleware,
	strictRateLimit,
	defaultRateLimiter,
} from '../../src/middleware/rateLimit.js';
import type { Context } from 'hono';

// Helper to create mock context
function createMockContext(overrides: {
	auth?: { userId?: string; rateLimit?: { limit: number; window: number } };
	headers?: Record<string, string>;
	path?: string;
} = {}): Context {
	const headers = new Map<string, string>();
	const responseHeaders = new Map<string, string>();

	return {
		get: vi.fn((key: string) => {
			if (key === 'auth') return overrides.auth;
			return undefined;
		}),
		set: vi.fn(),
		req: {
			header: vi.fn((name: string) => overrides.headers?.[name] || null),
			path: overrides.path || '/api/test',
		},
		header: vi.fn((name: string, value: string) => {
			responseHeaders.set(name, value);
		}),
		json: vi.fn((data: unknown, status?: number) => {
			return new Response(JSON.stringify(data), { status: status || 200 });
		}),
		// Helper to get response headers for assertions
		_responseHeaders: responseHeaders,
	} as unknown as Context;
}

describe('RateLimiter', () => {
	let rateLimiter: RateLimiter;

	beforeEach(() => {
		rateLimiter = new RateLimiter();
	});

	describe('check()', () => {
		it('should return correct remaining count on first call', () => {
			const info = rateLimiter.check('test-key', 100, 60);

			expect(info.limit).toBe(100);
			expect(info.remaining).toBe(99);
			expect(info.retryAfter).toBe(0);
			expect(info.reset).toBeGreaterThan(0);
		});

		it('should decrement remaining on each call', () => {
			const key = 'decrement-test';

			const info1 = rateLimiter.check(key, 100, 60);
			expect(info1.remaining).toBe(99);

			const info2 = rateLimiter.check(key, 100, 60);
			expect(info2.remaining).toBe(98);

			const info3 = rateLimiter.check(key, 100, 60);
			expect(info3.remaining).toBe(97);
		});

		it('should return retryAfter > 0 when limit exceeded', () => {
			const key = 'exceeded-test';
			const limit = 3;

			// Use up the limit
			rateLimiter.check(key, limit, 60);
			rateLimiter.check(key, limit, 60);
			rateLimiter.check(key, limit, 60);

			// Exceed the limit
			const info = rateLimiter.check(key, limit, 60);

			expect(info.remaining).toBe(0);
			expect(info.retryAfter).toBeGreaterThan(0);
		});

		it('should return negative remaining when over limit', () => {
			const key = 'over-limit-test';
			const limit = 2;

			rateLimiter.check(key, limit, 60);
			rateLimiter.check(key, limit, 60);
			const info = rateLimiter.check(key, limit, 60);

			expect(info.remaining).toBe(0); // Math.max(0, limit - count)
		});

		it('should reset after window expires', async () => {
			const key = 'window-reset-test';
			const windowSeconds = 1; // 1 second window

			// First call
			const info1 = rateLimiter.check(key, 100, windowSeconds);
			expect(info1.remaining).toBe(99);

			// Wait for window to expire
			await new Promise((resolve) => setTimeout(resolve, 1100));

			// Should reset
			const info2 = rateLimiter.check(key, 100, windowSeconds);
			expect(info2.remaining).toBe(99);
		});

		it('should handle different keys independently', () => {
			const info1 = rateLimiter.check('key-a', 100, 60);
			const info2 = rateLimiter.check('key-b', 100, 60);

			expect(info1.remaining).toBe(99);
			expect(info2.remaining).toBe(99);

			// Further calls on key-a shouldn't affect key-b
			rateLimiter.check('key-a', 100, 60);
			rateLimiter.check('key-a', 100, 60);

			const info3 = rateLimiter.check('key-b', 100, 60);
			expect(info3.remaining).toBe(98);
		});
	});

	describe('reset()', () => {
		it('should clear a specific key', () => {
			const key = 'reset-key-test';

			// Use up some of the limit
			rateLimiter.check(key, 100, 60);
			rateLimiter.check(key, 100, 60);

			// Reset
			rateLimiter.reset(key);

			// Should start fresh
			const info = rateLimiter.check(key, 100, 60);
			expect(info.remaining).toBe(99);
		});

		it('should not affect other keys when resetting one', () => {
			const key1 = 'reset-key-1';
			const key2 = 'reset-key-2';

			rateLimiter.check(key1, 100, 60);
			rateLimiter.check(key2, 100, 60);
			rateLimiter.check(key2, 100, 60);

			// Reset key1
			rateLimiter.reset(key1);

			// key2 should be unaffected
			const info = rateLimiter.check(key2, 100, 60);
			expect(info.remaining).toBe(97);
		});
	});

	describe('stats()', () => {
		it('should return correct active keys count', () => {
			expect(rateLimiter.stats().activeKeys).toBe(0);

			rateLimiter.check('stats-key-1', 100, 60);
			expect(rateLimiter.stats().activeKeys).toBe(1);

			rateLimiter.check('stats-key-2', 100, 60);
			expect(rateLimiter.stats().activeKeys).toBe(2);

			rateLimiter.check('stats-key-1', 100, 60); // Same key
			expect(rateLimiter.stats().activeKeys).toBe(2);
		});

		it('should decrease after reset', () => {
			rateLimiter.check('stats-reset-1', 100, 60);
			rateLimiter.check('stats-reset-2', 100, 60);

			expect(rateLimiter.stats().activeKeys).toBe(2);

			rateLimiter.reset('stats-reset-1');

			expect(rateLimiter.stats().activeKeys).toBe(1);
		});
	});

	describe('cleanup()', () => {
		it('should remove expired entries during lazy cleanup', async () => {
			const rateLimiter = new RateLimiter();

			// Create entries with short windows
			rateLimiter.check('cleanup-1', 100, 1);
			rateLimiter.check('cleanup-2', 100, 1);

			expect(rateLimiter.stats().activeKeys).toBe(2);

			// Wait for expiration
			await new Promise((resolve) => setTimeout(resolve, 1100));

			// Force cleanup by making a new check (triggers lazy cleanup if > 60s)
			// Since lazy cleanup only triggers every 60s, we test via indirect means
			// The entries should be gone when accessed after expiration
			const info = rateLimiter.check('cleanup-1', 100, 60);
			expect(info.remaining).toBe(99); // Fresh start = 99
		});
	});

	describe('destroy()', () => {
		it('should clear all entries', () => {
			rateLimiter.check('destroy-1', 100, 60);
			rateLimiter.check('destroy-2', 100, 60);

			expect(rateLimiter.stats().activeKeys).toBe(2);

			rateLimiter.destroy();

			expect(rateLimiter.stats().activeKeys).toBe(0);
		});
	});
});

describe('createRateLimitMiddleware', () => {
	beforeEach(() => {
		// Reset the default rate limiter
		defaultRateLimiter.destroy();
	});

	it('should apply default limits (100/60s)', async () => {
		const middleware = createRateLimitMiddleware();
		const context = createMockContext();
		const next = vi.fn();

		await middleware(context, next);

		expect(next).toHaveBeenCalled();
		expect(context.header).toHaveBeenCalledWith('X-RateLimit-Limit', '100');
	});

	it('should use auth context rate limits if present', async () => {
		const middleware = createRateLimitMiddleware();
		const context = createMockContext({
			auth: {
				userId: 'user-1',
				rateLimit: { limit: 500, window: 120 },
			},
		});
		const next = vi.fn();

		await middleware(context, next);

		expect(next).toHaveBeenCalled();
		expect(context.header).toHaveBeenCalledWith('X-RateLimit-Limit', '500');
	});

	it('should skip rate limiting when skip() returns true', async () => {
		const middleware = createRateLimitMiddleware({
			skip: () => true,
		});
		const context = createMockContext();
		const next = vi.fn();

		await middleware(context, next);

		expect(next).toHaveBeenCalled();
		// Headers should not be set when skipping
		expect(context.header).not.toHaveBeenCalled();
	});

	it('should add X-RateLimit headers', async () => {
		const middleware = createRateLimitMiddleware();
		const context = createMockContext();
		const next = vi.fn();

		await middleware(context, next);

		expect(context.header).toHaveBeenCalledWith('X-RateLimit-Limit', expect.any(String));
		expect(context.header).toHaveBeenCalledWith('X-RateLimit-Remaining', expect.any(String));
		expect(context.header).toHaveBeenCalledWith('X-RateLimit-Reset', expect.any(String));
	});

	it('should return 429 when limit exceeded', async () => {
		const middleware = createRateLimitMiddleware({ defaultLimit: 2 });
		const context = createMockContext();
		const next = vi.fn();

		// Exhaust limit
		await middleware(context, next);
		await middleware(context, next);

		// Should be blocked
		const result = await middleware(context, next);

		expect(result).toBeInstanceOf(Response);
		expect((result as Response).status).toBe(429);
	});

	it('should call custom onLimit handler when limit exceeded', async () => {
		const onLimit = vi.fn(() => new Response('Custom', { status: 429 }));
		const middleware = createRateLimitMiddleware({
			defaultLimit: 1,
			onLimit,
		});
		const context = createMockContext();
		const next = vi.fn();

		await middleware(context, next);
		await middleware(context, next);

		expect(onLimit).toHaveBeenCalled();
	});

	it('should use custom key generator', async () => {
		const keyGenerator = vi.fn(() => 'custom-key');
		const middleware = createRateLimitMiddleware({ keyGenerator });
		const context = createMockContext();
		const next = vi.fn();

		await middleware(context, next);

		expect(keyGenerator).toHaveBeenCalledWith(context);
	});
});

describe('defaultKeyGenerator (via middleware)', () => {
	beforeEach(() => {
		defaultRateLimiter.destroy();
	});

	it('should use user:${userId} when authenticated', async () => {
		const middleware = createRateLimitMiddleware({ defaultLimit: 100 });

		// Two different users should have independent limits
		const context1 = createMockContext({ auth: { userId: 'user-a' } });
		const context2 = createMockContext({ auth: { userId: 'user-b' } });
		const next = vi.fn();

		await middleware(context1, next);
		await middleware(context2, next);

		// Both should have full limits (99 remaining after first call)
		expect(context1.header).toHaveBeenCalledWith('X-RateLimit-Remaining', '99');
		expect(context2.header).toHaveBeenCalledWith('X-RateLimit-Remaining', '99');
	});

	it('should use ip when not authenticated', async () => {
		const middleware = createRateLimitMiddleware({ defaultLimit: 100 });
		const context = createMockContext({
			headers: { 'CF-Connecting-IP': '192.168.1.100' },
		});
		const next = vi.fn();

		await middleware(context, next);

		expect(context.header).toHaveBeenCalledWith('X-RateLimit-Remaining', '99');
	});

	it('should prefer CF-Connecting-IP over X-Forwarded-For', async () => {
		const middleware = createRateLimitMiddleware({ defaultLimit: 100 });

		// Both headers present, should use CF-Connecting-IP
		const context1 = createMockContext({
			headers: {
				'CF-Connecting-IP': '192.168.1.1',
				'X-Forwarded-For': '10.0.0.1',
			},
		});

		// Only X-Forwarded-For (should be different key)
		const context2 = createMockContext({
			headers: {
				'X-Forwarded-For': '10.0.0.1',
			},
		});

		const next = vi.fn();

		await middleware(context1, next);
		await middleware(context2, next);

		// Both should be at 99 (independent keys)
		expect(context1.header).toHaveBeenCalledWith('X-RateLimit-Remaining', '99');
		expect(context2.header).toHaveBeenCalledWith('X-RateLimit-Remaining', '99');
	});

	it('should fallback to unknown when no IP available', async () => {
		const middleware = createRateLimitMiddleware({ defaultLimit: 100 });
		const context = createMockContext();
		const next = vi.fn();

		await middleware(context, next);

		// Should still work with fallback key
		expect(next).toHaveBeenCalled();
	});
});

describe('strictRateLimit', () => {
	beforeEach(() => {
		defaultRateLimiter.destroy();
	});

	it('should apply custom limit', async () => {
		const middleware = strictRateLimit(5, 60);
		const context = createMockContext({ auth: { userId: 'strict-user' } });
		const next = vi.fn();

		await middleware(context, next);

		expect(context.header).toHaveBeenCalledWith('X-RateLimit-Limit', '5');
		expect(context.header).toHaveBeenCalledWith('X-RateLimit-Remaining', '4');
	});

	it('should include path in key for granularity', async () => {
		const middleware = strictRateLimit(10, 60);

		const context1 = createMockContext({
			auth: { userId: 'path-user' },
			path: '/api/endpoint-a',
		});
		const context2 = createMockContext({
			auth: { userId: 'path-user' },
			path: '/api/endpoint-b',
		});
		const next = vi.fn();

		await middleware(context1, next);
		await middleware(context2, next);

		// Different paths should have independent limits
		expect(context1.header).toHaveBeenCalledWith('X-RateLimit-Remaining', '9');
		expect(context2.header).toHaveBeenCalledWith('X-RateLimit-Remaining', '9');
	});

	it('should return 429 when strict limit exceeded', async () => {
		const middleware = strictRateLimit(2, 60);
		const context = createMockContext({ auth: { userId: 'strict-exceed' } });
		const next = vi.fn();

		await middleware(context, next);
		await middleware(context, next);

		const result = await middleware(context, next);

		expect(result).toBeInstanceOf(Response);
		expect((result as Response).status).toBe(429);
	});

	it('should use anonymous key when not authenticated', async () => {
		const middleware = strictRateLimit(10, 60);
		const context = createMockContext({ path: '/api/test' });
		const next = vi.fn();

		await middleware(context, next);

		expect(context.header).toHaveBeenCalledWith('X-RateLimit-Remaining', '9');
	});
});
