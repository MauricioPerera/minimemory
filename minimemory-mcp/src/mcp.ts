/**
 * MinimemoryMCP - MCP Server for minimemory
 *
 * Extends McpAgent to provide memory operations to AI agents
 * with permission filtering based on agent tokens
 */

import { McpAgent } from 'agents/mcp';
import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { z } from 'zod';
import { MinimemoryClient } from './client/minimemory.js';

interface Env {
	MCP_OBJECT: DurableObjectNamespace;
}

// Agent context passed from the worker
interface AgentContext {
	minimemoryUrl: string;
	apiKey: string;
	userId: string;
	tenantId: string;
	agentToken: string;
	agentName: string;
	allowedMemories: string[]; // ["mem_123"] or ["*"]
	permissions: string[]; // ["read", "write"]
}

// Zod schemas for tool inputs
const rememberSchema = {
	content: z.string().describe('The memory content to store'),
	type: z
		.enum(['episodic', 'semantic', 'working'])
		.optional()
		.default('semantic')
		.describe('Memory type: episodic (events), semantic (facts), working (temporary)'),
	importance: z
		.number()
		.min(0)
		.max(1)
		.optional()
		.default(0.5)
		.describe('Importance score from 0 to 1'),
	metadata: z
		.record(z.unknown())
		.optional()
		.describe('Optional metadata key-value pairs'),
};

const recallSchema = {
	query: z.string().describe('Search query to find relevant memories'),
	type: z
		.enum(['episodic', 'semantic', 'working', 'knowledge'])
		.optional()
		.describe('Filter by memory type'),
	limit: z
		.number()
		.min(1)
		.max(50)
		.optional()
		.default(10)
		.describe('Maximum number of results to return'),
	threshold: z
		.number()
		.min(0)
		.max(1)
		.optional()
		.default(0.7)
		.describe('Minimum similarity threshold'),
	mode: z
		.enum(['vector', 'keyword', 'hybrid'])
		.optional()
		.default('hybrid')
		.describe('Search mode: vector (semantic), keyword (BM25), or hybrid'),
};

const getSchema = {
	id: z.string().describe('Memory ID to retrieve'),
};

const forgetSchema = {
	id: z.string().describe('Memory ID to delete'),
};

const ingestSchema = {
	content: z.string().describe('Document content to ingest'),
	name: z.string().describe('Document name or title'),
	type: z
		.enum(['document', 'webpage', 'code', 'note'])
		.optional()
		.default('document')
		.describe('Document type'),
	chunking: z
		.object({
			strategy: z
				.enum(['fixed', 'semantic', 'paragraph'])
				.optional()
				.describe('Chunking strategy'),
			maxChunkSize: z
				.number()
				.optional()
				.describe('Maximum chunk size in characters'),
			overlap: z
				.number()
				.optional()
				.describe('Overlap between chunks in characters'),
		})
		.optional()
		.describe('Chunking options'),
};

export class MinimemoryMCP extends McpAgent<Env, unknown, Record<string, unknown>> {
	// MCP Server instance
	server = new McpServer(
		{
			name: 'minimemory',
			version: '0.1.0',
		},
		{
			capabilities: {
				tools: {},
			},
		}
	);

	private client: MinimemoryClient | null = null;
	private agentContext: AgentContext | null = null;

	/**
	 * Extract agent context from request headers
	 */
	private extractAgentContext(request: Request): AgentContext | null {
		const minimemoryUrl = request.headers.get('X-Minimemory-Url');
		const apiKey = request.headers.get('X-Api-Key');
		const userId = request.headers.get('X-User-Id');
		const agentToken = request.headers.get('X-Agent-Token');

		if (!minimemoryUrl || !apiKey || !agentToken) {
			return null;
		}

		let allowedMemories: string[] = ['*'];
		let permissions: string[] = ['read', 'write'];

		try {
			const memoriesHeader = request.headers.get('X-Allowed-Memories');
			if (memoriesHeader) {
				allowedMemories = JSON.parse(memoriesHeader);
			}
			const permsHeader = request.headers.get('X-Permissions');
			if (permsHeader) {
				permissions = JSON.parse(permsHeader);
			}
		} catch {
			// Keep defaults
		}

		return {
			minimemoryUrl,
			apiKey,
			userId: userId || '',
			tenantId: request.headers.get('X-Tenant-Id') || '',
			agentToken,
			agentName: request.headers.get('X-Agent-Name') || 'Unknown Agent',
			allowedMemories,
			permissions,
		};
	}

	/**
	 * Check if the agent has a specific permission
	 */
	private hasPermission(permission: 'read' | 'write'): boolean {
		if (!this.agentContext) return false;
		return this.agentContext.permissions.includes(permission);
	}

