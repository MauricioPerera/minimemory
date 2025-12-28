/**
 * VectorPass MCP Server
 * OAuth-enabled MCP server for VectorPass vector database
 */

import { Env, OAuthToken } from './types';
import { AuthorizationServerMetadata } from './oauth/types';
import { handleAuthorizeGet, handleAuthorizePost } from './oauth/authorize';
import { handleToken } from './oauth/token';
import { handleRegister } from './oauth/register';
import { handleMCPRequest, handleSSE } from './mcp/server';
import { TOOLS } from './mcp/tools';

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const url = new URL(request.url);
    const path = url.pathname;
    const method = request.method;

    // CORS headers for all responses
    const corsHeaders = {
      'Access-Control-Allow-Origin': '*',
      'Access-Control-Allow-Methods': 'GET, POST, OPTIONS',
      'Access-Control-Allow-Headers': 'Content-Type, Authorization, X-API-Key',
    };

    // Handle preflight
    if (method === 'OPTIONS') {
      return new Response(null, { headers: corsHeaders });
    }

    try {
      let response: Response;

      // Route requests
      switch (true) {
        // Root - Server info (GET) or MCP endpoint (POST with JSON-RPC)
        case path === '/' || path === '':
          if (method === 'POST') {
            // Claude and some clients send MCP requests to root instead of /mcp
            response = await handleProtectedMCP(request, env);
          } else {
            response = handleRoot(url);
          }
          break;

        // Health check
        case path === '/health':
          response = new Response(JSON.stringify({ status: 'ok' }), {
            headers: { 'Content-Type': 'application/json' },
          });
          break;

        // OAuth Authorization Server Metadata (RFC 8414)
        case path === '/.well-known/oauth-authorization-server':
          response = handleMetadata(url);
          break;

        // OAuth Protected Resource Metadata (RFC 9728)
        case path === '/.well-known/oauth-protected-resource':
          response = handleProtectedResourceMetadata(url);
          break;

        // OAuth Authorize
        case path === '/authorize' && method === 'GET':
          response = await handleAuthorizeGet(request, env);
          break;

        case path === '/authorize' && method === 'POST':
          response = await handleAuthorizePost(request, env);
          break;

        // OAuth Token
        case path === '/token' && method === 'POST':
          response = await handleToken(request, env);
          break;

        // OAuth Dynamic Client Registration (RFC 7591)
        case path === '/register' && method === 'POST':
          response = await handleRegister(request, env);
          break;

        // MCP Endpoint (Streamable HTTP)
        case path === '/mcp' && method === 'POST':
          response = await handleProtectedMCP(request, env);
          break;

        // MCP SSE (fallback transport)
        case path === '/sse' && method === 'GET':
          response = await handleProtectedSSE(request, env);
          break;

        // Default: 404
        default:
          response = new Response(
            JSON.stringify({ error: 'Not found' }),
            { status: 404, headers: { 'Content-Type': 'application/json' } }
          );
      }

      // Add CORS headers
      const headers = new Headers(response.headers);
      Object.entries(corsHeaders).forEach(([key, value]) => {
        headers.set(key, value);
      });

      return new Response(response.body, {
        status: response.status,
        statusText: response.statusText,
        headers,
      });
    } catch (error) {
      console.error('Server error:', error);
      return new Response(
        JSON.stringify({
          error: 'Internal server error',
          message: error instanceof Error ? error.message : 'Unknown error',
        }),
        {
          status: 500,
          headers: { 'Content-Type': 'application/json', ...corsHeaders },
        }
      );
    }
  },
};

/**
 * Root endpoint - Server info and discovery
 */
function handleRoot(url: URL): Response {
  const baseUrl = `${url.protocol}//${url.host}`;

  const info = {
    name: 'VectorPass MCP Server',
    version: '1.0.0',
    description: 'MCP server for VectorPass vector database',
    mcp_endpoint: `${baseUrl}/mcp`,
    oauth: {
      authorization_endpoint: `${baseUrl}/authorize`,
      token_endpoint: `${baseUrl}/token`,
      registration_endpoint: `${baseUrl}/register`,
    },
    documentation: 'https://vectorpass.pages.dev',
  };

  return new Response(JSON.stringify(info, null, 2), {
    headers: { 'Content-Type': 'application/json' },
  });
}

/**
 * OAuth Authorization Server Metadata (RFC 8414)
 */
function handleMetadata(url: URL): Response {
  const issuer = `${url.protocol}//${url.host}`;

  const metadata: AuthorizationServerMetadata & { registration_endpoint?: string } = {
    issuer,
    authorization_endpoint: `${issuer}/authorize`,
    token_endpoint: `${issuer}/token`,
    registration_endpoint: `${issuer}/register`,
    response_types_supported: ['code'],
    grant_types_supported: ['authorization_code', 'refresh_token'],
    code_challenge_methods_supported: ['S256', 'plain'],
    token_endpoint_auth_methods_supported: ['none', 'client_secret_post'],
  };

  return new Response(JSON.stringify(metadata, null, 2), {
    headers: { 'Content-Type': 'application/json' },
  });
}

/**
 * OAuth Protected Resource Metadata (RFC 9728)
 */
function handleProtectedResourceMetadata(url: URL): Response {
  const resource = `${url.protocol}//${url.host}`;

  const metadata = {
    resource,
    authorization_servers: [resource],
    scopes_supported: ['mcp:read', 'mcp:write'],
    bearer_methods_supported: ['header'],
  };

  return new Response(JSON.stringify(metadata, null, 2), {
    headers: { 'Content-Type': 'application/json' },
  });
}

/**
 * Validate Bearer token and get token data
 */
async function validateToken(
  request: Request,
  env: Env
): Promise<OAuthToken | null> {
  const authHeader = request.headers.get('Authorization');

  if (!authHeader || !authHeader.startsWith('Bearer ')) {
    return null;
  }

  const token = authHeader.slice(7);

  // Look up token
  const tokenData = await env.OAUTH_TOKENS.get(`access:${token}`);
  if (!tokenData) {
    return null;
  }

  const parsed: OAuthToken = JSON.parse(tokenData);

  // Check expiration
  if (Date.now() > parsed.expiresAt) {
    await env.OAUTH_TOKENS.delete(`access:${token}`);
    return null;
  }

  return parsed;
}

/**
 * Handle protected MCP endpoint
 */
async function handleProtectedMCP(
  request: Request,
  env: Env
): Promise<Response> {
  const tokenData = await validateToken(request, env);

  if (!tokenData) {
    return new Response(
      JSON.stringify({
        jsonrpc: '2.0',
        id: null,
        error: {
          code: -32001,
          message: 'Unauthorized: Invalid or missing access token',
        },
      }),
      {
        status: 401,
        headers: {
          'Content-Type': 'application/json',
          'WWW-Authenticate': 'Bearer',
        },
      }
    );
  }

  return handleMCPRequest(request, tokenData, env);
}

/**
 * Handle protected SSE endpoint
 */
async function handleProtectedSSE(
  request: Request,
  env: Env
): Promise<Response> {
  const tokenData = await validateToken(request, env);

  if (!tokenData) {
    return new Response('Unauthorized', {
      status: 401,
      headers: { 'WWW-Authenticate': 'Bearer' },
    });
  }

  return handleSSE(request, tokenData, env);
}
