/**
 * Text processing utilities for BM25 keyword search
 */
export interface TokenizerOptions {
    lowercase?: boolean;
    removePunctuation?: boolean;
    removeNumbers?: boolean;
    minTokenLength?: number;
}
/**
 * Tokenizes text into normalized tokens
 */
export declare function tokenize(text: string, options?: TokenizerOptions): string[];
/**
 * Creates term frequency map for a document
 */
export declare function getTermFrequencies(tokens: string[]): Map<string, number>;
/**
 * Extracts text from metadata fields and concatenates them
 */
export declare function extractTextFromMetadata(metadata: Record<string, unknown> | null, textFields: string[]): string;
//# sourceMappingURL=TextUtils.d.ts.map