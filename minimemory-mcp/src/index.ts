/**
 * minimemory-mcp Worker Entry Point
 *
 * Routes MCP requests to the MinimemoryMCP Durable Object
 * with dual authentication (API Key + Agent Token)
 */

import { MinimemoryMCP } from './mcp.js';

interface Env {
	MINIMEMORY_URL: string;
	MCP_OBJECT: DurableObjectNamespace;
}

interface AgentValidationResult {
	valid: boolean;
	error?: string;
	userId?: string;
	tenantId?: string;
	agentTokenId?: string;
	agentName?: string;
	allowedMemories?: string[];
	permissions?: string[];
	expiresAt?: number;
}

/**
 * Validate API key and agent token against the minimemory service
 */
async function validateAgent(
	minimemoryUrl: string,
	apiKey: string,
	agentToken: string
): Promise<AgentValidationResult> {
	try {
		const response = await fetch(`${minimemoryUrl}/api/v1/auth/validate-agent`, {
			method: 'POST',
			headers: {
				'Content-Type': 'application/json',
			},
			body: JSON.stringify({ apiKey, agentToken }),
		});

		const result = (await response.json()) as AgentValidationResult;
		return result;
	} catch (error) {
		console.error('Agent validation error:', error);
		return {
			valid: false,
			error: error instanceof Error ? error.message : 'Validation request failed',
		};
	}
}

export default {
	async fetch(request: Request, env: Env, ctx: ExecutionContext): Promise<Response> {
		const url = new URL(request.url);

		// Health check endpoint
		if (url.pathname === '/' || url.pathname === '/health') {
			return new Response(
				JSON.stringify({
					name: 'minimemory-mcp',
					version: '0.1.0',
					status: 'ok',
					description: 'MCP Server for minimemory - enables AI agents to read and write memories',
					authentication: {
						method: 'Query parameters',
						required: ['api_key', 'agent_token'],
						example: '/sse?api_key=mm_xxx&agent_token=at_yyy',
					},
					endpoints: {
						'/sse': 'MCP SSE endpoint',
						'/mcp': 'MCP SSE endpoint (alias)',
					},
					tools: ['remember', 'recall', 'get', 'forget', 'stats', 'ingest'],
				}),
				{
					headers: { 'Content-Type': 'application/json' },
				}
			);
		}

		// MCP SSE endpoint - route to Durable Object with authentication
		if (url.pathname === '/sse' || url.pathname === '/mcp') {
			// Extract credentials from query parameters
			const apiKey = url.searchParams.get('api_key');
			const agentToken = url.searchParams.get('agent_token');

			// Validate required parameters
			if (!apiKey) {
				return new Response(
					JSON.stringify({
						error: 'api_key query parameter is required',
						hint: 'Add ?api_key=your_key&agent_token=your_token to the URL',
					}),
					{
						status: 401,
						headers: { 'Content-Type': 'application/json' },
					}
				);
			}

			if (!agentToken) {
				return new Response(
					JSON.stringify({
						error: 'agent_token query parameter is required',
						hint: 'Create an agent token in the minimemory dashboard',
					}),
					{
						status: 401,
						headers: { 'Content-Type': 'application/json' },
					}
				);
			}

			// Validate credentials against minimemory service
			const validation = await validateAgent(env.MINIMEMORY_URL, apiKey, agentToken);

			if (!validation.valid) {
				return new Response(
					JSON.stringify({
						error: validation.error || 'Invalid credentials',
						hint: 'Check your API key and agent token',
					}),
					{
						status: 401,
						headers: { 'Content-Type': 'application/json' },
					}
				);
			}

			// Create a unique DO ID for this agent token
			// This ensures each token gets its own session
			const id = env.MCP_OBJECT.idFromName(`agent-${agentToken}`);
			const stub = env.MCP_OBJECT.get(id);

			// Create a new request with validated context in headers
			const newRequest = new Request(request.url, {
				method: request.method,
				headers: new Headers({
					...Object.fromEntries(request.headers),
					// Pass validated context to the Durable Object
					'X-Minimemory-Url': env.MINIMEMORY_URL,
					'X-Api-Key': apiKey,
					'X-User-Id': validation.userId || '',
					'X-Tenant-Id': validation.tenantId || '',
					'X-Agent-Token': agentToken,
					'X-Agent-Name': validation.agentName || '',
					'X-Allowed-Memories': JSON.stringify(validation.allowedMemories || []),
					'X-Permissions': JSON.stringify(validation.permissions || []),
				}),
				body: request.body,
				// @ts-ignore - duplex is needed for streaming but not in types
				duplex: 'half',
			});

			// Forward the request to the Durable Object
			return stub.fetch(newRequest);
		}

		// 404 for unknown paths
		return new Response(
			JSON.stringify({
				error: 'Not found',
				path: url.pathname,
				hint: 'Use /sse or /mcp for MCP connections',
			}),
			{
				status: 404,
				headers: { 'Content-Type': 'application/json' },
			}
		);
	},
};

// Export the Durable Object class
export { MinimemoryMCP };
