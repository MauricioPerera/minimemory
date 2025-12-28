/**
 * Ejemplo: minimemory + EmbeddingGemma en Cloudflare Workers
 *
 * Este wrapper automatiza:
 * - Generacion de embeddings con Gemma
 * - Truncado Matryoshka (768d -> 256d)
 * - Normalizacion L2
 * - Persistencia en KV
 *
 * Uso:
 *   wrangler dev
 *   curl -X POST http://localhost:8787/index -d '{"id":"doc1","text":"Hello world"}'
 *   curl -X POST http://localhost:8787/search -d '{"query":"greeting","k":5}'
 */

import init, { WasmVectorDB } from './pkg/minimemory.js';

// Configuracion
const CONFIG = {
    dimensions: 256,        // Matryoshka truncated (768 -> 256)
    distance: "cosine",
    index: "hnsw",
    quantization: "int8",   // 4x compression
    model: "@cf/google/embeddinggemma-300m"
};

/**
 * Wrapper que encapsula WasmVectorDB + EmbeddingGemma
 */
class GemmaVectorDB {
    private db: WasmVectorDB;
    private ai: any;
    private kv: KVNamespace;
    private kvKey: string;

    private constructor(db: WasmVectorDB, ai: any, kv: KVNamespace, kvKey: string) {
        this.db = db;
        this.ai = ai;
        this.kv = kv;
        this.kvKey = kvKey;
    }

    /**
     * Crea o restaura una instancia de GemmaVectorDB
     */
    static async create(env: Env, kvKey: string = "vectordb"): Promise<GemmaVectorDB> {
        await init();

        const db = WasmVectorDB.new_int8(
            CONFIG.dimensions,
            CONFIG.distance,
            CONFIG.index
        );

        // Restaurar estado si existe
        const saved = await env.KV.get(kvKey);
        if (saved) {
            db.import_json(saved);
        }

        return new GemmaVectorDB(db, env.AI, env.KV, kvKey);
    }

    /**
     * Genera embedding con Gemma (768d)
     */
    private async embed(text: string): Promise<Float32Array> {
        const result = await this.ai.run(CONFIG.model, { text });
        return new Float32Array(result.data);
    }

    /**
     * Genera embeddings en batch (hasta 100 textos)
     */
    private async embedBatch(texts: string[]): Promise<Float32Array[]> {
        const result = await this.ai.run(CONFIG.model, { text: texts });

        // El resultado puede ser un array de arrays o un array plano
        if (Array.isArray(result.data[0])) {
            return result.data.map((arr: number[]) => new Float32Array(arr));
        }

        // Si es plano, dividir por dimensiones
        const embeddings: Float32Array[] = [];
        const fullDims = 768;
        for (let i = 0; i < texts.length; i++) {
            const start = i * fullDims;
            embeddings.push(new Float32Array(result.data.slice(start, start + fullDims)));
        }
        return embeddings;
    }

    /**
     * Indexa un documento por texto
     */
    async index(id: string, text: string, metadata?: Record<string, any>): Promise<void> {
        const embedding = await this.embed(text);

        if (metadata) {
            // Guardar snippet del texto en metadata
            const fullMeta = { ...metadata, _snippet: text.slice(0, 500) };
            this.db.insert_auto_with_metadata(id, embedding, JSON.stringify(fullMeta));
        } else {
            this.db.insert_auto(id, embedding);
        }
    }

    /**
     * Indexa multiples documentos en batch
     */
    async indexBatch(items: Array<{id: string, text: string, metadata?: Record<string, any>}>): Promise<number> {
        const texts = items.map(i => i.text);
        const embeddings = await this.embedBatch(texts);

        for (let i = 0; i < items.length; i++) {
            const item = items[i];
            if (item.metadata) {
                const fullMeta = { ...item.metadata, _snippet: item.text.slice(0, 500) };
                this.db.insert_auto_with_metadata(item.id, embeddings[i], JSON.stringify(fullMeta));
            } else {
                this.db.insert_auto(item.id, embeddings[i]);
            }
        }

        return items.length;
    }

    /**
     * Busca por texto (semantic search)
     */
    async search(query: string, k: number = 10): Promise<SearchResult[]> {
        const embedding = await this.embed(query);
        const resultsJson = this.db.search_auto(embedding, k);
        return JSON.parse(resultsJson);
    }

