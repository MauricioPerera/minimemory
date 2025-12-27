/**
 * BM25 (Best Matching 25) keyword search implementation
 * Pure TypeScript with zero external dependencies
 */
import { TokenizerOptions } from './TextUtils.js';
export interface BM25Options {
    k1?: number;
    b?: number;
    textFields: string[];
    tokenizerOptions?: TokenizerOptions;
}
export interface BM25SearchResult {
    id: string;
    score: number;
    metadata?: Record<string, unknown>;
}
export interface SerializedBM25Index {
    version: string;
    k1: number;
    b: number;
    textFields: string[];
    avgDocLength: number;
    documentCount: number;
    documents: Array<{
        id: string;
        length: number;
        termFrequencies: Record<string, number>;
    }>;
    documentFrequencies: Record<string, number>;
}
/**
 * BM25 Index for full-text keyword search
 */
export declare class BM25Index {
    private k1;
    private b;
    private textFields;
    private tokenizerOptions;
    private documents;
    private documentFrequencies;
    private totalDocLength;
    constructor(options: BM25Options);
    /**
     * Gets the text fields being indexed
     */
    get indexedFields(): string[];
    /**
     * Gets the number of documents in the index
     */
    get documentCount(): number;
    /**
     * Gets the average document length
     */
    get avgDocLength(): number;
    /**
     * Gets the vocabulary size (unique terms)
     */
    get vocabularySize(): number;
    /**
     * Adds a document to the index
     */
    addDocument(id: string, metadata: Record<string, unknown> | null): void;
    /**
     * Updates a document in the index
     */
    updateDocument(id: string, metadata: Record<string, unknown> | null): void;
    /**
     * Removes a document from the index
     */
    removeDocument(id: string): boolean;
    /**
     * Calculates the IDF (Inverse Document Frequency) for a term
     */
    private calculateIDF;
    /**
     * Calculates the BM25 score for a document given query terms
     */
    private calculateScore;
    /**
     * Searches the index and returns k most relevant documents
     */
    search(query: string, k: number): BM25SearchResult[];
    /**
     * Gets statistics about the index
     */
    getStats(): {
        documentCount: number;
        avgDocLength: number;
        vocabularySize: number;
        k1: number;
        b: number;
        textFields: string[];
    };
    /**
     * Serializes the index to a plain object
     */
    serialize(): SerializedBM25Index;
    /**
     * Deserializes an index from a plain object
     */
    static deserialize(data: SerializedBM25Index): BM25Index;
    /**
     * Clears the entire index
     */
    clear(): void;
}
//# sourceMappingURL=BM25Index.d.ts.map