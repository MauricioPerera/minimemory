/**
 * MCP Tool Definitions and Handlers
 */

import { VectorPassClient } from '../vectorpass-client';
import { MCPTool } from '../types';

// Tool definitions
export const TOOLS: MCPTool[] = [
  {
    name: 'vectorpass_index',
    description: 'Index a document in VectorPass vector database. The text will be automatically chunked if longer than 1500 characters.',
    inputSchema: {
      type: 'object',
      properties: {
        id: {
          type: 'string',
          description: 'Unique document ID',
        },
        text: {
          type: 'string',
          description: 'Text content to index',
        },
        metadata: {
          type: 'object',
          description: 'Optional metadata to store with the document',
        },
        database: {
          type: 'string',
          description: 'Database name (default: "default")',
        },
      },
      required: ['id', 'text'],
    },
  },
  {
    name: 'vectorpass_batch_index',
    description: 'Index multiple documents in a single request. More efficient than indexing one by one.',
    inputSchema: {
      type: 'object',
      properties: {
        items: {
          type: 'array',
          description: 'Array of documents to index',
          items: {
            type: 'object',
            properties: {
              id: { type: 'string' },
              text: { type: 'string' },
              metadata: { type: 'object' },
            },
            required: ['id', 'text'],
          },
        },
        database: {
          type: 'string',
          description: 'Database name (default: "default")',
        },
      },
      required: ['items'],
    },
  },
  {
    name: 'vectorpass_search',
    description: 'Semantic search in VectorPass. Finds documents similar in meaning to the query.',
    inputSchema: {
      type: 'object',
      properties: {
        query: {
          type: 'string',
          description: 'Search query text',
        },
        k: {
          type: 'number',
          description: 'Number of results to return (default: 10)',
        },
        database: {
          type: 'string',
          description: 'Database name (default: "default")',
        },
        filter: {
          type: 'object',
          description: 'Metadata filter (e.g., {"category": "news"})',
        },
      },
      required: ['query'],
    },
  },
  {
    name: 'vectorpass_keyword_search',
    description: 'Keyword search using BM25 algorithm. Finds documents containing the exact keywords.',
    inputSchema: {
      type: 'object',
      properties: {
        query: {
          type: 'string',
          description: 'Keywords to search for',
        },
        k: {
          type: 'number',
          description: 'Number of results to return (default: 10)',
        },
        database: {
          type: 'string',
          description: 'Database name (default: "default")',
        },
      },
      required: ['query'],
    },
  },
  {
    name: 'vectorpass_delete',
    description: 'Delete a document from VectorPass by its ID.',
    inputSchema: {
      type: 'object',
      properties: {
        id: {
          type: 'string',
          description: 'Document ID to delete',
        },
        database: {
          type: 'string',
          description: 'Database name (default: "default")',
        },
      },
      required: ['id'],
    },
  },
  {
    name: 'vectorpass_list_databases',
    description: 'List all databases for the authenticated user.',
    inputSchema: {
      type: 'object',
      properties: {},
    },
  },
  {
    name: 'vectorpass_create_database',
    description: 'Create a new vector database.',
    inputSchema: {
      type: 'object',
      properties: {
        name: {
          type: 'string',
          description: 'Database name (alphanumeric and underscores only)',
        },
      },
      required: ['name'],
    },
  },
  {
    name: 'vectorpass_stats',
    description: 'Get usage statistics for your VectorPass account.',
    inputSchema: {
      type: 'object',
      properties: {
        database: {
          type: 'string',
          description: 'Database name (default: all databases)',
        },
      },
    },
  },
];

/**
 * Handle tool execution
 */
export async function executeTool(
  toolName: string,
  args: Record<string, any>,
  client: VectorPassClient
): Promise<{ content: Array<{ type: string; text: string }> }> {
  try {
    let result: any;

    switch (toolName) {
      case 'vectorpass_index':
        result = await client.index({
          id: args.id,
          text: args.text,
          metadata: args.metadata,
          db: args.database,
        });
        break;

      case 'vectorpass_batch_index':
        result = await client.batchIndex({
          items: args.items,
          db: args.database,
        });
        break;

      case 'vectorpass_search':
        result = await client.search({
          query: args.query,
          k: args.k,
          filter: args.filter,
          db: args.database,
        });
        break;

      case 'vectorpass_keyword_search':
        result = await client.keywordSearch({
          query: args.query,
          k: args.k,
          db: args.database,
        });
        break;

      case 'vectorpass_delete':
        result = await client.delete(args.id, args.database);
        break;

      case 'vectorpass_list_databases':
        result = await client.listDatabases();
        break;

      case 'vectorpass_create_database':
        result = await client.createDatabase(args.name);
        break;

      case 'vectorpass_stats':
        result = await client.getStats(args.database);
        break;

      default:
        return {
          content: [
            {
              type: 'text',
              text: JSON.stringify({ error: `Unknown tool: ${toolName}` }),
            },
          ],
        };
    }

    return {
      content: [
        {
          type: 'text',
          text: JSON.stringify(result, null, 2),
        },
      ],
    };
  } catch (error) {
    return {
      content: [
        {
          type: 'text',
          text: JSON.stringify({
            error: error instanceof Error ? error.message : 'Tool execution failed',
          }),
        },
      ],
    };
  }
}
