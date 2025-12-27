/**
 * BM25 (Best Matching 25) keyword search implementation
 * Pure TypeScript with zero external dependencies
 */
import { tokenize, getTermFrequencies, extractTextFromMetadata } from './TextUtils.js';
/**
 * BM25 Index for full-text keyword search
 */
export class BM25Index {
    k1;
    b;
    textFields;
    tokenizerOptions;
    // Document storage: id -> document data
    documents = new Map();
    // Inverted index: term -> count of documents containing this term
    documentFrequencies = new Map();
    // Statistics
    totalDocLength = 0;
    constructor(options) {
        this.k1 = options.k1 ?? 1.2;
        this.b = options.b ?? 0.75;
        this.textFields = options.textFields;
        this.tokenizerOptions = options.tokenizerOptions ?? {};
    }
    /**
     * Gets the text fields being indexed
     */
    get indexedFields() {
        return [...this.textFields];
    }
    /**
     * Gets the number of documents in the index
     */
    get documentCount() {
        return this.documents.size;
    }
    /**
     * Gets the average document length
     */
    get avgDocLength() {
        if (this.documents.size === 0)
            return 0;
        return this.totalDocLength / this.documents.size;
    }
    /**
     * Gets the vocabulary size (unique terms)
     */
    get vocabularySize() {
        return this.documentFrequencies.size;
    }
    /**
     * Adds a document to the index
     */
    addDocument(id, metadata) {
        // Remove existing document if present
        if (this.documents.has(id)) {
            this.removeDocument(id);
        }
        // Extract and tokenize text
        const text = extractTextFromMetadata(metadata, this.textFields);
        const tokens = tokenize(text, this.tokenizerOptions);
        const termFrequencies = getTermFrequencies(tokens);
        const docLength = tokens.length;
        // Update document frequencies (IDF)
        const seenTerms = new Set();
        for (const term of tokens) {
            if (!seenTerms.has(term)) {
                seenTerms.add(term);
                const current = this.documentFrequencies.get(term) || 0;
                this.documentFrequencies.set(term, current + 1);
            }
        }
        // Store document
        this.documents.set(id, {
            length: docLength,
            termFrequencies,
            metadata: metadata || undefined,
        });
        this.totalDocLength += docLength;
    }
    /**
     * Updates a document in the index
     */
    updateDocument(id, metadata) {
        this.addDocument(id, metadata);
    }
    /**
     * Removes a document from the index
     */
    removeDocument(id) {
        const doc = this.documents.get(id);
        if (!doc)
            return false;
        // Update document frequencies
        for (const [term] of doc.termFrequencies) {
            const current = this.documentFrequencies.get(term) || 0;
            if (current <= 1) {
                this.documentFrequencies.delete(term);
            }
            else {
                this.documentFrequencies.set(term, current - 1);
            }
        }
        // Update total length
        this.totalDocLength -= doc.length;
        // Remove document
        this.documents.delete(id);
        return true;
    }
    /**
     * Calculates the IDF (Inverse Document Frequency) for a term
     */
    calculateIDF(term) {
        const n = this.documentFrequencies.get(term) || 0;
        const N = this.documents.size;
        if (n === 0)
            return 0;
        // BM25 IDF formula (Robertson-Walker IDF)
        return Math.log(((N - n + 0.5) / (n + 0.5)) + 1);
    }
    /**
     * Calculates the BM25 score for a document given query terms
     */
    calculateScore(docId, queryTerms) {
        const doc = this.documents.get(docId);
        if (!doc)
            return 0;
        const avgdl = this.avgDocLength;
        if (avgdl === 0)
            return 0;
        let score = 0;
        for (const term of queryTerms) {
            const idf = this.calculateIDF(term);
            const tf = doc.termFrequencies.get(term) || 0;
            if (tf === 0)
                continue;
            // BM25 scoring formula
            const numerator = tf * (this.k1 + 1);
            const denominator = tf + this.k1 * (1 - this.b + this.b * (doc.length / avgdl));
            score += idf * (numerator / denominator);
        }
        return score;
    }
    /**
     * Searches the index and returns k most relevant documents
     */
    search(query, k) {
        if (this.documents.size === 0 || !query.trim()) {
            return [];
        }
        // Tokenize query
        const queryTerms = tokenize(query, this.tokenizerOptions);
        if (queryTerms.length === 0) {
            return [];
        }
        // Calculate scores for all documents
        const scores = [];
        for (const [docId] of this.documents) {
            const score = this.calculateScore(docId, queryTerms);
            if (score > 0) {
                scores.push({ id: docId, score });
            }
        }
        // Sort by score (descending)
        scores.sort((a, b) => b.score - a.score);
        // Return top k results with metadata
        return scores.slice(0, k).map(({ id, score }) => {
            const doc = this.documents.get(id);
            return {
                id,
                score,
                metadata: doc?.metadata,
            };
        });
    }
    /**
     * Gets statistics about the index
     */
    getStats() {
        return {
            documentCount: this.documentCount,
            avgDocLength: this.avgDocLength,
            vocabularySize: this.vocabularySize,
            k1: this.k1,
            b: this.b,
            textFields: this.textFields,
        };
    }
    /**
     * Serializes the index to a plain object
     */
    serialize() {
        const documents = [];
        for (const [id, doc] of this.documents) {
            const termFrequencies = {};
            for (const [term, freq] of doc.termFrequencies) {
                termFrequencies[term] = freq;
            }
            documents.push({
                id,
                length: doc.length,
                termFrequencies,
            });
        }
        const documentFrequencies = {};
        for (const [term, freq] of this.documentFrequencies) {
            documentFrequencies[term] = freq;
        }
        return {
            version: '1.0.0',
            k1: this.k1,
            b: this.b,
            textFields: this.textFields,
            avgDocLength: this.avgDocLength,
            documentCount: this.documentCount,
            documents,
            documentFrequencies,
        };
    }
    /**
     * Deserializes an index from a plain object
     */
    static deserialize(data) {
        const index = new BM25Index({
            k1: data.k1,
            b: data.b,
            textFields: data.textFields,
        });
        // Restore documents
        for (const doc of data.documents) {
            const termFrequencies = new Map();
            for (const [term, freq] of Object.entries(doc.termFrequencies)) {
                termFrequencies.set(term, freq);
            }
            index.documents.set(doc.id, {
                length: doc.length,
                termFrequencies,
            });
            index.totalDocLength += doc.length;
        }
        // Restore document frequencies
        for (const [term, freq] of Object.entries(data.documentFrequencies)) {
            index.documentFrequencies.set(term, freq);
        }
        return index;
    }
    /**
     * Clears the entire index
     */
    clear() {
        this.documents.clear();
        this.documentFrequencies.clear();
        this.totalDocLength = 0;
    }
}
//# sourceMappingURL=BM25Index.js.map