	/**
	 * Check if the agent can access a specific memory
	 */
	private canAccessMemory(memoryId: string): boolean {
		if (!this.agentContext) return false;

		// Wildcard access
		if (this.agentContext.allowedMemories.includes('*')) return true;

		// Specific memory access
		return this.agentContext.allowedMemories.includes(memoryId);
	}

	/**
	 * Filter a list of memories to only those the agent can access
	 */
	private filterAccessibleMemories<T extends { id: string }>(memories: T[]): T[] {
		if (!this.agentContext) return [];

		// Wildcard access - return all
		if (this.agentContext.allowedMemories.includes('*')) return memories;

		// Filter to allowed memories
		return memories.filter((m) => this.agentContext!.allowedMemories.includes(m.id));
	}

	/**
	 * Get the MinimemoryClient instance
	 */
	private getClient(): MinimemoryClient {
		if (!this.client && this.agentContext) {
			this.client = new MinimemoryClient(
				this.agentContext.minimemoryUrl,
				this.agentContext.apiKey,
				'default' // Namespace is handled by the API key
			);
		}
		if (!this.client) {
			throw new Error('Client not initialized - missing agent context');
		}
		return this.client;
	}

	/**
	 * Handle incoming requests - extract context before processing
	 */
	async fetch(request: Request): Promise<Response> {
		// Extract agent context from headers
		this.agentContext = this.extractAgentContext(request);

		if (!this.agentContext) {
			return new Response(
				JSON.stringify({
					error: 'Missing agent context',
					hint: 'This endpoint requires authentication via api_key and agent_token',
				}),
				{
					status: 401,
					headers: { 'Content-Type': 'application/json' },
				}
			);
		}

		// Reset client to use new context
		this.client = null;

		// Call parent fetch
		return super.fetch(request);
	}

