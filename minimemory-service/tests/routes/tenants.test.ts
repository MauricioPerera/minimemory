/**
 * Tests for Tenant Routes
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { Hono } from 'hono';
import tenants from '../../src/routes/tenants.js';

// Mock JWT middleware
vi.mock('../../src/middleware/jwt.js', () => ({
	requireJwtAuth: vi.fn((c: any, next: any) => {
		const authHeader = c.req.header('Authorization');
		if (authHeader === 'Bearer valid_token_user1') {
			c.set('userId', 'user-1');
			return next();
		}
		if (authHeader === 'Bearer valid_token_user2') {
			c.set('userId', 'user-2');
			return next();
		}
		if (authHeader === 'Bearer valid_token_user3') {
			c.set('userId', 'user-3');
			return next();
		}
		return c.json({ error: 'Unauthorized' }, 401);
	}),
}));

// Mock token utilities
vi.mock('../../src/utils/tokens.js', () => ({
	generateId: vi.fn(() => `id_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`),
}));

// Mock data types
interface MockDbData {
	users: Map<string, {
		id: string;
		email: string;
		name: string | null;
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
	namespaces: Map<string, {
		name: string;
		tenant_id: string;
		dimensions: number;
		created_at: number;
	}>;
	memories: Map<string, any>;
}

function createMockD1(data: MockDbData) {
	return {
		prepare: vi.fn((sql: string) => ({
			bind: vi.fn((...params: unknown[]) => ({
				run: vi.fn(async () => {
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
							name: name as string,
							tenant_id: tenant_id as string,
							dimensions: dimensions as number,
							created_at: created_at as number,
						});
						return { meta: { changes: 1 } };
					}

					// UPDATE tenants
					if (sql.includes('UPDATE tenants SET name')) {
						const [name, updated_at, id] = params;
						const tenant = data.tenants.get(id as string);
						if (tenant) {
							tenant.name = name as string;
							tenant.updated_at = updated_at as number;
						}
						return { meta: { changes: tenant ? 1 : 0 } };
					}

					// UPDATE user_tenants role
					if (sql.includes('UPDATE user_tenants SET role')) {
						const [role, user_id, tenant_id] = params;
						const key = `${user_id}_${tenant_id}`;
						const ut = data.userTenants.get(key);
						if (ut) {
							ut.role = role as string;
						}
						return { meta: { changes: ut ? 1 : 0 } };
					}

					// DELETE FROM tenants
					if (sql.includes('DELETE FROM tenants WHERE id')) {
						const id = params[0] as string;
						const deleted = data.tenants.delete(id);
						// Cascade delete user_tenants
						for (const [key, ut] of data.userTenants) {
							if (ut.tenant_id === id) {
								data.userTenants.delete(key);
							}
						}
						// Cascade delete namespaces
						for (const [name, ns] of data.namespaces) {
							if (ns.tenant_id === id) {
								data.namespaces.delete(name);
							}
						}
						return { meta: { changes: deleted ? 1 : 0 } };
					}

					// DELETE FROM user_tenants
					if (sql.includes('DELETE FROM user_tenants WHERE user_id')) {
						const [user_id, tenant_id] = params;
						const key = `${user_id}_${tenant_id}`;
						const deleted = data.userTenants.delete(key);
						return { meta: { changes: deleted ? 1 : 0 } };
					}

					return { meta: { changes: 0 } };
				}),

				first: vi.fn(async <T>(): Promise<T | null> => {
					// SELECT role FROM user_tenants
					if (sql.includes('SELECT role FROM user_tenants')) {
						const [user_id, tenant_id] = params;
						const key = `${user_id}_${tenant_id}`;
						const ut = data.userTenants.get(key);
						if (ut) {
							return { role: ut.role } as T;
						}
						return null;
					}

					// SELECT COUNT(*) FROM user_tenants WHERE user_id
					if (sql.includes('SELECT COUNT(*) as count FROM user_tenants WHERE user_id')) {
						const userId = params[0] as string;
						let count = 0;
						for (const ut of data.userTenants.values()) {
							if (ut.user_id === userId) count++;
						}
						return { count } as T;
					}

					// SELECT tenant with stats
					if (sql.includes('FROM tenants t') && sql.includes('WHERE t.id')) {
						const tenantId = params[0] as string;
						const tenant = data.tenants.get(tenantId);
						if (tenant) {
							let namespaceCount = 0;
							let memberCount = 0;
							let memoryCount = 0;

							for (const ns of data.namespaces.values()) {
								if (ns.tenant_id === tenantId) namespaceCount++;
							}
							for (const ut of data.userTenants.values()) {
								if (ut.tenant_id === tenantId) memberCount++;
							}

							return {
								...tenant,
								namespace_count: namespaceCount,
								member_count: memberCount,
								memory_count: memoryCount,
							} as T;
						}
						return null;
					}

					// SELECT user by email
					if (sql.includes('SELECT id, email, name FROM users WHERE email')) {
						const email = (params[0] as string).toLowerCase();
						for (const user of data.users.values()) {
							if (user.email === email) {
								return user as T;
							}
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
									let namespaceCount = 0;
									let memberCount = 0;
									for (const ns of data.namespaces.values()) {
										if (ns.tenant_id === tenant.id) namespaceCount++;
									}
									for (const utCheck of data.userTenants.values()) {
										if (utCheck.tenant_id === tenant.id) memberCount++;
									}

									results.push({
										id: tenant.id,
										name: tenant.name,
										plan: tenant.plan,
										max_memories: tenant.max_memories,
										max_namespaces: tenant.max_namespaces,
										created_at: tenant.created_at,
										updated_at: tenant.updated_at,
										role: ut.role,
										namespace_count: namespaceCount,
										member_count: memberCount,
									});
								}
							}
						}
						return { results: results as T[] };
					}

					// SELECT namespaces for tenant
					if (sql.includes('FROM namespaces n') && sql.includes('WHERE n.tenant_id')) {
						const tenantId = params[0] as string;
						const results: any[] = [];
						for (const ns of data.namespaces.values()) {
							if (ns.tenant_id === tenantId) {
								results.push({
									name: ns.name,
									dimensions: ns.dimensions,
									created_at: ns.created_at,
									memory_count: 0,
								});
							}
						}
						return { results: results as T[] };
					}

					// SELECT members for tenant
					if (sql.includes('FROM users u') && sql.includes('JOIN user_tenants ut')) {
						const tenantId = params[0] as string;
						const results: any[] = [];
						for (const ut of data.userTenants.values()) {
							if (ut.tenant_id === tenantId) {
								const user = data.users.get(ut.user_id);
								if (user) {
									results.push({
										id: user.id,
										email: user.email,
										name: user.name,
										role: ut.role,
										created_at: ut.created_at,
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

// Create app with tenant routes
function createApp(db: D1Database) {
	const app = new Hono<{
		Bindings: {
			DB: D1Database;
			JWT_SECRET: string;
		};
	}>();

	app.route('/tenants', tenants);

	return {
		fetch: (request: Request) => {
			return app.fetch(request, {
				DB: db,
				JWT_SECRET: 'test_jwt_secret',
			});
		},
	};
}

describe('Tenant Routes', () => {
	let mockData: MockDbData;
	let db: D1Database;
	let app: ReturnType<typeof createApp>;

	beforeEach(() => {
		vi.clearAllMocks();

		mockData = {
			users: new Map([
				['user-1', { id: 'user-1', email: 'user1@example.com', name: 'User One' }],
				['user-2', { id: 'user-2', email: 'user2@example.com', name: 'User Two' }],
				['user-3', { id: 'user-3', email: 'user3@example.com', name: 'User Three' }],
			]),
			tenants: new Map([
				['tenant-1', {
					id: 'tenant-1',
					name: 'Test Workspace',
					plan: 'free',
					max_memories: 1000,
					max_namespaces: 1,
					created_at: Date.now() - 86400000,
					updated_at: Date.now() - 86400000,
				}],
			]),
			userTenants: new Map([
				['user-1_tenant-1', {
					user_id: 'user-1',
					tenant_id: 'tenant-1',
					role: 'owner',
					created_at: Date.now() - 86400000,
				}],
			]),
			namespaces: new Map([
				['tenant-1-default', {
					name: 'tenant-1-default',
					tenant_id: 'tenant-1',
					dimensions: 1536,
					created_at: Date.now() - 86400000,
				}],
			]),
			memories: new Map(),
		};

		db = createMockD1(mockData);
		app = createApp(db);
	});

	describe('GET /tenants', () => {
		it('should list tenants for authenticated user', async () => {
			const res = await app.fetch(new Request('http://localhost/tenants', {
				method: 'GET',
				headers: { 'Authorization': 'Bearer valid_token_user1' },
			}));

			expect(res.status).toBe(200);
			const data = await res.json();
			expect(data.tenants).toHaveLength(1);
			expect(data.tenants[0].name).toBe('Test Workspace');
			expect(data.tenants[0].role).toBe('owner');
		});

		it('should include role and counts in response', async () => {
			const res = await app.fetch(new Request('http://localhost/tenants', {
				method: 'GET',
				headers: { 'Authorization': 'Bearer valid_token_user1' },
			}));

			const data = await res.json();
			expect(data.tenants[0]).toHaveProperty('role');
			expect(data.tenants[0]).toHaveProperty('namespaceCount');
			expect(data.tenants[0]).toHaveProperty('memberCount');
		});

		it('should return 401 without authentication', async () => {
			const res = await app.fetch(new Request('http://localhost/tenants', {
				method: 'GET',
			}));

			expect(res.status).toBe(401);
		});

		it('should return empty list for user with no tenants', async () => {
			const res = await app.fetch(new Request('http://localhost/tenants', {
				method: 'GET',
				headers: { 'Authorization': 'Bearer valid_token_user2' },
			}));

			expect(res.status).toBe(200);
			const data = await res.json();
			expect(data.tenants).toHaveLength(0);
		});
	});

	describe('POST /tenants', () => {
		it('should create a new tenant', async () => {
			const res = await app.fetch(new Request('http://localhost/tenants', {
				method: 'POST',
				headers: {
					'Content-Type': 'application/json',
					'Authorization': 'Bearer valid_token_user1',
				},
				body: JSON.stringify({ name: 'New Workspace' }),
			}));

			expect(res.status).toBe(201);
			const data = await res.json();
			expect(data.success).toBe(true);
			expect(data.tenant.name).toBe('New Workspace');
			expect(data.tenant.role).toBe('owner');
		});

		it('should create default namespace for new tenant', async () => {
			const initialNamespaces = mockData.namespaces.size;

			await app.fetch(new Request('http://localhost/tenants', {
				method: 'POST',
				headers: {
					'Content-Type': 'application/json',
					'Authorization': 'Bearer valid_token_user1',
				},
				body: JSON.stringify({ name: 'New Workspace' }),
			}));

			expect(mockData.namespaces.size).toBe(initialNamespaces + 1);
		});

		it('should return 400 when name is empty', async () => {
			const res = await app.fetch(new Request('http://localhost/tenants', {
				method: 'POST',
				headers: {
					'Content-Type': 'application/json',
					'Authorization': 'Bearer valid_token_user1',
				},
				body: JSON.stringify({ name: '' }),
			}));

			expect(res.status).toBe(400);
			const data = await res.json();
			expect(data.error).toBe('Tenant name is required');
		});

		it('should return 400 when name is too long', async () => {
			const res = await app.fetch(new Request('http://localhost/tenants', {
				method: 'POST',
				headers: {
					'Content-Type': 'application/json',
					'Authorization': 'Bearer valid_token_user1',
				},
				body: JSON.stringify({ name: 'a'.repeat(101) }),
			}));

			expect(res.status).toBe(400);
			const data = await res.json();
			expect(data.error).toContain('100 characters');
		});

		it('should trim whitespace from name', async () => {
			const res = await app.fetch(new Request('http://localhost/tenants', {
				method: 'POST',
				headers: {
					'Content-Type': 'application/json',
					'Authorization': 'Bearer valid_token_user1',
				},
				body: JSON.stringify({ name: '  Trimmed Name  ' }),
			}));

			const data = await res.json();
			expect(data.tenant.name).toBe('Trimmed Name');
		});
	});

	describe('GET /tenants/:id', () => {
		it('should return tenant details for member', async () => {
			const res = await app.fetch(new Request('http://localhost/tenants/tenant-1', {
				method: 'GET',
				headers: { 'Authorization': 'Bearer valid_token_user1' },
			}));

			expect(res.status).toBe(200);
			const data = await res.json();
			expect(data.tenant.id).toBe('tenant-1');
			expect(data.tenant.name).toBe('Test Workspace');
			expect(data.tenant.role).toBe('owner');
			expect(data.tenant.stats).toBeDefined();
		});

		it('should return 404 for non-member', async () => {
			const res = await app.fetch(new Request('http://localhost/tenants/tenant-1', {
				method: 'GET',
				headers: { 'Authorization': 'Bearer valid_token_user2' },
			}));

			expect(res.status).toBe(404);
		});

		it('should return 404 for non-existent tenant', async () => {
			const res = await app.fetch(new Request('http://localhost/tenants/nonexistent', {
				method: 'GET',
				headers: { 'Authorization': 'Bearer valid_token_user1' },
			}));

			expect(res.status).toBe(404);
		});

		it('should include namespaces in response', async () => {
			const res = await app.fetch(new Request('http://localhost/tenants/tenant-1', {
				method: 'GET',
				headers: { 'Authorization': 'Bearer valid_token_user1' },
			}));

			const data = await res.json();
			expect(data.namespaces).toBeDefined();
			expect(data.namespaces.length).toBeGreaterThanOrEqual(1);
		});
	});

	describe('PUT /tenants/:id', () => {
		it('should update tenant name for owner', async () => {
			const res = await app.fetch(new Request('http://localhost/tenants/tenant-1', {
				method: 'PUT',
				headers: {
					'Content-Type': 'application/json',
					'Authorization': 'Bearer valid_token_user1',
				},
				body: JSON.stringify({ name: 'Updated Workspace' }),
			}));

			expect(res.status).toBe(200);
			const data = await res.json();
			expect(data.success).toBe(true);
			expect(data.tenant.name).toBe('Updated Workspace');
		});

		it('should update tenant name for admin', async () => {
			// Add user-2 as admin
			mockData.userTenants.set('user-2_tenant-1', {
				user_id: 'user-2',
				tenant_id: 'tenant-1',
				role: 'admin',
				created_at: Date.now(),
			});

			const res = await app.fetch(new Request('http://localhost/tenants/tenant-1', {
				method: 'PUT',
				headers: {
					'Content-Type': 'application/json',
					'Authorization': 'Bearer valid_token_user2',
				},
				body: JSON.stringify({ name: 'Admin Updated' }),
			}));

			expect(res.status).toBe(200);
		});

		it('should return 403 for member role', async () => {
			// Add user-2 as member
			mockData.userTenants.set('user-2_tenant-1', {
				user_id: 'user-2',
				tenant_id: 'tenant-1',
				role: 'member',
				created_at: Date.now(),
			});

			const res = await app.fetch(new Request('http://localhost/tenants/tenant-1', {
				method: 'PUT',
				headers: {
					'Content-Type': 'application/json',
					'Authorization': 'Bearer valid_token_user2',
				},
				body: JSON.stringify({ name: 'Unauthorized Update' }),
			}));

			expect(res.status).toBe(403);
		});

		it('should return 403 for viewer role', async () => {
			mockData.userTenants.set('user-2_tenant-1', {
				user_id: 'user-2',
				tenant_id: 'tenant-1',
				role: 'viewer',
				created_at: Date.now(),
			});

			const res = await app.fetch(new Request('http://localhost/tenants/tenant-1', {
				method: 'PUT',
				headers: {
					'Content-Type': 'application/json',
					'Authorization': 'Bearer valid_token_user2',
				},
				body: JSON.stringify({ name: 'Unauthorized Update' }),
			}));

			expect(res.status).toBe(403);
		});

		it('should return 400 for empty name', async () => {
			const res = await app.fetch(new Request('http://localhost/tenants/tenant-1', {
				method: 'PUT',
				headers: {
					'Content-Type': 'application/json',
					'Authorization': 'Bearer valid_token_user1',
				},
				body: JSON.stringify({ name: '' }),
			}));

			expect(res.status).toBe(400);
		});
	});

	describe('DELETE /tenants/:id', () => {
		beforeEach(() => {
			// Add a second tenant so user-1 can delete tenant-1
			mockData.tenants.set('tenant-2', {
				id: 'tenant-2',
				name: 'Second Workspace',
				plan: 'free',
				max_memories: 1000,
				max_namespaces: 1,
				created_at: Date.now(),
				updated_at: Date.now(),
			});
			mockData.userTenants.set('user-1_tenant-2', {
				user_id: 'user-1',
				tenant_id: 'tenant-2',
				role: 'owner',
				created_at: Date.now(),
			});
		});

		it('should delete tenant for owner', async () => {
			const res = await app.fetch(new Request('http://localhost/tenants/tenant-1', {
				method: 'DELETE',
				headers: { 'Authorization': 'Bearer valid_token_user1' },
			}));

			expect(res.status).toBe(200);
			const data = await res.json();
			expect(data.success).toBe(true);
			expect(mockData.tenants.has('tenant-1')).toBe(false);
		});

		it('should return 403 for admin (owner required)', async () => {
			mockData.userTenants.set('user-2_tenant-1', {
				user_id: 'user-2',
				tenant_id: 'tenant-1',
				role: 'admin',
				created_at: Date.now(),
			});

			const res = await app.fetch(new Request('http://localhost/tenants/tenant-1', {
				method: 'DELETE',
				headers: { 'Authorization': 'Bearer valid_token_user2' },
			}));

			expect(res.status).toBe(403);
		});

		it('should return 400 when trying to delete last tenant', async () => {
			// Remove second tenant ownership
			mockData.userTenants.delete('user-1_tenant-2');

			const res = await app.fetch(new Request('http://localhost/tenants/tenant-1', {
				method: 'DELETE',
				headers: { 'Authorization': 'Bearer valid_token_user1' },
			}));

			expect(res.status).toBe(400);
			const data = await res.json();
			expect(data.error).toContain('last tenant');
		});

		it('should cascade delete namespaces', async () => {
			expect(mockData.namespaces.has('tenant-1-default')).toBe(true);

			await app.fetch(new Request('http://localhost/tenants/tenant-1', {
				method: 'DELETE',
				headers: { 'Authorization': 'Bearer valid_token_user1' },
			}));

			expect(mockData.namespaces.has('tenant-1-default')).toBe(false);
		});
	});

	describe('GET /tenants/:id/members', () => {
		it('should list members for tenant', async () => {
			const res = await app.fetch(new Request('http://localhost/tenants/tenant-1/members', {
				method: 'GET',
				headers: { 'Authorization': 'Bearer valid_token_user1' },
			}));

			expect(res.status).toBe(200);
			const data = await res.json();
			expect(data.members).toBeDefined();
			expect(data.members.length).toBeGreaterThanOrEqual(1);
		});

		it('should return 404 for non-member', async () => {
			const res = await app.fetch(new Request('http://localhost/tenants/tenant-1/members', {
				method: 'GET',
				headers: { 'Authorization': 'Bearer valid_token_user2' },
			}));

			expect(res.status).toBe(404);
		});
	});

	describe('POST /tenants/:id/members', () => {
		it('should invite member by email', async () => {
			const res = await app.fetch(new Request('http://localhost/tenants/tenant-1/members', {
				method: 'POST',
				headers: {
					'Content-Type': 'application/json',
					'Authorization': 'Bearer valid_token_user1',
				},
				body: JSON.stringify({
					email: 'user2@example.com',
					role: 'member',
				}),
			}));

			expect(res.status).toBe(201);
			const data = await res.json();
			expect(data.success).toBe(true);
			expect(data.member.email).toBe('user2@example.com');
			expect(data.member.role).toBe('member');
		});

		it('should return 409 when user is already a member', async () => {
			// Add user-2 as member first
			mockData.userTenants.set('user-2_tenant-1', {
				user_id: 'user-2',
				tenant_id: 'tenant-1',
				role: 'member',
				created_at: Date.now(),
			});

			const res = await app.fetch(new Request('http://localhost/tenants/tenant-1/members', {
				method: 'POST',
				headers: {
					'Content-Type': 'application/json',
					'Authorization': 'Bearer valid_token_user1',
				},
				body: JSON.stringify({
					email: 'user2@example.com',
					role: 'member',
				}),
			}));

			expect(res.status).toBe(409);
		});

		it('should return 404 when user does not exist', async () => {
			const res = await app.fetch(new Request('http://localhost/tenants/tenant-1/members', {
				method: 'POST',
				headers: {
					'Content-Type': 'application/json',
					'Authorization': 'Bearer valid_token_user1',
				},
				body: JSON.stringify({
					email: 'nonexistent@example.com',
					role: 'member',
				}),
			}));

			expect(res.status).toBe(404);
		});

		it('should return 403 for member role (admin required)', async () => {
			mockData.userTenants.set('user-2_tenant-1', {
				user_id: 'user-2',
				tenant_id: 'tenant-1',
				role: 'member',
				created_at: Date.now(),
			});

			const res = await app.fetch(new Request('http://localhost/tenants/tenant-1/members', {
				method: 'POST',
				headers: {
					'Content-Type': 'application/json',
					'Authorization': 'Bearer valid_token_user2',
				},
				body: JSON.stringify({
					email: 'user3@example.com',
					role: 'viewer',
				}),
			}));

			expect(res.status).toBe(403);
		});

		it('should return 400 for invalid role', async () => {
			const res = await app.fetch(new Request('http://localhost/tenants/tenant-1/members', {
				method: 'POST',
				headers: {
					'Content-Type': 'application/json',
					'Authorization': 'Bearer valid_token_user1',
				},
				body: JSON.stringify({
					email: 'user2@example.com',
					role: 'invalid_role',
				}),
			}));

			expect(res.status).toBe(400);
		});

		it('should allow admin to invite members', async () => {
			mockData.userTenants.set('user-2_tenant-1', {
				user_id: 'user-2',
				tenant_id: 'tenant-1',
				role: 'admin',
				created_at: Date.now(),
			});

			const res = await app.fetch(new Request('http://localhost/tenants/tenant-1/members', {
				method: 'POST',
				headers: {
					'Content-Type': 'application/json',
					'Authorization': 'Bearer valid_token_user2',
				},
				body: JSON.stringify({
					email: 'user3@example.com',
					role: 'member',
				}),
			}));

			expect(res.status).toBe(201);
		});
	});

	describe('PUT /tenants/:id/members/:memberId', () => {
		beforeEach(() => {
			// Add user-2 as member
			mockData.userTenants.set('user-2_tenant-1', {
				user_id: 'user-2',
				tenant_id: 'tenant-1',
				role: 'member',
				created_at: Date.now(),
			});
		});

		it('should update member role (owner only)', async () => {
			const res = await app.fetch(new Request('http://localhost/tenants/tenant-1/members/user-2', {
				method: 'PUT',
				headers: {
					'Content-Type': 'application/json',
					'Authorization': 'Bearer valid_token_user1',
				},
				body: JSON.stringify({ role: 'admin' }),
			}));

			expect(res.status).toBe(200);
			const data = await res.json();
			expect(data.success).toBe(true);
			expect(data.role).toBe('admin');
		});

		it('should return 403 when admin tries to change roles', async () => {
			mockData.userTenants.get('user-2_tenant-1')!.role = 'admin';

			mockData.userTenants.set('user-3_tenant-1', {
				user_id: 'user-3',
				tenant_id: 'tenant-1',
				role: 'member',
				created_at: Date.now(),
			});

			const res = await app.fetch(new Request('http://localhost/tenants/tenant-1/members/user-3', {
				method: 'PUT',
				headers: {
					'Content-Type': 'application/json',
					'Authorization': 'Bearer valid_token_user2',
				},
				body: JSON.stringify({ role: 'admin' }),
			}));

			expect(res.status).toBe(403);
		});

		it('should return 400 when trying to change owner role', async () => {
			const res = await app.fetch(new Request('http://localhost/tenants/tenant-1/members/user-1', {
				method: 'PUT',
				headers: {
					'Content-Type': 'application/json',
					'Authorization': 'Bearer valid_token_user1',
				},
				body: JSON.stringify({ role: 'admin' }),
			}));

			expect(res.status).toBe(400);
		});

		it('should return 404 when member not found', async () => {
			const res = await app.fetch(new Request('http://localhost/tenants/tenant-1/members/nonexistent', {
				method: 'PUT',
				headers: {
					'Content-Type': 'application/json',
					'Authorization': 'Bearer valid_token_user1',
				},
				body: JSON.stringify({ role: 'admin' }),
			}));

			expect(res.status).toBe(404);
		});

		it('should return 400 for invalid role', async () => {
			const res = await app.fetch(new Request('http://localhost/tenants/tenant-1/members/user-2', {
				method: 'PUT',
				headers: {
					'Content-Type': 'application/json',
					'Authorization': 'Bearer valid_token_user1',
				},
				body: JSON.stringify({ role: 'invalid' }),
			}));

			expect(res.status).toBe(400);
		});
	});

	describe('DELETE /tenants/:id/members/:memberId', () => {
		beforeEach(() => {
			mockData.userTenants.set('user-2_tenant-1', {
				user_id: 'user-2',
				tenant_id: 'tenant-1',
				role: 'member',
				created_at: Date.now(),
			});
		});

		it('should remove member (owner)', async () => {
			const res = await app.fetch(new Request('http://localhost/tenants/tenant-1/members/user-2', {
				method: 'DELETE',
				headers: { 'Authorization': 'Bearer valid_token_user1' },
			}));

			expect(res.status).toBe(200);
			const data = await res.json();
			expect(data.success).toBe(true);
		});

		it('should return 400 when trying to remove owner', async () => {
			const res = await app.fetch(new Request('http://localhost/tenants/tenant-1/members/user-1', {
				method: 'DELETE',
				headers: { 'Authorization': 'Bearer valid_token_user1' },
			}));

			expect(res.status).toBe(400);
		});

		it('should return 403 when member tries to remove', async () => {
			const res = await app.fetch(new Request('http://localhost/tenants/tenant-1/members/user-2', {
				method: 'DELETE',
				headers: { 'Authorization': 'Bearer valid_token_user2' },
			}));

			expect(res.status).toBe(403);
		});

		it('should allow admin to remove member/viewer', async () => {
			mockData.userTenants.set('user-3_tenant-1', {
				user_id: 'user-3',
				tenant_id: 'tenant-1',
				role: 'admin',
				created_at: Date.now(),
			});

			const res = await app.fetch(new Request('http://localhost/tenants/tenant-1/members/user-2', {
				method: 'DELETE',
				headers: { 'Authorization': 'Bearer valid_token_user3' },
			}));

			expect(res.status).toBe(200);
		});

		it('should return 403 when admin tries to remove another admin', async () => {
			mockData.userTenants.get('user-2_tenant-1')!.role = 'admin';

			mockData.userTenants.set('user-3_tenant-1', {
				user_id: 'user-3',
				tenant_id: 'tenant-1',
				role: 'admin',
				created_at: Date.now(),
			});

			const res = await app.fetch(new Request('http://localhost/tenants/tenant-1/members/user-2', {
				method: 'DELETE',
				headers: { 'Authorization': 'Bearer valid_token_user3' },
			}));

			expect(res.status).toBe(403);
		});

		it('should return 404 when member not found', async () => {
			const res = await app.fetch(new Request('http://localhost/tenants/tenant-1/members/nonexistent', {
				method: 'DELETE',
				headers: { 'Authorization': 'Bearer valid_token_user1' },
			}));

			expect(res.status).toBe(404);
		});
	});
});
