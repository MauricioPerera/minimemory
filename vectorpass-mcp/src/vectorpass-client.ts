/**
 * VectorPass API Client
 * Internal client to call the VectorPass API using user's API key
 */

import { Env, VectorPassResponse } from './types';

export class VectorPassClient {
  private apiKey: string;
  private baseUrl: string;

  constructor(apiKey: string, env: Env) {
    this.apiKey = apiKey;
    this.baseUrl = env.VECTORPASS_API_URL;
  }

  /**
   * Index a document
   */
  async index(params: {
    id: string;
    text: string;
    metadata?: Record<string, any>;
    db?: string;
  }): Promise<VectorPassResponse> {
    return this.request('POST', '/v1/index', params);
  }

  /**
   * Batch index documents
   */
  async batchIndex(params: {
    items: Array<{ id: string; text: string; metadata?: Record<string, any> }>;
    db?: string;
  }): Promise<VectorPassResponse> {
    return this.request('POST', '/v1/batch', params);
  }

  /**
   * Semantic search
   */
  async search(params: {
    query: string;
    k?: number;
    filter?: Record<string, any>;
    db?: string;
  }): Promise<VectorPassResponse> {
    return this.request('POST', '/v1/search', params);
  }

  /**
   * Keyword search (BM25)
   */
  async keywordSearch(params: {
    query: string;
    k?: number;
    db?: string;
  }): Promise<VectorPassResponse> {
    return this.request('POST', '/v1/keyword', params);
  }

  /**
   * Delete a document
   */
  async delete(id: string, db?: string): Promise<VectorPassResponse> {
    const url = db ? `/v1/vectors/${id}?db=${db}` : `/v1/vectors/${id}`;
    return this.request('DELETE', url);
  }

  /**
   * List databases
   */
  async listDatabases(): Promise<VectorPassResponse> {
    return this.request('GET', '/v1/databases');
  }

  /**
   * Create database
   */
  async createDatabase(name: string): Promise<VectorPassResponse> {
    return this.request('POST', '/v1/databases', { name });
  }

  /**
   * Delete database
   */
  async deleteDatabase(name: string): Promise<VectorPassResponse> {
    return this.request('DELETE', `/v1/databases/${name}`);
  }

  /**
   * Get stats
   */
  async getStats(db?: string): Promise<VectorPassResponse> {
    const url = db ? `/v1/stats?db=${db}` : '/v1/stats';
    return this.request('GET', url);
  }

  /**
   * Make API request
   */
  private async request(
    method: string,
    path: string,
    body?: any
  ): Promise<VectorPassResponse> {
    const url = `${this.baseUrl}${path}`;

    const options: RequestInit = {
      method,
      headers: {
        'X-API-Key': this.apiKey,
        'Content-Type': 'application/json',
      },
    };

    if (body && method !== 'GET') {
      options.body = JSON.stringify(body);
    }

    try {
      const response = await fetch(url, options);
      const data: any = await response.json();

      if (!response.ok) {
        return {
          success: false,
          error: data?.error || `HTTP ${response.status}`,
        };
      }

      return data as VectorPassResponse;
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : 'Unknown error',
      };
    }
  }
}
