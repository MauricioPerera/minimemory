import { useState } from 'react';
import { ChevronDown, ChevronRight, Copy, Check, ExternalLink } from 'lucide-react';

interface Endpoint {
  method: 'GET' | 'POST' | 'PUT' | 'PATCH' | 'DELETE';
  path: string;
  description: string;
  auth: 'API Key' | 'JWT' | 'Both';
  headers?: Record<string, string>;
  body?: object;
  response?: object;
  params?: Array<{
    name: string;
    type: string;
    required: boolean;
    description: string;
  }>;
}

interface EndpointCategory {
  name: string;
  description: string;
  endpoints: Endpoint[];
}

const API_URL = import.meta.env.VITE_API_URL || 'https://minimemory-service.rckflr.workers.dev';

const categories: EndpointCategory[] = [
  {
    name: 'Memory Operations',
    description: 'Core endpoints for storing and retrieving memories',
    endpoints: [
      {
        method: 'POST',
        path: '/api/v1/remember',
        description: 'Store a new memory with optional embedding',
        auth: 'Both',
        headers: {
          'Content-Type': 'application/json',
          'X-API-Key': 'your-api-key',
          'X-Namespace': 'default',
        },
        body: {
          content: 'The user prefers dark mode',
          type: 'semantic',
          importance: 0.8,
          metadata: { source: 'settings', tags: ['preference'] },
        },
        response: {
          success: true,
          id: 'mem_abc123',
          persisted: true,
        },
        params: [
          { name: 'content', type: 'string', required: true, description: 'The memory content text' },
          { name: 'type', type: 'string', required: false, description: 'Memory type: episodic, semantic, or working' },
          { name: 'importance', type: 'number', required: false, description: 'Importance score 0-1 (default: 0.5)' },
          { name: 'metadata', type: 'object', required: false, description: 'Additional metadata as JSON' },
          { name: 'embedding', type: 'number[]', required: false, description: 'Pre-computed embedding vector' },
          { name: 'sessionId', type: 'string', required: false, description: 'Session ID for working memory' },
          { name: 'ttl', type: 'number', required: false, description: 'Time-to-live in ms for working memory' },
        ],
      },
      {
        method: 'POST',
        path: '/api/v1/recall',
        description: 'Search for memories using vector, keyword, or hybrid search',
        auth: 'Both',
        headers: {
          'Content-Type': 'application/json',
          'X-API-Key': 'your-api-key',
          'X-Namespace': 'default',
        },
        body: {
          keywords: 'user preferences',
          mode: 'hybrid',
          limit: 10,
          type: 'semantic',
          minImportance: 0.5,
        },
        response: {
          success: true,
          results: [
            {
              id: 'mem_abc123',
              content: 'The user prefers dark mode',
              type: 'semantic',
              importance: 0.8,
              score: 0.95,
              metadata: { source: 'settings' },
              createdAt: 1700000000000,
            },
          ],
          count: 1,
          mode: 'hybrid',
        },
        params: [
          { name: 'keywords', type: 'string', required: false, description: 'Search keywords for BM25' },
          { name: 'embedding', type: 'number[]', required: false, description: 'Query embedding for vector search' },
          { name: 'mode', type: 'string', required: false, description: 'Search mode: vector, keyword, or hybrid' },
          { name: 'limit', type: 'number', required: false, description: 'Max results (default: 10)' },
          { name: 'type', type: 'string', required: false, description: 'Filter by memory type' },
          { name: 'minImportance', type: 'number', required: false, description: 'Minimum importance threshold' },
          { name: 'sessionId', type: 'string', required: false, description: 'Filter by session ID' },
        ],
      },
      {
        method: 'GET',
        path: '/api/v1/memory/:id',
        description: 'Get a specific memory by ID',
        auth: 'Both',
        headers: {
          'X-API-Key': 'your-api-key',
          'X-Namespace': 'default',
        },
        response: {
          success: true,
          memory: {
            id: 'mem_abc123',
            content: 'The user prefers dark mode',
            type: 'semantic',
            importance: 0.8,
            metadata: { source: 'settings' },
            createdAt: 1700000000000,
            updatedAt: 1700000000000,
            accessCount: 5,
          },
        },
        params: [
          { name: 'id', type: 'string', required: true, description: 'Memory ID (URL parameter)' },
        ],
      },
      {
        method: 'PATCH',
        path: '/api/v1/memory/:id',
        description: 'Update a memory',
        auth: 'Both',
        headers: {
          'Content-Type': 'application/json',
          'X-API-Key': 'your-api-key',
          'X-Namespace': 'default',
        },
        body: {
          content: 'Updated content',
          importance: 0.9,
          metadata: { updated: true },
        },
        response: {
          success: true,
          id: 'mem_abc123',
        },
        params: [
          { name: 'id', type: 'string', required: true, description: 'Memory ID (URL parameter)' },
          { name: 'content', type: 'string', required: false, description: 'New content' },
          { name: 'importance', type: 'number', required: false, description: 'New importance score' },
          { name: 'metadata', type: 'object', required: false, description: 'New metadata (merged)' },
        ],
      },
      {
        method: 'DELETE',
        path: '/api/v1/forget/:id',
        description: 'Delete a specific memory',
        auth: 'Both',
        headers: {
          'X-API-Key': 'your-api-key',
          'X-Namespace': 'default',
        },
        response: {
          success: true,
          deleted: true,
        },
        params: [
          { name: 'id', type: 'string', required: true, description: 'Memory ID (URL parameter)' },
        ],
      },
      {
        method: 'POST',
        path: '/api/v1/forget',
        description: 'Delete memories by filter',
        auth: 'Both',
        headers: {
          'Content-Type': 'application/json',
          'X-API-Key': 'your-api-key',
          'X-Namespace': 'default',
        },
        body: {
          type: 'working',
          sessionId: 'session_123',
          olderThan: 1700000000000,
        },
        response: {
          success: true,
          deleted: 5,
        },
        params: [
          { name: 'type', type: 'string', required: false, description: 'Delete by memory type' },
          { name: 'sessionId', type: 'string', required: false, description: 'Delete by session ID' },
          { name: 'olderThan', type: 'number', required: false, description: 'Delete memories older than timestamp' },
        ],
      },
    ],
  },
  {
    name: 'Namespace Management',
    description: 'Manage memory namespaces for isolation',
    endpoints: [
      {
        method: 'GET',
        path: '/api/v1/namespaces',
        description: 'List all namespaces',
        auth: 'Both',
        headers: {
          'X-API-Key': 'your-api-key',
        },
        response: {
          success: true,
          namespaces: [
            { name: 'default', dimensions: 1536 },
            { name: 'agent-1', dimensions: 1536 },
          ],
          count: 2,
        },
      },
      {
        method: 'POST',
        path: '/api/v1/namespaces',
        description: 'Create a new namespace',
        auth: 'Both',
        headers: {
          'Content-Type': 'application/json',
          'X-API-Key': 'your-api-key',
        },
        body: {
          name: 'my-agent',
          dimensions: 1536,
        },
        response: {
          success: true,
          namespace: { name: 'my-agent', dimensions: 1536 },
        },
        params: [
          { name: 'name', type: 'string', required: true, description: 'Namespace name' },
          { name: 'dimensions', type: 'number', required: true, description: 'Vector dimensions (e.g., 1536 for OpenAI)' },
        ],
      },
      {
        method: 'DELETE',
        path: '/api/v1/namespaces/:name',
        description: 'Delete a namespace and all its memories',
        auth: 'Both',
        headers: {
          'X-API-Key': 'your-api-key',
        },
        response: {
          success: true,
          message: 'Namespace deleted',
        },
        params: [
          { name: 'name', type: 'string', required: true, description: 'Namespace name (URL parameter)' },
        ],
      },
    ],
  },
  {
    name: 'Statistics & Maintenance',
    description: 'Get stats and perform maintenance operations',
    endpoints: [
      {
        method: 'GET',
        path: '/api/v1/stats',
        description: 'Get memory statistics for a namespace',
        auth: 'Both',
        headers: {
          'X-API-Key': 'your-api-key',
          'X-Namespace': 'default',
        },
        response: {
          success: true,
          stats: {
            total: 150,
            byType: { episodic: 50, semantic: 80, working: 20 },
            avgImportance: 0.65,
            oldest: 1699000000000,
            newest: 1700000000000,
          },
        },
      },
      {
        method: 'POST',
        path: '/api/v1/cleanup',
        description: 'Clean up expired working memories',
        auth: 'Both',
        headers: {
          'X-API-Key': 'your-api-key',
          'X-Namespace': 'default',
        },
        response: {
          success: true,
          cleaned: 10,
        },
      },
      {
        method: 'POST',
        path: '/api/v1/decay',
        description: 'Apply importance decay to memories',
        auth: 'Both',
        headers: {
          'Content-Type': 'application/json',
          'X-API-Key': 'your-api-key',
          'X-Namespace': 'default',
        },
        body: {
          rate: 0.1,
          minImportance: 0.1,
        },
        response: {
          success: true,
          affected: 50,
        },
        params: [
          { name: 'rate', type: 'number', required: false, description: 'Decay rate (default: 0.1)' },
          { name: 'minImportance', type: 'number', required: false, description: 'Minimum importance floor' },
        ],
      },
      {
        method: 'DELETE',
        path: '/api/v1/clear',
        description: 'Clear all memories in a namespace',
        auth: 'Both',
        headers: {
          'X-API-Key': 'your-api-key',
          'X-Namespace': 'default',
        },
        response: {
          success: true,
          cleared: 150,
        },
      },
    ],
  },
  {
    name: 'Import/Export',
    description: 'Backup and restore memories',
    endpoints: [
      {
        method: 'POST',
        path: '/api/v1/export',
        description: 'Export all memories from a namespace',
        auth: 'Both',
        headers: {
          'X-API-Key': 'your-api-key',
          'X-Namespace': 'default',
        },
        response: {
          success: true,
          memories: [],
          count: 150,
          exportedAt: 1700000000000,
        },
      },
      {
        method: 'POST',
        path: '/api/v1/import',
        description: 'Import memories into a namespace',
        auth: 'Both',
        headers: {
          'Content-Type': 'application/json',
          'X-API-Key': 'your-api-key',
          'X-Namespace': 'default',
        },
        body: {
          memories: [],
          overwrite: false,
        },
        response: {
          success: true,
          imported: 50,
          skipped: 0,
        },
        params: [
          { name: 'memories', type: 'array', required: true, description: 'Array of memory objects' },
          { name: 'overwrite', type: 'boolean', required: false, description: 'Overwrite existing IDs' },
        ],
      },
    ],
  },
];

