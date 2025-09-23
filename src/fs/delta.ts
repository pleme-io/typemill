/**
 * Delta update system for efficient file synchronization
 * Minimizes network payload by sending only file diffs instead of complete content
 */

import DiffMatchPatch from 'diff-match-patch';
import { logger } from '../core/diagnostics/logger.js';

export interface FileDelta {
  filePath: string;
  baseVersion: string; // Hash or version identifier of the base content
  patches: string; // Serialized patches in diff-match-patch format
  newVersion: string; // Hash or version identifier after applying patches
  fullSize: number; // Size of full file content
  deltaSize: number; // Size of the delta/patches
  compressionRatio: number; // Ratio of space saved (1.0 = no savings, 0.5 = 50% savings)
}

export interface DeltaUpdateRequest {
  filePath: string;
  baseVersion?: string; // If provided, try to generate delta from this version
  patches?: string; // If provided, apply these patches to cached content
}

export interface DeltaUpdateResponse {
  content: string;
  version: string;
  usedDelta: boolean;
  compressionRatio?: number;
}

export class DeltaProcessor {
  private dmp: DiffMatchPatch;
  private readonly MIN_DELTA_SIZE = 1024; // Don't use delta for files smaller than 1KB
  private readonly MAX_PATCH_SIZE_RATIO = 0.8; // Don't use delta if patches are >80% of original

  constructor() {
    this.dmp = new DiffMatchPatch();
    // Configure diff-match-patch for optimal performance
    this.dmp.Diff_Timeout = 1.0; // 1 second timeout for large files
    this.dmp.Diff_EditCost = 4; // Cost of edit operations
  }

  /**
   * Generate a delta between old and new file content
   */
  generateDelta(oldContent: string, newContent: string, filePath: string): FileDelta | null {
    const startTime = Date.now();

    try {
      // Skip delta for small files
      if (newContent.length < this.MIN_DELTA_SIZE) {
        logger.debug('File too small for delta compression', {
          component: 'DeltaProcessor',
          filePath,
          fileSize: newContent.length,
          minSize: this.MIN_DELTA_SIZE,
        });
        return null;
      }

      // Generate diffs
      const diffs = this.dmp.diff_main(oldContent, newContent);
      this.dmp.diff_cleanupSemantic(diffs);

      // Create patches
      const patches = this.dmp.patch_make(oldContent, diffs);
      const patchText = this.dmp.patch_toText(patches);

      const deltaSize = patchText.length;
      const fullSize = newContent.length;
      const compressionRatio = deltaSize / fullSize;

      // Skip delta if it's not significantly smaller
      if (compressionRatio > this.MAX_PATCH_SIZE_RATIO) {
        logger.debug('Delta not efficient enough, using full content', {
          component: 'DeltaProcessor',
          filePath,
          fullSize,
          deltaSize,
          compressionRatio,
          maxRatio: this.MAX_PATCH_SIZE_RATIO,
        });
        return null;
      }

      const processingTime = Date.now() - startTime;

      const delta: FileDelta = {
        filePath,
        baseVersion: this.generateContentHash(oldContent),
        patches: patchText,
        newVersion: this.generateContentHash(newContent),
        fullSize,
        deltaSize,
        compressionRatio,
      };

      logger.info('Delta generated successfully', {
        component: 'DeltaProcessor',
        filePath,
        fullSize,
        deltaSize,
        compressionRatio: Math.round((1 - compressionRatio) * 100) / 100,
        spaceSaved: fullSize - deltaSize,
        processingTimeMs: processingTime,
      });

      return delta;
    } catch (error) {
      logger.error('Failed to generate delta', error as Error, {
        component: 'DeltaProcessor',
        filePath,
        oldContentLength: oldContent.length,
        newContentLength: newContent.length,
      });
      return null;
    }
  }

  /**
   * Apply a delta to base content to reconstruct the new content
   */
  applyDelta(baseContent: string, delta: FileDelta): string | null {
    const startTime = Date.now();

    try {
      // Verify base version matches
      const baseVersion = this.generateContentHash(baseContent);
      if (baseVersion !== delta.baseVersion) {
        logger.warn('Base version mismatch when applying delta', {
          component: 'DeltaProcessor',
          filePath: delta.filePath,
          expectedBaseVersion: delta.baseVersion,
          actualBaseVersion: baseVersion,
        });
        return null;
      }

      // Parse and apply patches
      const patches = this.dmp.patch_fromText(delta.patches);
      const [newContent, results] = this.dmp.patch_apply(patches, baseContent);

      // Check if all patches applied successfully
      const failedPatches = results.filter((success: boolean) => !success).length;
      if (failedPatches > 0) {
        logger.error('Some patches failed to apply', new Error('Patch application failed'), {
          component: 'DeltaProcessor',
          filePath: delta.filePath,
          totalPatches: results.length,
          failedPatches,
        });
        return null;
      }

      // Verify resulting content version
      const resultVersion = this.generateContentHash(newContent);
      if (resultVersion !== delta.newVersion) {
        logger.error(
          'Result version mismatch after applying delta',
          new Error('Version verification failed'),
          {
            component: 'DeltaProcessor',
            filePath: delta.filePath,
            expectedVersion: delta.newVersion,
            actualVersion: resultVersion,
          }
        );
        return null;
      }

      const processingTime = Date.now() - startTime;

      logger.info('Delta applied successfully', {
        component: 'DeltaProcessor',
        filePath: delta.filePath,
        compressionRatio: Math.round((1 - delta.compressionRatio) * 100) / 100,
        processingTimeMs: processingTime,
      });

      return newContent;
    } catch (error) {
      logger.error('Failed to apply delta', error as Error, {
        component: 'DeltaProcessor',
        filePath: delta.filePath,
        baseContentLength: baseContent.length,
      });
      return null;
    }
  }

  /**
   * Generate a simple hash for content versioning
   */
  private generateContentHash(content: string): string {
    let hash = 0;
    for (let i = 0; i < content.length; i++) {
      const char = content.charCodeAt(i);
      hash = (hash << 5) - hash + char;
      hash = hash & hash; // Convert to 32-bit integer
    }
    return hash.toString(36);
  }

  /**
   * Estimate if delta would be beneficial without actually generating it
   */
  shouldUseDelta(oldContent: string, newContent: string): boolean {
    // Skip for small files
    if (newContent.length < this.MIN_DELTA_SIZE) {
      return false;
    }

    // Quick heuristic: if content is >90% similar by length, likely beneficial
    const lengthRatio =
      Math.abs(oldContent.length - newContent.length) /
      Math.max(oldContent.length, newContent.length);
    return lengthRatio < 0.5; // Use delta if length difference is <50%
  }

  /**
   * Get statistics about delta processing performance
   */
  getStats(): {
    diffTimeout: number;
    editCost: number;
    minDeltaSize: number;
    maxPatchSizeRatio: number;
  } {
    return {
      diffTimeout: this.dmp.Diff_Timeout,
      editCost: this.dmp.Diff_EditCost,
      minDeltaSize: this.MIN_DELTA_SIZE,
      maxPatchSizeRatio: this.MAX_PATCH_SIZE_RATIO,
    };
  }
}
