/**
 * MCP Server Implementation
 * Handles MCP protocol messages
 */

import { Env, OAuthToken } from '../types';
import { VectorPassClient } from '../vectorpass-client';
import { TOOLS, executeTool } from './tools';

interface MCPMessage {
  jsonrpc: '2.0';
  id?: string | number | null;
  method?: string;
  params?: any;
  result?: any;
  error?: { code: number; message: string; data?: any };
}

/**
 * Handle MCP request
 */
export async function handleMCPRequest(
  request: Request,
  tokenData: OAuthToken,
  env: Env
): Promise<Response> {
  const client = new VectorPassClient(tokenData.apiKey, env);

  // Parse the MCP message
  let message: MCPMessage;
  try {
    message = await request.json();
  } catch {
    return jsonRpcError(null, -32700, 'Parse error');
  }

  // Validate JSON-RPC 2.0 format
  if (message.jsonrpc !== '2.0') {
    return jsonRpcError(message.id, -32600, 'Invalid Request: jsonrpc must be "2.0"');
  }

  if (!message.method) {
    return jsonRpcError(message.id, -32600, 'Invalid Request: method is required');
  }

  console.log(`MCP Request: method=${message.method}, id=${message.id}`);

  // Handle MCP methods
  switch (message.method) {
    case 'initialize':
      return handleInitialize(message);

    // Handle initialized notification (no response needed for notifications)
    case 'initialized':
    case 'notifications/initialized':
      // Notifications don't require a response, but we send an empty success
      return jsonRpcSuccess(message.id, {});

    case 'tools/list':
      return handleToolsList(message);

    case 'tools/call':
      return handleToolsCall(message, client);

    case 'ping':
      return jsonRpcSuccess(message.id, {});

    // Resource methods (not implemented but respond gracefully)
    case 'resources/list':
      return jsonRpcSuccess(message.id, { resources: [] });

    case 'prompts/list':
      return jsonRpcSuccess(message.id, { prompts: [] });

    default:
      return jsonRpcError(message.id, -32601, `Method not found: ${message.method}`);
  }
}

/**
 * Handle MCP initialize
 */
function handleInitialize(message: MCPMessage): Response {
  console.log('MCP Initialize called with params:', JSON.stringify(message.params));

  return jsonRpcSuccess(message.id, {
    protocolVersion: '2024-11-05',
    capabilities: {
      tools: {
        listChanged: true,
      },
      resources: {},
      prompts: {},
    },
    serverInfo: {
      name: 'vectorpass',
      version: '1.0.0',
    },
  });
}

/**
 * Handle tools/list
 */
function handleToolsList(message: MCPMessage): Response {
  console.log('MCP tools/list called, returning', TOOLS.length, 'tools');

  return jsonRpcSuccess(message.id, {
    tools: TOOLS,
  });
}

/**
 * Handle tools/call
 */
async function handleToolsCall(
  message: MCPMessage,
  client: VectorPassClient
): Promise<Response> {
  const { name, arguments: args } = message.params || {};

  if (!name) {
    return jsonRpcError(message.id, -32602, 'Invalid params: tool name is required');
  }

  // Check if tool exists
  const tool = TOOLS.find((t) => t.name === name);
  if (!tool) {
    return jsonRpcError(message.id, -32602, `Unknown tool: ${name}`);
  }

  // Execute tool
  const result = await executeTool(name, args || {}, client);

  return jsonRpcSuccess(message.id, result);
}

/**
 * Create JSON-RPC success response
 */
function jsonRpcSuccess(id: string | number | null | undefined, result: any): Response {
  const response: MCPMessage = {
    jsonrpc: '2.0',
    id: id ?? null,
    result,
  };

  return new Response(JSON.stringify(response), {
    headers: { 'Content-Type': 'application/json' },
  });
}

/**
 * Create JSON-RPC error response
 */
function jsonRpcError(
  id: string | number | null | undefined,
  code: number,
  message: string,
  data?: any
): Response {
  const response: MCPMessage = {
    jsonrpc: '2.0',
    id: id ?? null,
    error: { code, message, data },
  };

  return new Response(JSON.stringify(response), {
    headers: { 'Content-Type': 'application/json' },
  });
}

/**
 * Handle SSE transport (for clients that don't support Streamable HTTP)
 */
export async function handleSSE(
  request: Request,
  tokenData: OAuthToken,
  env: Env
): Promise<Response> {
  const client = new VectorPassClient(tokenData.apiKey, env);

  // Create SSE stream
  const { readable, writable } = new TransformStream();
  const writer = writable.getWriter();
  const encoder = new TextEncoder();

  // Send initial connection event
  const sendEvent = async (event: string, data: any) => {
    await writer.write(encoder.encode(`event: ${event}\n`));
    await writer.write(encoder.encode(`data: ${JSON.stringify(data)}\n\n`));
  };

  // Handle incoming messages via query parameter or POST to /sse/message
  const url = new URL(request.url);
  const messageParam = url.searchParams.get('message');

  if (messageParam) {
    try {
      const message = JSON.parse(messageParam);
      // Process and respond via SSE
      // This is a simplified version - full SSE would need persistent connections
    } catch {
      // Ignore parse errors
    }
  }

  // Send server info
  sendEvent('message', {
    jsonrpc: '2.0',
    method: 'notifications/initialized',
    params: {
      protocolVersion: '2024-11-05',
      capabilities: { tools: {} },
      serverInfo: { name: 'vectorpass', version: '1.0.0' },
    },
  });

  return new Response(readable, {
    headers: {
      'Content-Type': 'text/event-stream',
      'Cache-Control': 'no-cache',
      Connection: 'keep-alive',
    },
  });
}
