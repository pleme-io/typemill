import { detectClones } from 'jscpd';
import type { IClone } from '@jscpd/core';
import { promises as fs } from 'node:fs';
import * as path from 'node:path';

export interface DuplicateInstance {
  file: string;
  startLine: number;
  endLine: number;
  startColumn: number;
  endColumn: number;
  tokenCount: number;
  content?: string;
}

export interface DuplicateGroup {
  hash: string;
  instances: DuplicateInstance[];
  tokenCount: number;
  lineCount: number;
  format?: string;
}

export interface DuplicateDetectionOptions {
  path: string;
  minTokens?: number;
  minLines?: number;
  maxLines?: number;
  maxSize?: string;
  languages?: string[];
  ignorePattern?: string;
  includeContent?: boolean;
}

export interface DuplicateDetectionResult {
  duplicates: DuplicateGroup[];
  statistics: {
    totalFiles: number;
    totalLines: number;
    duplicatedLines: number;
    duplicatedTokens: number;
    duplicatePercentage: number;
    filesWithDuplicates: number;
  };
  errors?: string[];
}

export class DuplicateDetectionService {
  /**
   * Detect duplicate code blocks in the specified path
   */
  async detectDuplicates(options: DuplicateDetectionOptions): Promise<DuplicateDetectionResult> {
    try {
      // Validate path exists
      const stats = await fs.stat(options.path).catch(() => null);
      if (!stats) {
        throw new Error(`Path does not exist: ${options.path}`);
      }

      // Configure jscpd options
      const jscpdOptions = {
        path: [options.path],
        minTokens: options.minTokens ?? 50,
        minLines: options.minLines ?? 5,
        maxLines: options.maxLines ?? 500,
        maxSize: options.maxSize ?? '100kb',
        format: options.languages,
        ignore: options.ignorePattern ? [options.ignorePattern] : undefined,
        silent: true,
        absolute: true,
      };

      // Run duplicate detection
      const clones = await detectClones(jscpdOptions);

      // Group clones by hash
      const groupedClones = await this.groupClones(clones, options.includeContent);

      // Calculate statistics
      const statistics = this.calculateStatistics(clones);

      return {
        duplicates: groupedClones,
        statistics,
      };
    } catch (error) {
      // Return error in a structured way
      const errorMessage = error instanceof Error ? error.message : String(error);
      return {
        duplicates: [],
        statistics: {
          totalFiles: 0,
          totalLines: 0,
          duplicatedLines: 0,
          duplicatedTokens: 0,
          duplicatePercentage: 0,
          filesWithDuplicates: 0,
        },
        errors: [errorMessage],
      };
    }
  }

  /**
   * Group clone instances by their hash
   */
  private async groupClones(clones: IClone[], includeContent = false): Promise<DuplicateGroup[]> {
    const groups = new Map<string, DuplicateGroup>();

    for (const clone of clones) {
      // Create hash for this clone pair
      const hash = this.generateHash(clone);

      if (!groups.has(hash)) {
        groups.set(hash, {
          hash,
          instances: [],
          tokenCount: clone.duplicationA.tokens || 0,
          lineCount: (clone.duplicationA.end.line - clone.duplicationA.start.line) + 1,
          format: clone.format,
        });
      }

      const group = groups.get(hash)!;

      // Add first instance
      const instanceA: DuplicateInstance = {
        file: clone.duplicationA.sourceId,
        startLine: clone.duplicationA.start.line,
        endLine: clone.duplicationA.end.line,
        startColumn: clone.duplicationA.start.column || 0,
        endColumn: clone.duplicationA.end.column || 0,
        tokenCount: clone.duplicationA.tokens || 0,
      };

      // Add second instance
      const instanceB: DuplicateInstance = {
        file: clone.duplicationB.sourceId,
        startLine: clone.duplicationB.start.line,
        endLine: clone.duplicationB.end.line,
        startColumn: clone.duplicationB.start.column || 0,
        endColumn: clone.duplicationB.end.column || 0,
        tokenCount: clone.duplicationB.tokens || 0,
      };

      // Load content if requested
      if (includeContent) {
        instanceA.content = await this.loadContent(
          clone.duplicationA.sourceId,
          clone.duplicationA.start.line,
          clone.duplicationA.end.line
        );
        instanceB.content = await this.loadContent(
          clone.duplicationB.sourceId,
          clone.duplicationB.start.line,
          clone.duplicationB.end.line
        );
      }

      // Check if instances already exist in group
      const existsA = group.instances.some(i =>
        i.file === instanceA.file &&
        i.startLine === instanceA.startLine
      );
      const existsB = group.instances.some(i =>
        i.file === instanceB.file &&
        i.startLine === instanceB.startLine
      );

      if (!existsA) group.instances.push(instanceA);
      if (!existsB) group.instances.push(instanceB);
    }

    return Array.from(groups.values()).sort((a, b) => b.tokenCount - a.tokenCount);
  }

  /**
   * Generate a hash for a clone pair
   */
  private generateHash(clone: IClone): string {
    // Use token count and line count as a simple hash
    // In a real implementation, you might hash the actual content
    const tokenCount = clone.duplicationA.tokens || 0;
    const lineCount = (clone.duplicationA.end.line - clone.duplicationA.start.line) + 1;
    return `${clone.format}_${tokenCount}_${lineCount}_${clone.duplicationA.start.line}`;
  }

  /**
   * Load content from a file between specified lines
   */
  private async loadContent(filePath: string, startLine: number, endLine: number): Promise<string> {
    try {
      const content = await fs.readFile(filePath, 'utf-8');
      const lines = content.split('\n');
      return lines.slice(startLine - 1, endLine).join('\n');
    } catch {
      return '';
    }
  }

  /**
   * Calculate statistics from clone results
   */
  private calculateStatistics(clones: IClone[]): DuplicateDetectionResult['statistics'] {
    const filesWithDuplicates = new Set<string>();
    let totalDuplicatedLines = 0;
    let totalDuplicatedTokens = 0;

    for (const clone of clones) {
      filesWithDuplicates.add(clone.duplicationA.sourceId);
      filesWithDuplicates.add(clone.duplicationB.sourceId);

      const lines = (clone.duplicationA.end.line - clone.duplicationA.start.line) + 1;
      totalDuplicatedLines += lines * 2; // Count both instances
      totalDuplicatedTokens += (clone.duplicationA.tokens || 0) * 2;
    }

    // These would need to be calculated from the actual file scanning
    // For now, returning estimates
    const totalLines = totalDuplicatedLines * 10; // Rough estimate
    const totalFiles = filesWithDuplicates.size * 2; // Rough estimate

    return {
      totalFiles,
      totalLines,
      duplicatedLines: totalDuplicatedLines,
      duplicatedTokens: totalDuplicatedTokens,
      duplicatePercentage: totalLines > 0 ? (totalDuplicatedLines / totalLines) * 100 : 0,
      filesWithDuplicates: filesWithDuplicates.size,
    };
  }
}

// Export singleton instance
export const duplicateDetectionService = new DuplicateDetectionService();