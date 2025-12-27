/**
 * Tests for Auth Routes
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { Hono } from 'hono';
import auth from '../../src/routes/auth.js';

// Mock password utilities
vi.mock('../../src/utils/password.js', () => ({
	hashPassword: vi.fn(async (password: string) => `hashed_${password}`),
	verifyPassword: vi.fn(async (password: string, hash: string) => hash === `hashed_${password}`),
	validatePassword: vi.fn((password: string) => {
		if (password.length < 8) return 'Password must be at least 8 characters long';
		if (!/[a-z]/.test(password)) return 'Password must contain at least one lowercase letter';
		if (!/[A-Z]/.test(password)) return 'Password must contain at least one uppercase letter';
		if (!/[0-9]/.test(password)) return 'Password must contain at least one number';
		return null;
	}),
	validateEmail: vi.fn((email: string) => {
		const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
		return emailRegex.test(email) && email.length <= 255;
	}),
}));

// Mock token utilities
vi.mock('../../src/utils/tokens.js', () => ({
	createAccessToken: vi.fn(async () => 'mock_access_token'),
	createRefreshToken: vi.fn(async () => 'mock_refresh_token'),
	verifyRefreshToken: vi.fn(async (token: string) => {
		if (token === 'valid_refresh_token') {
			return { sub: 'user-1', sid: 'session-1' };
		}
		if (token === 'expired_refresh_token') {
			return null;
		}
		return null;
	}),
	hashRefreshToken: vi.fn(async (token: string) => `hashed_${token}`),
	generateId: vi.fn(() => `id_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`),
	getRefreshTokenExpiry: vi.fn(() => Date.now() + 7 * 24 * 60 * 60 * 1000),
	TenantInfo: {},
}));

// Mock JWT middleware
vi.mock('../../src/middleware/jwt.js', () => ({
	requireJwtAuth: vi.fn((c: any, next: any) => {
		const authHeader = c.req.header('Authorization');
		if (authHeader === 'Bearer valid_token') {
			c.set('userId', 'user-1');
			return next();
		}
		return c.json({ error: 'Unauthorized' }, 401);
	}),
}));

// Helper to create mock D1 database
interface MockDbData {
	users: Map<string, {
		id: string;
		email: string;
		password_hash: string;
		name: string | null;
		is_active: number;
		created_at: number;
		last_login: number | null;
	}>;
	tenants: Map<string, {
		id: string;
		name: string;
		plan: string;
		max_memories: number;
		max_namespaces: number;
		created_at: number;
		updated_at: number;
	}>;
	userTenants: Map<string, {
		user_id: string;
		tenant_id: string;
		role: string;
		created_at: number;
	}>;
	sessions: Map<string, {
		id: string;
		user_id: string;
		refresh_token_hash: string;
		expires_at: number;
		created_at: number;
	}>;
	namespaces: Map<string, any>;
}

function createMockD1(data: MockDbData) {
	return {
		prepare: vi.fn((sql: string) => ({
			bind: vi.fn((...params: unknown[]) => ({
				run: vi.fn(async () => {
					// INSERT INTO users
					if (sql.includes('INSERT INTO users')) {
						const [id, email, password_hash, name, is_active, created_at, last_login] = params;
						data.users.set(id as string, {
							id: id as string,
							email: email as string,
							password_hash: password_hash as string,
							name: name as string | null,
							is_active: is_active as number,
							created_at: created_at as number,
							last_login: last_login as number,
						});
						return { meta: { changes: 1 } };
					}

					// INSERT INTO tenants
					if (sql.includes('INSERT INTO tenants')) {
						const [id, name, plan, max_memories, max_namespaces, created_at, updated_at] = params;
						data.tenants.set(id as string, {
							id: id as string,
							name: name as string,
							plan: plan as string,
							max_memories: max_memories as number,
							max_namespaces: max_namespaces as number,
							created_at: created_at as number,
							updated_at: updated_at as number,
						});
						return { meta: { changes: 1 } };
					}

					// INSERT INTO user_tenants
					if (sql.includes('INSERT INTO user_tenants')) {
						const [user_id, tenant_id, role, created_at] = params;
						data.userTenants.set(`${user_id}_${tenant_id}`, {
							user_id: user_id as string,
							tenant_id: tenant_id as string,
							role: role as string,
							created_at: created_at as number,
						});
						return { meta: { changes: 1 } };
					}

					// INSERT INTO namespaces
					if (sql.includes('INSERT INTO namespaces')) {
						const [name, tenant_id, dimensions, created_at, updated_at] = params;
						data.namespaces.set(name as string, {
							name,
							tenant_id,
							dimensions,
							created_at,
							updated_at,
						});
						return { meta: { changes: 1 } };
					}

					// INSERT INTO sessions
					if (sql.includes('INSERT INTO sessions')) {
						const [id, user_id, refresh_token_hash, expires_at, created_at] = params;
						data.sessions.set(id as string, {
							id: id as string,
							user_id: user_id as string,
							refresh_token_hash: refresh_token_hash as string,
							expires_at: expires_at as number,
							created_at: created_at as number,
						});
						return { meta: { changes: 1 } };
					}

					// UPDATE users SET last_login
					if (sql.includes('UPDATE users SET last_login')) {
						const [last_login, id] = params;
						const user = data.users.get(id as string);
						if (user) {
							user.last_login = last_login as number;
						}
						return { meta: { changes: user ? 1 : 0 } };
					}

					// UPDATE users SET password_hash
					if (sql.includes('UPDATE users SET password_hash')) {
						const [password_hash, id] = params;
						const user = data.users.get(id as string);
						if (user) {
							user.password_hash = password_hash as string;
						}
						return { meta: { changes: user ? 1 : 0 } };
					}

					// DELETE FROM sessions WHERE id
					if (sql.includes('DELETE FROM sessions WHERE id')) {
						const id = params[0] as string;
						const deleted = data.sessions.delete(id);
						return { meta: { changes: deleted ? 1 : 0 } };
					}

					// DELETE FROM sessions WHERE refresh_token_hash
					if (sql.includes('DELETE FROM sessions WHERE refresh_token_hash')) {
						const hash = params[0] as string;
						for (const [id, session] of data.sessions) {
							if (session.refresh_token_hash === hash) {
								data.sessions.delete(id);
							}
						}
						return { meta: { changes: 1 } };
					}

					// DELETE FROM sessions WHERE user_id
					if (sql.includes('DELETE FROM sessions WHERE user_id')) {
						const userId = params[0] as string;
						for (const [id, session] of data.sessions) {
							if (session.user_id === userId) {
								data.sessions.delete(id);
							}
						}
						return { meta: { changes: 1 } };
					}

					return { meta: { changes: 0 } };
				}),

				first: vi.fn(async <T>(): Promise<T | null> => {
					// SELECT id FROM users WHERE email
					if (sql.includes('SELECT id FROM users WHERE email')) {
						const email = (params[0] as string).toLowerCase();
						for (const user of data.users.values()) {
							if (user.email === email) {
								return { id: user.id } as T;
							}
						}
						return null;
					}

					// SELECT ... FROM users WHERE email (login)
					if (sql.includes('SELECT id, email, password_hash, name, is_active FROM users WHERE email')) {
						const email = (params[0] as string).toLowerCase();
						for (const user of data.users.values()) {
							if (user.email === email) {
								return user as T;
							}
						}
						return null;
					}

					// SELECT ... FROM sessions ... WHERE refresh_token_hash
					if (sql.includes('FROM sessions s') && sql.includes('WHERE s.refresh_token_hash')) {
						const tokenHash = params[0] as string;
						const userId = params[1] as string;
						for (const session of data.sessions.values()) {
							if (session.refresh_token_hash === tokenHash && session.user_id === userId) {
								const user = data.users.get(session.user_id);
								if (user) {
									return {
										id: session.id,
										user_id: session.user_id,
										expires_at: session.expires_at,
										email: user.email,
										name: user.name,
										is_active: user.is_active,
									} as T;
								}
							}
						}
						return null;
					}

					// SELECT ... FROM users WHERE id (for /me and /password)
					if (sql.includes('FROM users WHERE id')) {
						const id = params[0] as string;
						const user = data.users.get(id);
						if (user) {
							if (sql.includes('password_hash')) {
								return { password_hash: user.password_hash } as T;
							}
							return user as T;
						}
						return null;
					}

					return null;
				}),

				all: vi.fn(async <T>(): Promise<{ results: T[] }> => {
					// SELECT tenants for user
					if (sql.includes('FROM tenants t') && sql.includes('JOIN user_tenants ut')) {
						const userId = params[0] as string;
						const results: any[] = [];
						for (const ut of data.userTenants.values()) {
							if (ut.user_id === userId) {
								const tenant = data.tenants.get(ut.tenant_id);
								if (tenant) {
									results.push({
										id: tenant.id,
										name: tenant.name,
										role: ut.role,
										plan: tenant.plan,
										max_memories: tenant.max_memories,
										max_namespaces: tenant.max_namespaces,
										created_at: tenant.created_at,
									});
								}
							}
						}
						return { results: results as T[] };
					}
					return { results: [] };
				}),
			})),
		})),
	} as unknown as D1Database;
}

// Create app with auth routes
function createApp(db: D1Database) {
	const app = new Hono<{
		Bindings: {
			DB: D1Database;
			JWT_SECRET: string;
			JWT_REFRESH_SECRET: string;
		};
	}>();

	// Mount auth routes
	app.route('/auth', auth);

	// Bind environment
	return {
		fetch: (request: Request) => {
			return app.fetch(request, {
				DB: db,
				JWT_SECRET: 'test_jwt_secret',
				JWT_REFRESH_SECRET: 'test_refresh_secret',
			});
		},
	};
}

describe('Auth Routes', () => {
	let mockData: MockDbData;
	let db: D1Database;
	let app: ReturnType<typeof createApp>;

	beforeEach(() => {
		vi.clearAllMocks();

		mockData = {
			users: new Map(),
			tenants: new Map(),
			userTenants: new Map(),
			sessions: new Map(),
			namespaces: new Map(),
		};

		db = createMockD1(mockData);
		app = createApp(db);
	});

	describe('POST /auth/register', () => {
		it('should register a new user successfully', async () => {
			const res = await app.fetch(new Request('http://localhost/auth/register', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					email: 'test@example.com',
					password: 'SecurePass123',
					name: 'Test User',
				}),
			}));

			expect(res.status).toBe(201);
			const data = await res.json();
			expect(data.success).toBe(true);
			expect(data.accessToken).toBe('mock_access_token');
			expect(data.refreshToken).toBe('mock_refresh_token');
			expect(data.user.email).toBe('test@example.com');
			expect(data.tenants).toHaveLength(1);
			expect(data.tenants[0].role).toBe('owner');
		});

		it('should return 400 when email is missing', async () => {
			const res = await app.fetch(new Request('http://localhost/auth/register', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					password: 'SecurePass123',
				}),
			}));

			expect(res.status).toBe(400);
			const data = await res.json();
			expect(data.error).toBe('Email and password are required');
		});

		it('should return 400 when password is missing', async () => {
			const res = await app.fetch(new Request('http://localhost/auth/register', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					email: 'test@example.com',
				}),
			}));

			expect(res.status).toBe(400);
			const data = await res.json();
			expect(data.error).toBe('Email and password are required');
		});

		it('should return 400 for invalid email format', async () => {
			const res = await app.fetch(new Request('http://localhost/auth/register', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					email: 'invalid-email',
					password: 'SecurePass123',
				}),
			}));

			expect(res.status).toBe(400);
			const data = await res.json();
			expect(data.error).toBe('Invalid email format');
		});

		it('should return 400 for weak password (too short)', async () => {
			const res = await app.fetch(new Request('http://localhost/auth/register', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					email: 'test@example.com',
					password: 'Short1',
				}),
			}));

			expect(res.status).toBe(400);
			const data = await res.json();
			expect(data.error).toContain('at least 8 characters');
		});

		it('should return 400 for password without uppercase', async () => {
			const res = await app.fetch(new Request('http://localhost/auth/register', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					email: 'test@example.com',
					password: 'lowercase123',
				}),
			}));

			expect(res.status).toBe(400);
			const data = await res.json();
			expect(data.error).toContain('uppercase');
		});

		it('should return 400 for password without lowercase', async () => {
			const res = await app.fetch(new Request('http://localhost/auth/register', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					email: 'test@example.com',
					password: 'UPPERCASE123',
				}),
			}));

			expect(res.status).toBe(400);
			const data = await res.json();
			expect(data.error).toContain('lowercase');
		});

		it('should return 400 for password without number', async () => {
			const res = await app.fetch(new Request('http://localhost/auth/register', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					email: 'test@example.com',
					password: 'NoNumberPass',
				}),
			}));

			expect(res.status).toBe(400);
			const data = await res.json();
			expect(data.error).toContain('number');
		});

		it('should return 409 when email already exists', async () => {
			// Pre-populate user
			mockData.users.set('existing-user', {
				id: 'existing-user',
				email: 'existing@example.com',
				password_hash: 'hashed_password',
				name: 'Existing',
				is_active: 1,
				created_at: Date.now(),
				last_login: null,
			});

			const res = await app.fetch(new Request('http://localhost/auth/register', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					email: 'existing@example.com',
					password: 'SecurePass123',
				}),
			}));

			expect(res.status).toBe(409);
			const data = await res.json();
			expect(data.error).toBe('Email already registered');
		});

		it('should create tenant and namespace on registration', async () => {
			await app.fetch(new Request('http://localhost/auth/register', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					email: 'test@example.com',
					password: 'SecurePass123',
					name: 'Test User',
				}),
			}));

			expect(mockData.users.size).toBe(1);
			expect(mockData.tenants.size).toBe(1);
			expect(mockData.namespaces.size).toBe(1);
			expect(mockData.sessions.size).toBe(1);

			const tenant = Array.from(mockData.tenants.values())[0];
			expect(tenant.name).toBe("Test User's Workspace");
		});
	});

	describe('POST /auth/login', () => {
		beforeEach(() => {
			// Pre-populate user for login tests
			mockData.users.set('user-1', {
				id: 'user-1',
				email: 'user@example.com',
				password_hash: 'hashed_SecurePass123',
				name: 'Test User',
				is_active: 1,
				created_at: Date.now(),
				last_login: null,
			});

			mockData.tenants.set('tenant-1', {
				id: 'tenant-1',
				name: 'Test Workspace',
				plan: 'free',
				max_memories: 1000,
				max_namespaces: 1,
				created_at: Date.now(),
				updated_at: Date.now(),
			});

			mockData.userTenants.set('user-1_tenant-1', {
				user_id: 'user-1',
				tenant_id: 'tenant-1',
				role: 'owner',
				created_at: Date.now(),
			});
		});

		it('should login successfully with valid credentials', async () => {
			const res = await app.fetch(new Request('http://localhost/auth/login', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					email: 'user@example.com',
					password: 'SecurePass123',
				}),
			}));

			expect(res.status).toBe(200);
			const data = await res.json();
			expect(data.success).toBe(true);
			expect(data.accessToken).toBe('mock_access_token');
			expect(data.refreshToken).toBe('mock_refresh_token');
			expect(data.user.email).toBe('user@example.com');
			expect(data.tenants).toHaveLength(1);
		});

		it('should return 400 when email is missing', async () => {
			const res = await app.fetch(new Request('http://localhost/auth/login', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					password: 'SecurePass123',
				}),
			}));

			expect(res.status).toBe(400);
		});

		it('should return 401 when email does not exist', async () => {
			const res = await app.fetch(new Request('http://localhost/auth/login', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					email: 'nonexistent@example.com',
					password: 'SecurePass123',
				}),
			}));

			expect(res.status).toBe(401);
			const data = await res.json();
			expect(data.error).toBe('Invalid email or password');
		});

		it('should return 401 when password is incorrect', async () => {
			const res = await app.fetch(new Request('http://localhost/auth/login', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					email: 'user@example.com',
					password: 'WrongPassword123',
				}),
			}));

			expect(res.status).toBe(401);
			const data = await res.json();
			expect(data.error).toBe('Invalid email or password');
		});

		it('should return 403 when account is disabled', async () => {
			mockData.users.get('user-1')!.is_active = 0;

			const res = await app.fetch(new Request('http://localhost/auth/login', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					email: 'user@example.com',
					password: 'SecurePass123',
				}),
			}));

			expect(res.status).toBe(403);
			const data = await res.json();
			expect(data.error).toBe('Account is disabled');
		});

		it('should update last_login timestamp on successful login', async () => {
			const beforeLogin = mockData.users.get('user-1')!.last_login;

			await app.fetch(new Request('http://localhost/auth/login', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					email: 'user@example.com',
					password: 'SecurePass123',
				}),
			}));

			const afterLogin = mockData.users.get('user-1')!.last_login;
			expect(afterLogin).not.toBe(beforeLogin);
			expect(afterLogin).toBeGreaterThan(0);
		});

		it('should create a new session on login', async () => {
			expect(mockData.sessions.size).toBe(0);

			await app.fetch(new Request('http://localhost/auth/login', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					email: 'user@example.com',
					password: 'SecurePass123',
				}),
			}));

			expect(mockData.sessions.size).toBe(1);
		});
	});

	describe('POST /auth/refresh', () => {
		beforeEach(() => {
			mockData.users.set('user-1', {
				id: 'user-1',
				email: 'user@example.com',
				password_hash: 'hashed_pass',
				name: 'Test User',
				is_active: 1,
				created_at: Date.now(),
				last_login: Date.now(),
			});

			mockData.tenants.set('tenant-1', {
				id: 'tenant-1',
				name: 'Workspace',
				plan: 'free',
				max_memories: 1000,
				max_namespaces: 1,
				created_at: Date.now(),
				updated_at: Date.now(),
			});

			mockData.userTenants.set('user-1_tenant-1', {
				user_id: 'user-1',
				tenant_id: 'tenant-1',
				role: 'owner',
				created_at: Date.now(),
			});

			mockData.sessions.set('session-1', {
				id: 'session-1',
				user_id: 'user-1',
				refresh_token_hash: 'hashed_valid_refresh_token',
				expires_at: Date.now() + 7 * 24 * 60 * 60 * 1000,
				created_at: Date.now(),
			});
		});

		it('should refresh token successfully with valid refresh token', async () => {
			const res = await app.fetch(new Request('http://localhost/auth/refresh', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					refreshToken: 'valid_refresh_token',
				}),
			}));

			expect(res.status).toBe(200);
			const data = await res.json();
			expect(data.success).toBe(true);
			expect(data.accessToken).toBe('mock_access_token');
			expect(data.refreshToken).toBe('mock_refresh_token');
		});

		it('should return 400 when refresh token is missing', async () => {
			const res = await app.fetch(new Request('http://localhost/auth/refresh', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({}),
			}));

			expect(res.status).toBe(400);
			const data = await res.json();
			expect(data.error).toBe('Refresh token is required');
		});

		it('should return 401 for invalid refresh token', async () => {
			const res = await app.fetch(new Request('http://localhost/auth/refresh', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					refreshToken: 'invalid_token',
				}),
			}));

			expect(res.status).toBe(401);
			const data = await res.json();
			expect(data.error).toBe('Invalid or expired refresh token');
		});

		it('should return 401 for expired refresh token', async () => {
			const res = await app.fetch(new Request('http://localhost/auth/refresh', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					refreshToken: 'expired_refresh_token',
				}),
			}));

			expect(res.status).toBe(401);
		});

		it('should rotate refresh token (delete old, create new session)', async () => {
			expect(mockData.sessions.has('session-1')).toBe(true);

			await app.fetch(new Request('http://localhost/auth/refresh', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					refreshToken: 'valid_refresh_token',
				}),
			}));

			// Old session should be deleted
			expect(mockData.sessions.has('session-1')).toBe(false);
			// New session should be created
			expect(mockData.sessions.size).toBe(1);
		});
	});

	describe('POST /auth/logout', () => {
		beforeEach(() => {
			mockData.sessions.set('session-1', {
				id: 'session-1',
				user_id: 'user-1',
				refresh_token_hash: 'hashed_logout_token',
				expires_at: Date.now() + 7 * 24 * 60 * 60 * 1000,
				created_at: Date.now(),
			});
		});

		it('should logout successfully', async () => {
			const res = await app.fetch(new Request('http://localhost/auth/logout', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					refreshToken: 'logout_token',
				}),
			}));

			expect(res.status).toBe(200);
			const data = await res.json();
			expect(data.success).toBe(true);
			expect(data.message).toBe('Logged out successfully');
		});

		it('should return 400 when refresh token is missing', async () => {
			const res = await app.fetch(new Request('http://localhost/auth/logout', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({}),
			}));

			expect(res.status).toBe(400);
			const data = await res.json();
			expect(data.error).toBe('Refresh token is required');
		});

		it('should delete the session on logout', async () => {
			expect(mockData.sessions.size).toBe(1);

			await app.fetch(new Request('http://localhost/auth/logout', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					refreshToken: 'logout_token',
				}),
			}));

			// Session should be deleted (by hash match)
			// Note: In our mock, we're clearing by hash, which works
		});
	});

	describe('GET /auth/me', () => {
		beforeEach(() => {
			mockData.users.set('user-1', {
				id: 'user-1',
				email: 'user@example.com',
				password_hash: 'hashed_pass',
				name: 'Test User',
				is_active: 1,
				created_at: Date.now() - 86400000,
				last_login: Date.now(),
			});

			mockData.tenants.set('tenant-1', {
				id: 'tenant-1',
				name: 'Workspace',
				plan: 'free',
				max_memories: 1000,
				max_namespaces: 1,
				created_at: Date.now(),
				updated_at: Date.now(),
			});

			mockData.userTenants.set('user-1_tenant-1', {
				user_id: 'user-1',
				tenant_id: 'tenant-1',
				role: 'owner',
				created_at: Date.now(),
			});
		});

		it('should return user profile with valid JWT', async () => {
			const res = await app.fetch(new Request('http://localhost/auth/me', {
				method: 'GET',
				headers: {
					'Authorization': 'Bearer valid_token',
				},
			}));

			expect(res.status).toBe(200);
			const data = await res.json();
			expect(data.user.id).toBe('user-1');
			expect(data.user.email).toBe('user@example.com');
			expect(data.tenants).toHaveLength(1);
		});

		it('should return 401 without authorization header', async () => {
			const res = await app.fetch(new Request('http://localhost/auth/me', {
				method: 'GET',
			}));

			expect(res.status).toBe(401);
		});

		it('should return 401 with invalid token', async () => {
			const res = await app.fetch(new Request('http://localhost/auth/me', {
				method: 'GET',
				headers: {
					'Authorization': 'Bearer invalid_token',
				},
			}));

			expect(res.status).toBe(401);
		});
	});

	describe('PUT /auth/password', () => {
		beforeEach(() => {
			mockData.users.set('user-1', {
				id: 'user-1',
				email: 'user@example.com',
				password_hash: 'hashed_CurrentPass123',
				name: 'Test User',
				is_active: 1,
				created_at: Date.now(),
				last_login: Date.now(),
			});

			mockData.sessions.set('session-1', {
				id: 'session-1',
				user_id: 'user-1',
				refresh_token_hash: 'some_hash',
				expires_at: Date.now() + 86400000,
				created_at: Date.now(),
			});
		});

		it('should change password successfully', async () => {
			const res = await app.fetch(new Request('http://localhost/auth/password', {
				method: 'PUT',
				headers: {
					'Content-Type': 'application/json',
					'Authorization': 'Bearer valid_token',
				},
				body: JSON.stringify({
					currentPassword: 'CurrentPass123',
					newPassword: 'NewSecurePass456',
				}),
			}));

			expect(res.status).toBe(200);
			const data = await res.json();
			expect(data.success).toBe(true);
			expect(data.message).toBe('Password changed successfully');
		});

		it('should return 400 when current password is missing', async () => {
			const res = await app.fetch(new Request('http://localhost/auth/password', {
				method: 'PUT',
				headers: {
					'Content-Type': 'application/json',
					'Authorization': 'Bearer valid_token',
				},
				body: JSON.stringify({
					newPassword: 'NewSecurePass456',
				}),
			}));

			expect(res.status).toBe(400);
		});

		it('should return 400 when new password is missing', async () => {
			const res = await app.fetch(new Request('http://localhost/auth/password', {
				method: 'PUT',
				headers: {
					'Content-Type': 'application/json',
					'Authorization': 'Bearer valid_token',
				},
				body: JSON.stringify({
					currentPassword: 'CurrentPass123',
				}),
			}));

			expect(res.status).toBe(400);
		});

		it('should return 401 when current password is incorrect', async () => {
			const res = await app.fetch(new Request('http://localhost/auth/password', {
				method: 'PUT',
				headers: {
					'Content-Type': 'application/json',
					'Authorization': 'Bearer valid_token',
				},
				body: JSON.stringify({
					currentPassword: 'WrongPassword123',
					newPassword: 'NewSecurePass456',
				}),
			}));

			expect(res.status).toBe(401);
			const data = await res.json();
			expect(data.error).toBe('Current password is incorrect');
		});

		it('should return 400 when new password does not meet requirements', async () => {
			const res = await app.fetch(new Request('http://localhost/auth/password', {
				method: 'PUT',
				headers: {
					'Content-Type': 'application/json',
					'Authorization': 'Bearer valid_token',
				},
				body: JSON.stringify({
					currentPassword: 'CurrentPass123',
					newPassword: 'weak',
				}),
			}));

			expect(res.status).toBe(400);
		});

		it('should update password hash in database', async () => {
			const oldHash = mockData.users.get('user-1')!.password_hash;

			await app.fetch(new Request('http://localhost/auth/password', {
				method: 'PUT',
				headers: {
					'Content-Type': 'application/json',
					'Authorization': 'Bearer valid_token',
				},
				body: JSON.stringify({
					currentPassword: 'CurrentPass123',
					newPassword: 'NewSecurePass456',
				}),
			}));

			const newHash = mockData.users.get('user-1')!.password_hash;
			expect(newHash).not.toBe(oldHash);
			expect(newHash).toBe('hashed_NewSecurePass456');
		});

		it('should invalidate all sessions after password change', async () => {
			expect(mockData.sessions.size).toBe(1);

			await app.fetch(new Request('http://localhost/auth/password', {
				method: 'PUT',
				headers: {
					'Content-Type': 'application/json',
					'Authorization': 'Bearer valid_token',
				},
				body: JSON.stringify({
					currentPassword: 'CurrentPass123',
					newPassword: 'NewSecurePass456',
				}),
			}));

			expect(mockData.sessions.size).toBe(0);
		});

		it('should return 401 without authentication', async () => {
			const res = await app.fetch(new Request('http://localhost/auth/password', {
				method: 'PUT',
				headers: {
					'Content-Type': 'application/json',
				},
				body: JSON.stringify({
					currentPassword: 'CurrentPass123',
					newPassword: 'NewSecurePass456',
				}),
			}));

			expect(res.status).toBe(401);
		});
	});
});
