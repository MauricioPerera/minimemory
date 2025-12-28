/**
 * OAuth Dynamic Client Registration (RFC 7591)
 * Allows clients like ChatGPT to register themselves dynamically
 */

import { Env } from '../types';

// Client registration request (RFC 7591)
export interface ClientRegistrationRequest {
  redirect_uris: string[];
  token_endpoint_auth_method?: string;
  grant_types?: string[];
  response_types?: string[];
  client_name?: string;
  client_uri?: string;
  logo_uri?: string;
  scope?: string;
  contacts?: string[];
  tos_uri?: string;
  policy_uri?: string;
  software_id?: string;
  software_version?: string;
}

// Client registration response (RFC 7591)
export interface ClientRegistrationResponse {
  client_id: string;
  client_secret?: string;
  client_id_issued_at: number;
  client_secret_expires_at?: number;
  redirect_uris: string[];
  token_endpoint_auth_method: string;
  grant_types: string[];
  response_types: string[];
  client_name?: string;
  client_uri?: string;
  logo_uri?: string;
  scope?: string;
}

// Stored client data
export interface RegisteredClient {
  clientId: string;
  clientSecret?: string;
  redirectUris: string[];
  clientName?: string;
  clientUri?: string;
  grantTypes: string[];
  responseTypes: string[];
  tokenEndpointAuthMethod: string;
  createdAt: number;
  softwareId?: string;
}

/**
 * Generate a random client ID
 */
function generateClientId(): string {
  const chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
  let result = 'vp_client_';
  for (let i = 0; i < 24; i++) {
    result += chars.charAt(Math.floor(Math.random() * chars.length));
  }
  return result;
}

/**
 * Generate a random client secret
 */
function generateClientSecret(): string {
  const chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
  let result = 'vp_secret_';
  for (let i = 0; i < 32; i++) {
    result += chars.charAt(Math.floor(Math.random() * chars.length));
  }
  return result;
}

/**
 * Handle POST /register - Dynamic Client Registration
 */
export async function handleRegister(
  request: Request,
  env: Env
): Promise<Response> {
  // Only accept POST
  if (request.method !== 'POST') {
    return errorResponse('invalid_request', 'Method not allowed', 405);
  }

  // Parse request body
  let body: ClientRegistrationRequest;
  try {
    body = await request.json();
  } catch {
    return errorResponse('invalid_request', 'Invalid JSON body');
  }

  // Validate required fields
  if (!body.redirect_uris || !Array.isArray(body.redirect_uris) || body.redirect_uris.length === 0) {
    return errorResponse('invalid_redirect_uri', 'redirect_uris is required and must be a non-empty array');
  }

  // Validate redirect URIs
  for (const uri of body.redirect_uris) {
    try {
      const parsed = new URL(uri);
      // Allow http only for localhost
      if (parsed.protocol !== 'https:' && !parsed.hostname.match(/^(localhost|127\.0\.0\.1)$/)) {
        return errorResponse('invalid_redirect_uri', `Invalid redirect URI: ${uri} - HTTPS required for non-localhost`);
      }
    } catch {
      return errorResponse('invalid_redirect_uri', `Invalid redirect URI: ${uri}`);
    }
  }

  // Generate client credentials
  const clientId = generateClientId();
  const clientSecret = generateClientSecret();
  const now = Math.floor(Date.now() / 1000);

  // Determine grant types (default to authorization_code)
  const grantTypes = body.grant_types || ['authorization_code', 'refresh_token'];
  const responseTypes = body.response_types || ['code'];
  const tokenEndpointAuthMethod = body.token_endpoint_auth_method || 'none';

  // Store client registration
  const client: RegisteredClient = {
    clientId,
    clientSecret,
    redirectUris: body.redirect_uris,
    clientName: body.client_name,
    clientUri: body.client_uri,
    grantTypes,
    responseTypes,
    tokenEndpointAuthMethod,
    createdAt: now,
    softwareId: body.software_id,
  };

  // Store in KV (using OAUTH_SESSIONS as we don't have a dedicated clients namespace)
  await env.OAUTH_SESSIONS.put(
    `client:${clientId}`,
    JSON.stringify(client),
    { expirationTtl: 365 * 24 * 60 * 60 } // 1 year
  );

  // Build response
  const response: ClientRegistrationResponse = {
    client_id: clientId,
    client_secret: clientSecret,
    client_id_issued_at: now,
    client_secret_expires_at: 0, // Never expires
    redirect_uris: body.redirect_uris,
    token_endpoint_auth_method: tokenEndpointAuthMethod,
    grant_types: grantTypes,
    response_types: responseTypes,
    client_name: body.client_name,
    client_uri: body.client_uri,
    logo_uri: body.logo_uri,
    scope: body.scope,
  };

  return new Response(JSON.stringify(response), {
    status: 201,
    headers: {
      'Content-Type': 'application/json',
      'Cache-Control': 'no-store',
    },
  });
}

/**
 * Get a registered client by ID
 */
export async function getRegisteredClient(
  clientId: string,
  env: Env
): Promise<RegisteredClient | null> {
  const data = await env.OAUTH_SESSIONS.get(`client:${clientId}`);
  if (!data) {
    return null;
  }
  return JSON.parse(data);
}

/**
 * Validate client credentials
 */
export async function validateClient(
  clientId: string,
  clientSecret: string | null,
  redirectUri: string,
  env: Env
): Promise<{ valid: boolean; error?: string }> {
  const client = await getRegisteredClient(clientId, env);

  if (!client) {
    // Allow unregistered clients with any redirect URI (for backwards compatibility)
    // In production, you might want to be stricter
    return { valid: true };
  }

  // Validate redirect URI
  if (!client.redirectUris.includes(redirectUri)) {
    return { valid: false, error: 'Invalid redirect URI for this client' };
  }

  // Validate client secret if provided and client has one
  if (client.clientSecret && clientSecret && client.clientSecret !== clientSecret) {
    return { valid: false, error: 'Invalid client secret' };
  }

  return { valid: true };
}

/**
 * Error response helper
 */
function errorResponse(error: string, description: string, status: number = 400): Response {
  return new Response(
    JSON.stringify({
      error,
      error_description: description,
    }),
    {
      status,
      headers: {
        'Content-Type': 'application/json',
        'Cache-Control': 'no-store',
      },
    }
  );
}