const methodColors: Record<string, string> = {
  GET: 'bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400',
  POST: 'bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-400',
  PUT: 'bg-yellow-100 text-yellow-700 dark:bg-yellow-900/30 dark:text-yellow-400',
  PATCH: 'bg-orange-100 text-orange-700 dark:bg-orange-900/30 dark:text-orange-400',
  DELETE: 'bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400',
};

function CodeBlock({ code, language = 'json' }: { code: string; language?: string }) {
  const [copied, setCopied] = useState(false);

  const handleCopy = () => {
    navigator.clipboard.writeText(code);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <div className="relative group">
      <pre className="bg-gray-900 text-gray-100 p-4 rounded-lg overflow-x-auto text-sm">
        <code className={`language-${language}`}>{code}</code>
      </pre>
      <button
        onClick={handleCopy}
        className="absolute top-2 right-2 p-2 bg-gray-700 hover:bg-gray-600 rounded opacity-0 group-hover:opacity-100 transition-opacity"
      >
        {copied ? <Check className="w-4 h-4 text-green-400" /> : <Copy className="w-4 h-4 text-gray-300" />}
      </button>
    </div>
  );
}

function EndpointCard({ endpoint }: { endpoint: Endpoint }) {
  const [isOpen, setIsOpen] = useState(false);

  const curlCommand = `curl -X ${endpoint.method} "${API_URL}${endpoint.path}" \\
${endpoint.headers ? Object.entries(endpoint.headers).map(([k, v]) => `  -H "${k}: ${v}"`).join(' \\\n') : ''}${endpoint.body ? ` \\
  -d '${JSON.stringify(endpoint.body, null, 2)}'` : ''}`;

  return (
    <div className="border border-gray-200 dark:border-gray-700 rounded-lg overflow-hidden">
      <button
        onClick={() => setIsOpen(!isOpen)}
        className="w-full px-4 py-3 flex items-center gap-3 hover:bg-gray-50 dark:hover:bg-gray-800/50 transition-colors"
      >
        {isOpen ? (
          <ChevronDown className="w-4 h-4 text-gray-500" />
        ) : (
          <ChevronRight className="w-4 h-4 text-gray-500" />
        )}
        <span className={`px-2 py-1 rounded text-xs font-bold ${methodColors[endpoint.method]}`}>
          {endpoint.method}
        </span>
        <code className="text-sm font-mono text-gray-700 dark:text-gray-300">{endpoint.path}</code>
        <span className="text-sm text-gray-500 dark:text-gray-400 ml-auto">{endpoint.description}</span>
      </button>

      {isOpen && (
        <div className="px-4 pb-4 space-y-4 border-t border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-800/30">
          <div className="pt-4">
            <h4 className="text-sm font-semibold text-gray-700 dark:text-gray-300 mb-2">Authentication</h4>
            <span className="px-2 py-1 bg-indigo-100 dark:bg-indigo-900/30 text-indigo-700 dark:text-indigo-400 rounded text-xs">
              {endpoint.auth}
            </span>
          </div>

          {endpoint.params && endpoint.params.length > 0 && (
            <div>
              <h4 className="text-sm font-semibold text-gray-700 dark:text-gray-300 mb-2">Parameters</h4>
              <div className="overflow-x-auto">
                <table className="w-full text-sm">
                  <thead>
                    <tr className="text-left text-gray-500 dark:text-gray-400">
                      <th className="pb-2 pr-4">Name</th>
                      <th className="pb-2 pr-4">Type</th>
                      <th className="pb-2 pr-4">Required</th>
                      <th className="pb-2">Description</th>
                    </tr>
                  </thead>
                  <tbody>
                    {endpoint.params.map((param) => (
                      <tr key={param.name} className="border-t border-gray-200 dark:border-gray-700">
                        <td className="py-2 pr-4 font-mono text-indigo-600 dark:text-indigo-400">{param.name}</td>
                        <td className="py-2 pr-4 text-gray-600 dark:text-gray-400">{param.type}</td>
                        <td className="py-2 pr-4">
                          {param.required ? (
                            <span className="text-red-600 dark:text-red-400">Yes</span>
                          ) : (
                            <span className="text-gray-400">No</span>
                          )}
                        </td>
                        <td className="py-2 text-gray-600 dark:text-gray-400">{param.description}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </div>
          )}

          {endpoint.body && (
            <div>
              <h4 className="text-sm font-semibold text-gray-700 dark:text-gray-300 mb-2">Request Body</h4>
              <CodeBlock code={JSON.stringify(endpoint.body, null, 2)} />
            </div>
          )}

          {endpoint.response && (
            <div>
              <h4 className="text-sm font-semibold text-gray-700 dark:text-gray-300 mb-2">Response</h4>
              <CodeBlock code={JSON.stringify(endpoint.response, null, 2)} />
            </div>
          )}

          <div>
            <h4 className="text-sm font-semibold text-gray-700 dark:text-gray-300 mb-2">cURL Example</h4>
            <CodeBlock code={curlCommand} language="bash" />
          </div>
        </div>
      )}
    </div>
  );
}

export function Documentation() {
  return (
    <div className="space-y-8 pb-16">
      <div>
        <h1 className="text-2xl font-bold text-gray-900 dark:text-white">API Documentation</h1>
        <p className="mt-2 text-gray-600 dark:text-gray-400">
          Complete reference for the minimemory REST API - a persistent memory system for AI agents.
        </p>
      </div>

      {/* Overview */}
      <div className="bg-gradient-to-r from-indigo-500 to-purple-600 rounded-xl p-6 text-white">
        <h2 className="text-lg font-semibold mb-3">Overview</h2>
        <p className="text-indigo-100 mb-4">
          MiniMemory is a vector database designed for AI agents. It stores memories with embeddings
          for semantic search, supports keyword search (BM25), and hybrid search combining both methods.
        </p>
        <div className="grid md:grid-cols-3 gap-4 text-sm">
          <div className="bg-white/10 rounded-lg p-3">
            <div className="font-semibold mb-1">🧠 Memory Types</div>
            <div className="text-indigo-100">Episodic, Semantic, Working</div>
          </div>
          <div className="bg-white/10 rounded-lg p-3">
            <div className="font-semibold mb-1">🔍 Search Modes</div>
            <div className="text-indigo-100">Vector, Keyword, Hybrid</div>
          </div>
          <div className="bg-white/10 rounded-lg p-3">
            <div className="font-semibold mb-1">🏢 Multi-tenant</div>
            <div className="text-indigo-100">Namespace isolation</div>
          </div>
        </div>
      </div>

      {/* Memory Types */}
      <div className="bg-white dark:bg-gray-800 rounded-xl p-6 shadow-sm">
        <h2 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">Memory Types</h2>
        <div className="grid md:grid-cols-3 gap-4">
          <div className="p-4 border border-indigo-200 dark:border-indigo-800 rounded-lg bg-indigo-50 dark:bg-indigo-900/20">
            <h3 className="font-semibold text-indigo-700 dark:text-indigo-400 mb-2">Episodic</h3>
            <p className="text-sm text-gray-600 dark:text-gray-400">
              Events and experiences with temporal context. Example: "User asked about pricing at 10:30 AM"
            </p>
          </div>
          <div className="p-4 border border-green-200 dark:border-green-800 rounded-lg bg-green-50 dark:bg-green-900/20">
            <h3 className="font-semibold text-green-700 dark:text-green-400 mb-2">Semantic</h3>
            <p className="text-sm text-gray-600 dark:text-gray-400">
              Facts and knowledge. Example: "User prefers short answers", "Company uses React"
            </p>
          </div>
          <div className="p-4 border border-amber-200 dark:border-amber-800 rounded-lg bg-amber-50 dark:bg-amber-900/20">
            <h3 className="font-semibold text-amber-700 dark:text-amber-400 mb-2">Working</h3>
            <p className="text-sm text-gray-600 dark:text-gray-400">
              Temporary task-related state with TTL. Example: "Currently searching for product XYZ"
            </p>
          </div>
        </div>
      </div>

      {/* Base URL */}
      <div className="bg-white dark:bg-gray-800 rounded-xl p-6 shadow-sm">
        <h2 className="text-lg font-semibold text-gray-900 dark:text-white mb-2">Base URL</h2>
        <div className="flex items-center gap-2">
          <code className="flex-1 bg-gray-100 dark:bg-gray-700 px-4 py-2 rounded-lg font-mono text-sm text-gray-700 dark:text-gray-300">
            {API_URL}
          </code>
          <a
            href={API_URL}
            target="_blank"
            rel="noopener noreferrer"
            className="p-2 text-gray-500 hover:text-gray-700 dark:hover:text-gray-300"
          >
            <ExternalLink className="w-5 h-5" />
          </a>
        </div>
      </div>

      {/* Authentication */}
      <div className="bg-white dark:bg-gray-800 rounded-xl p-6 shadow-sm">
        <h2 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">Authentication</h2>
        <div className="space-y-4 text-sm text-gray-600 dark:text-gray-400">
          <p>The API supports two authentication methods:</p>
          <div className="grid md:grid-cols-2 gap-4">
            <div className="p-4 bg-gray-50 dark:bg-gray-700/50 rounded-lg">
              <h3 className="font-semibold text-gray-900 dark:text-white mb-2">API Key</h3>
              <p className="mb-2">Pass your API key in the <code className="bg-gray-200 dark:bg-gray-600 px-1 rounded">X-API-Key</code> header.</p>
              <CodeBlock code={`X-API-Key: mm_dev_key_12345`} />
            </div>
            <div className="p-4 bg-gray-50 dark:bg-gray-700/50 rounded-lg">
              <h3 className="font-semibold text-gray-900 dark:text-white mb-2">JWT Token</h3>
              <p className="mb-2">Pass your JWT token in the <code className="bg-gray-200 dark:bg-gray-600 px-1 rounded">Authorization</code> header.</p>
              <CodeBlock code={`Authorization: Bearer eyJhbGc...`} />
            </div>
          </div>
        </div>
      </div>

      {/* Namespaces Header */}
      <div className="bg-white dark:bg-gray-800 rounded-xl p-6 shadow-sm">
        <h2 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">Namespace Header</h2>
        <p className="text-sm text-gray-600 dark:text-gray-400 mb-4">
          Most memory operations require specifying a namespace via the <code className="bg-gray-200 dark:bg-gray-600 px-1 rounded">X-Namespace</code> header.
          If not provided, the <code className="bg-gray-200 dark:bg-gray-600 px-1 rounded">default</code> namespace is used.
        </p>
        <CodeBlock code={`X-Namespace: my-agent-namespace`} />
      </div>

      {/* Endpoint Categories */}
      {categories.map((category) => (
        <div key={category.name} className="bg-white dark:bg-gray-800 rounded-xl p-6 shadow-sm">
          <h2 className="text-lg font-semibold text-gray-900 dark:text-white mb-2">{category.name}</h2>
          <p className="text-sm text-gray-600 dark:text-gray-400 mb-4">{category.description}</p>
          <div className="space-y-2">
            {category.endpoints.map((endpoint, idx) => (
              <EndpointCard key={idx} endpoint={endpoint} />
            ))}
          </div>
        </div>
      ))}
    </div>
  );
}
