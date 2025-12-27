/**
 * Knowledge Bank API Routes
 *
 * Endpoints for RAG document ingestion and knowledge management
 */
import { Hono } from 'hono';
import { KnowledgeService } from '../services/KnowledgeService.js';
import { EmbeddingService } from '../services/EmbeddingService.js';
import { createAuditLogger } from '../services/AuditService.js';
import { createWebhookTrigger } from '../services/WebhookService.js';
const knowledgeRoutes = new Hono();
/**
 * Helper to get namespace from header
 */
function getNamespace(c) {
    return c.req.header('X-Namespace') || 'default';
}
/**
 * Helper to get audit context
 */
function getAuditContext(c) {
    return {
        namespace: getNamespace(c),
        userId: c.req.header('X-User-Id'),
        tenantId: c.req.header('X-Tenant-Id'),
        apiKey: c.req.header('X-API-Key'),
        ipAddress: c.req.header('CF-Connecting-IP') || c.req.header('X-Forwarded-For'),
        userAgent: c.req.header('User-Agent'),
        requestId: c.req.header('X-Request-Id'),
    };
}
// ============ Document Ingestion ============
/**
 * POST /knowledge/ingest
 * Ingest a document into the knowledge bank
 */
knowledgeRoutes.post('/ingest', async (c) => {
    const startTime = Date.now();
    const namespace = getNamespace(c);
    try {
        const body = await c.req.json();
        const { content, name, type = 'document', url, mimeType, metadata = {}, chunking = {}, generateEmbeddings = true, } = body;
        // Validate required fields
        if (!content || typeof content !== 'string') {
            return c.json({ error: 'content is required and must be a string' }, 400);
        }
        if (!name || typeof name !== 'string') {
            return c.json({ error: 'name is required and must be a string' }, 400);
        }
        const knowledgeService = new KnowledgeService(c.env?.DB || null);
        if (!knowledgeService.isAvailable()) {
            return c.json({ error: 'Knowledge service requires D1 database' }, 503);
        }
        // Chunk the content
        const chunks = knowledgeService.chunkText(content, chunking);
        if (chunks.length === 0) {
            return c.json({ error: 'No valid chunks could be created from content' }, 400);
        }
        // Create the source record
        const source = await knowledgeService.createSource(namespace, {
            name,
            type,
            url,
            mimeType,
            size: content.length,
            chunkCount: chunks.length,
            namespace,
            metadata,
        });
        // Generate embeddings if requested and AI is available
        let embeddingsGenerated = false;
        let chunkEmbeddings = [];
        if (generateEmbeddings && c.env?.AI) {
            const embeddingService = new EmbeddingService(c.env.AI);
            const texts = chunks.map(chunk => chunk.text);
            try {
                const result = await embeddingService.embedBatch(texts, { dimensions: 768 });
                chunkEmbeddings = result.embeddings;
                embeddingsGenerated = true;
            }
            catch (error) {
                console.error('Failed to generate embeddings:', error);
                // Continue without embeddings
            }
        }
        // Store chunks as knowledge memories
        const db = c.env?.DB;
        if (db) {
            for (let i = 0; i < chunks.length; i++) {
                const chunk = chunks[i];
                const embedding = chunkEmbeddings[i] || [];
                const memoryId = `mem_${source.id}_${i}`;
                await db
                    .prepare(`
						INSERT INTO memories
						(id, namespace, type, content, embedding, importance, metadata, created_at, updated_at, access_count)
						VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
					`)
                    .bind(memoryId, namespace, 'knowledge', chunk.text, JSON.stringify(embedding), 0.5, // Default importance for knowledge
                JSON.stringify({
                    sourceId: source.id,
                    sourceName: name,
                    sourceType: type,
                    sourceUrl: url,
                    chunkIndex: i,
                    totalChunks: chunks.length,
                    startOffset: chunk.startOffset,
                    endOffset: chunk.endOffset,
                    ...metadata,
                }), Date.now(), Date.now(), 0)
                    .run();
            }
        }
        // Audit log
        if (c.env?.DB) {
            const logger = createAuditLogger(c.env.DB, getAuditContext(c));
            await logger.logMemory('create', source.id, {
                action: 'ingest',
                sourceName: name,
                sourceType: type,
                chunksCreated: chunks.length,
                embeddingsGenerated,
                contentLength: content.length,
            });
            // Trigger webhook
            const tenantId = c.req.header('X-Tenant-Id');
            const webhookTrigger = createWebhookTrigger(c.env.DB, c.executionCtx);
            webhookTrigger(namespace, 'knowledge.ingested', {
                sourceId: source.id,
                sourceName: name,
                sourceType: type,
                chunksCreated: chunks.length,
                totalCharacters: content.length,
            }, tenantId);
        }
        const result = {
            sourceId: source.id,
            sourceName: name,
            chunksCreated: chunks.length,
            embeddingsGenerated,
            totalCharacters: content.length,
            averageChunkSize: Math.round(content.length / chunks.length),
        };
        return c.json({
            success: true,
            ...result,
            durationMs: Date.now() - startTime,
        });
    }
    catch (error) {
        return c.json({
            error: error instanceof Error ? error.message : 'Unknown error',
        }, 500);
    }
});
// ============ Source Management ============
/**
 * GET /knowledge/sources
 * List all knowledge sources in the namespace
 */
knowledgeRoutes.get('/sources', async (c) => {
    const namespace = getNamespace(c);
    const type = c.req.query('type');
    const limit = parseInt(c.req.query('limit') || '100');
    const offset = parseInt(c.req.query('offset') || '0');
    const knowledgeService = new KnowledgeService(c.env?.DB || null);
    if (!knowledgeService.isAvailable()) {
        return c.json({ error: 'Knowledge service requires D1 database' }, 503);
    }
    const { sources, total } = await knowledgeService.listSources(namespace, {
        type,
        limit,
        offset,
    });
    return c.json({
        success: true,
        sources,
        total,
        hasMore: offset + sources.length < total,
    });
});
/**
 * GET /knowledge/sources/:id
 * Get a specific knowledge source
 */