    /**
     * Busca por palabras clave (BM25)
     */
    keywordSearch(query: string, k: number = 10): KeywordResult[] {
        const resultsJson = this.db.keyword_search(query, k);
        return JSON.parse(resultsJson);
    }

    /**
     * Actualiza un documento
     */
    async update(id: string, text: string, metadata?: Record<string, any>): Promise<void> {
        const embedding = await this.embed(text);

        if (metadata) {
            const fullMeta = { ...metadata, _snippet: text.slice(0, 500) };
            this.db.update_auto_with_metadata(id, embedding, JSON.stringify(fullMeta));
        } else {
            this.db.update_auto(id, embedding);
        }
    }

    /**
     * Elimina un documento
     */
    delete(id: string): boolean {
        return this.db.delete(id);
    }

    /**
     * Verifica si existe
     */
    contains(id: string): boolean {
        return this.db.contains(id);
    }

    /**
     * Obtiene estadisticas
     */
    stats(): Stats {
        return {
            count: this.db.len(),
            dimensions: this.db.dimensions(),
            isEmpty: this.db.is_empty(),
            model: CONFIG.model,
            quantization: CONFIG.quantization
        };
    }

    /**
     * Persiste a KV
     */
    async save(): Promise<void> {
        await this.kv.put(this.kvKey, this.db.export_json());
    }

    /**
     * Limpia la base de datos
     */
    clear(): void {
        this.db.clear();
    }
}

// Tipos
interface SearchResult {
    id: string;
    distance: number;
    metadata?: Record<string, any>;
}

interface KeywordResult {
    id: string;
    score: number;
}

interface Stats {
    count: number;
    dimensions: number;
    isEmpty: boolean;
    model: string;
    quantization: string;
}

interface Env {
    AI: any;
    KV: KVNamespace;
}

// ============================================================================
// Cloudflare Worker Handler
// ============================================================================

export default {
    async fetch(request: Request, env: Env): Promise<Response> {
        const url = new URL(request.url);

        try {
            const db = await GemmaVectorDB.create(env);

            // POST /index - Indexar documento
            if (request.method === "POST" && url.pathname === "/index") {
                const { id, text, metadata } = await request.json() as any;

                if (!id || !text) {
                    return Response.json({ error: "id and text required" }, { status: 400 });
                }

                await db.index(id, text, metadata);
                await db.save();

                return Response.json({ success: true, id, stats: db.stats() });
            }

            // POST /batch - Indexar multiples
            if (request.method === "POST" && url.pathname === "/batch") {
                const { items } = await request.json() as any;

                if (!items || !Array.isArray(items)) {
                    return Response.json({ error: "items array required" }, { status: 400 });
                }

                const count = await db.indexBatch(items);
                await db.save();

                return Response.json({ indexed: count, stats: db.stats() });
            }

            // POST /search - Busqueda semantica
            if (request.method === "POST" && url.pathname === "/search") {
                const { query, k = 10 } = await request.json() as any;

                if (!query) {
                    return Response.json({ error: "query required" }, { status: 400 });
                }

                const results = await db.search(query, k);
                return Response.json({ results, query, stats: db.stats() });
            }

            // POST /keyword - Busqueda por palabras clave
            if (request.method === "POST" && url.pathname === "/keyword") {
                const { query, k = 10 } = await request.json() as any;

                if (!query) {
                    return Response.json({ error: "query required" }, { status: 400 });
                }

                const results = db.keywordSearch(query, k);
                return Response.json({ results, query });
            }

            // DELETE /vectors/:id
            if (request.method === "DELETE" && url.pathname.startsWith("/vectors/")) {
                const id = url.pathname.split("/")[2];
                const deleted = db.delete(id);
                await db.save();
                return Response.json({ deleted, id });
            }

            // GET /stats
            if (url.pathname === "/stats") {
                return Response.json(db.stats());
            }

            // GET / - Info
            return Response.json({
                name: "minimemory + EmbeddingGemma",
                endpoints: {
                    "POST /index": "Index document {id, text, metadata?}",
                    "POST /batch": "Index multiple {items: [{id, text, metadata?}]}",
                    "POST /search": "Semantic search {query, k?}",
                    "POST /keyword": "Keyword search {query, k?}",
                    "DELETE /vectors/:id": "Delete document",
                    "GET /stats": "Get statistics"
                },
                config: CONFIG,
                stats: db.stats()
            });

        } catch (error: any) {
            return Response.json({ error: error.message }, { status: 500 });
        }
    }
};