	/**
	 * Initialize the MCP server with tools
	 */
	async init(): Promise<void> {
		// Register remember tool
		this.server.registerTool(
			'remember',
			{
				description: 'Store a new memory in the memory bank. Use this to save information for later retrieval.',
				inputSchema: rememberSchema,
			},
			async (args) => {
				// Check write permission
				if (!this.hasPermission('write')) {
					return {
						content: [
							{
								type: 'text' as const,
								text: 'Permission denied: write access not allowed for this agent',
							},
						],
						isError: true,
					};
				}

				const client = this.getClient();

				try {
					const result = await client.remember({
						content: args.content,
						type: args.type as 'episodic' | 'semantic' | 'working' | undefined,
						importance: args.importance,
						metadata: args.metadata as Record<string, unknown> | undefined,
					});

					return {
						content: [
							{
								type: 'text' as const,
								text: `Memory stored successfully!\n\nID: ${result.id}\nType: ${result.type}\nImportance: ${result.importance}`,
							},
						],
					};
				} catch (error) {
					return {
						content: [
							{
								type: 'text' as const,
								text: `Error storing memory: ${error instanceof Error ? error.message : 'Unknown error'}`,
							},
						],
						isError: true,
					};
				}
			}
		);

		// Register recall tool
		this.server.registerTool(
			'recall',
			{
				description: 'Search for memories similar to a query. Use this to retrieve previously stored information.',
				inputSchema: recallSchema,
			},
			async (args) => {
				// Check read permission
				if (!this.hasPermission('read')) {
					return {
						content: [
							{
								type: 'text' as const,
								text: 'Permission denied: read access not allowed for this agent',
							},
						],
						isError: true,
					};
				}

				const client = this.getClient();

				try {
					const result = await client.recall({
						query: args.query,
						type: args.type as 'episodic' | 'semantic' | 'working' | 'knowledge' | undefined,
						limit: args.limit,
						threshold: args.threshold,
						mode: args.mode as 'vector' | 'keyword' | 'hybrid' | undefined,
					});

					// Filter to only accessible memories
					const accessibleMemories = this.filterAccessibleMemories(result.memories);

					if (accessibleMemories.length === 0) {
						return {
							content: [
								{
									type: 'text' as const,
									text: 'No memories found matching your query.',
								},
							],
						};
					}

					const memoriesText = accessibleMemories
						.map((m, i) => {
							const score = m.score ? ` (score: ${m.score.toFixed(3)})` : '';
							return `${i + 1}. [${m.type}] ${m.content}${score}`;
						})
						.join('\n\n');

					return {
						content: [
							{
								type: 'text' as const,
								text: `Found ${accessibleMemories.length} memories:\n\n${memoriesText}`,
							},
						],
					};
				} catch (error) {
					return {
						content: [
							{
								type: 'text' as const,
								text: `Error searching memories: ${error instanceof Error ? error.message : 'Unknown error'}`,
							},
						],
						isError: true,
					};
				}
			}
		);

		// Register get tool
		this.server.registerTool(
			'get',
			{
				description: 'Get a specific memory by its ID.',
				inputSchema: getSchema,
			},
			async (args) => {
				// Check read permission
				if (!this.hasPermission('read')) {
					return {
						content: [
							{
								type: 'text' as const,
								text: 'Permission denied: read access not allowed for this agent',
							},
						],
						isError: true,
					};
				}

				// Check memory access
				if (!this.canAccessMemory(args.id)) {
					return {
						content: [
							{
								type: 'text' as const,
								text: `Memory not accessible: ${args.id} is not in the allowed memories list`,
							},
						],
						isError: true,
					};
				}

				const client = this.getClient();

				try {
					const result = await client.get(args.id);

					if (!result.memory) {
						return {
							content: [
								{
									type: 'text' as const,
									text: `Memory not found: ${args.id}`,
								},
							],
						};
					}

					return {
						content: [
							{
								type: 'text' as const,
								text: JSON.stringify(result.memory, null, 2),
							},
						],
					};
				} catch (error) {
					return {
						content: [
							{
								type: 'text' as const,
								text: `Error getting memory: ${error instanceof Error ? error.message : 'Unknown error'}`,
							},
						],
						isError: true,
					};
				}
			}
		);

		// Register forget tool
		this.server.registerTool(
			'forget',
			{
				description: 'Delete a memory by its ID. Use this to remove information that is no longer needed.',
				inputSchema: forgetSchema,
			},
			async (args) => {
				// Check write permission
				if (!this.hasPermission('write')) {
					return {
						content: [
							{
								type: 'text' as const,
								text: 'Permission denied: write access not allowed for this agent',
							},
						],
						isError: true,
					};
				}

				// Check memory access
				if (!this.canAccessMemory(args.id)) {
					return {
						content: [
							{
								type: 'text' as const,
								text: `Memory not accessible: ${args.id} is not in the allowed memories list`,
							},
						],
						isError: true,
					};
				}

				const client = this.getClient();

				try {
					const result = await client.forget(args.id);

					return {
						content: [
							{
								type: 'text' as const,
								text: result.deleted
									? `Memory ${args.id} deleted successfully.`
									: `Memory ${args.id} not found.`,
							},
						],
					};
				} catch (error) {
					return {
						content: [
							{
								type: 'text' as const,
								text: `Error deleting memory: ${error instanceof Error ? error.message : 'Unknown error'}`,
							},
						],
						isError: true,
					};
				}
			}
		);

		// Register stats tool
		this.server.registerTool(
			'stats',
			{
				description: 'Get statistics about the memory bank, including total memories and counts by type.',
			},
			async () => {
				// Check read permission
				if (!this.hasPermission('read')) {
					return {
						content: [
							{
								type: 'text' as const,
								text: 'Permission denied: read access not allowed for this agent',
							},
						],
						isError: true,
					};
				}

				const client = this.getClient();

				try {
					const result = await client.stats();

					const byTypeText = Object.entries(result.byType || {})
						.map(([type, count]) => `  - ${type}: ${count}`)
						.join('\n');

					return {
						content: [
							{
								type: 'text' as const,
								text: `Memory Statistics for namespace "${result.namespace}":\n\nTotal memories: ${result.total}\n\nBy type:\n${byTypeText || '  (no memories)'}`,
							},
						],
					};
				} catch (error) {
					return {
						content: [
							{
								type: 'text' as const,
								text: `Error getting stats: ${error instanceof Error ? error.message : 'Unknown error'}`,
							},
						],
						isError: true,
					};
				}
			}
		);

		// Register ingest tool
		this.server.registerTool(
			'ingest',
			{
				description: 'Ingest a document into the knowledge bank for RAG. The document will be chunked and embedded for later retrieval.',
				inputSchema: ingestSchema,
			},
			async (args) => {
				// Check write permission
				if (!this.hasPermission('write')) {
					return {
						content: [
							{
								type: 'text' as const,
								text: 'Permission denied: write access not allowed for this agent',
							},
						],
						isError: true,
					};
				}

				const client = this.getClient();

				try {
					const result = await client.ingest({
						content: args.content,
						name: args.name,
						type: args.type as 'document' | 'webpage' | 'code' | 'note' | undefined,
						chunking: args.chunking as {
							strategy?: 'fixed' | 'semantic' | 'paragraph';
							maxChunkSize?: number;
							overlap?: number;
						} | undefined,
					});

					return {
						content: [
							{
								type: 'text' as const,
								text: `Document "${result.sourceName}" ingested successfully!\n\n- Source ID: ${result.sourceId}\n- Chunks created: ${result.chunksCreated}\n- Total characters: ${result.totalCharacters}\n- Embeddings generated: ${result.embeddingsGenerated ? 'Yes' : 'No'}`,
							},
						],
					};
				} catch (error) {
					return {
						content: [
							{
								type: 'text' as const,
								text: `Error ingesting document: ${error instanceof Error ? error.message : 'Unknown error'}`,
							},
						],
						isError: true,
					};
				}
			}
		);
	}
}