knowledgeRoutes.get('/sources/:id', async (c) => {
    const id = c.req.param('id');
    const knowledgeService = new KnowledgeService(c.env?.DB || null);
    if (!knowledgeService.isAvailable()) {
        return c.json({ error: 'Knowledge service requires D1 database' }, 503);
    }
    const source = await knowledgeService.getSource(id);
    if (!source) {
        return c.json({ error: 'Source not found' }, 404);
    }
    return c.json({
        success: true,
        source,
    });
});
/**
 * DELETE /knowledge/sources/:id
 * Delete a knowledge source and all its chunks
 */
knowledgeRoutes.delete('/sources/:id', async (c) => {
    const id = c.req.param('id');
    const knowledgeService = new KnowledgeService(c.env?.DB || null);
    if (!knowledgeService.isAvailable()) {
        return c.json({ error: 'Knowledge service requires D1 database' }, 503);
    }
    const source = await knowledgeService.getSource(id);
    if (!source) {
        return c.json({ error: 'Source not found' }, 404);
    }
    const deleted = await knowledgeService.deleteSource(id);
    // Audit log
    if (c.env?.DB) {
        const logger = createAuditLogger(c.env.DB, getAuditContext(c));
        await logger.logMemory('delete', id, {
            action: 'deleteSource',
            sourceName: source.name,
            chunksDeleted: source.chunkCount,
        });
        // Trigger webhook if deleted successfully
        if (deleted) {
            const namespace = getNamespace(c);
            const tenantId = c.req.header('X-Tenant-Id');
            const webhookTrigger = createWebhookTrigger(c.env.DB, c.executionCtx);
            webhookTrigger(namespace, 'knowledge.deleted', {
                sourceId: id,
                sourceName: source.name,
                chunksDeleted: source.chunkCount,
            }, tenantId);
        }
    }
    return c.json({
        success: deleted,
        message: deleted ? `Source "${source.name}" and ${source.chunkCount} chunks deleted` : 'Delete failed',
    });
});
// ============ Knowledge Stats ============
/**
 * GET /knowledge/stats
 * Get knowledge bank statistics
 */
knowledgeRoutes.get('/stats', async (c) => {
    const namespace = getNamespace(c);
    const knowledgeService = new KnowledgeService(c.env?.DB || null);
    if (!knowledgeService.isAvailable()) {
        return c.json({ error: 'Knowledge service requires D1 database' }, 503);
    }
    const stats = await knowledgeService.getStats(namespace);
    return c.json({
        success: true,
        namespace,
        stats,
    });
});
// ============ Chunking Preview ============
/**
 * POST /knowledge/chunk-preview
 * Preview how content will be chunked (without storing)
 */
knowledgeRoutes.post('/chunk-preview', async (c) => {
    try {
        const body = await c.req.json();
        const { content, chunking = {} } = body;
        if (!content || typeof content !== 'string') {
            return c.json({ error: 'content is required and must be a string' }, 400);
        }
        const knowledgeService = new KnowledgeService(null);
        const chunks = knowledgeService.chunkText(content, chunking);
        return c.json({
            success: true,
            totalChunks: chunks.length,
            totalCharacters: content.length,
            averageChunkSize: chunks.length > 0 ? Math.round(content.length / chunks.length) : 0,
            chunks: chunks.map((chunk, i) => ({
                index: i,
                length: chunk.text.length,
                preview: chunk.text.substring(0, 100) + (chunk.text.length > 100 ? '...' : ''),
                startOffset: chunk.startOffset,
                endOffset: chunk.endOffset,
            })),
        });
    }
    catch (error) {
        return c.json({
            error: error instanceof Error ? error.message : 'Unknown error',
        }, 500);
    }
});
// ============ Get Chunks by Source ============
/**
 * GET /knowledge/sources/:id/chunks
 * Get all chunks for a specific source
 */
knowledgeRoutes.get('/sources/:id/chunks', async (c) => {
    const id = c.req.param('id');
    const namespace = getNamespace(c);
    const limit = parseInt(c.req.query('limit') || '100');
    const offset = parseInt(c.req.query('offset') || '0');
    if (!c.env?.DB) {
        return c.json({ error: 'D1 database required' }, 503);
    }
    const knowledgeService = new KnowledgeService(c.env.DB);
    const source = await knowledgeService.getSource(id);
    if (!source) {
        return c.json({ error: 'Source not found' }, 404);
    }
    // Query chunks from memories table
    const results = await c.env.DB
        .prepare(`
			SELECT id, content, metadata, created_at
			FROM memories
			WHERE namespace = ? AND type = 'knowledge'
			AND metadata LIKE ?
			ORDER BY id
			LIMIT ? OFFSET ?
		`)
        .bind(namespace, `%"sourceId":"${id}"%`, limit, offset)
        .all();
    const chunks = (results.results || []).map(row => {
        const metadata = JSON.parse(row.metadata || '{}');
        return {
            id: row.id,
            content: row.content,
            chunkIndex: metadata.chunkIndex,
            startOffset: metadata.startOffset,
            endOffset: metadata.endOffset,
            createdAt: row.created_at,
        };
    });
    return c.json({
        success: true,
        source: {
            id: source.id,
            name: source.name,
            type: source.type,
        },
        chunks,
        total: source.chunkCount,
        hasMore: offset + chunks.length < source.chunkCount,
    });
});
export default knowledgeRoutes;
//# sourceMappingURL=knowledge.js.map