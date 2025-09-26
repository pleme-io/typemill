import { randomUUID } from 'node:crypto';
import { existsSync, renameSync, unlinkSync, mkdirSync, writeFileSync } from 'node:fs';
import { dirname } from 'node:path';
import type { FileService } from '../../services/file-service.js';
import type { FileSystemSnapshot, Transaction, FileOperation } from './types.js';

export class TransactionManager {
  private activeTransaction: Transaction | null = null;
  private fileService?: FileService;

  constructor(fileService?: FileService) {
    this.fileService = fileService;
  }

  setFileService(fileService: FileService): void {
    this.fileService = fileService;
  }

  beginTransaction(): Transaction {
    if (this.activeTransaction) {
      throw new Error('A transaction is already in progress.');
    }
    const transactionId = randomUUID();
    this.activeTransaction = {
      id: transactionId,
      checkpoints: new Map(),
      operations: [],
    };
    return this.activeTransaction;
  }

  async saveCheckpoint(name: string): Promise<void> {
    if (!this.activeTransaction) {
      throw new Error('No active transaction.');
    }
    // If fileService is not available, use an empty array of tracked files
    const trackedFiles = this.fileService?.getTrackedFiles() || [];
    const snapshot = await this.captureState(trackedFiles);
    snapshot.operations = [...this.activeTransaction.operations]; // Copy current operations
    this.activeTransaction.checkpoints.set(name, snapshot);
  }

  async rollbackToCheckpoint(name: string): Promise<void> {
    if (!this.activeTransaction) {
      throw new Error('No active transaction.');
    }
    const snapshot = this.activeTransaction.checkpoints.get(name);
    if (!snapshot) {
      throw new Error(`Checkpoint not found: ${name}`);
    }
    await this.restoreState(snapshot);
    // Also rollback operations that happened after the checkpoint
    await this.rollbackOperations(snapshot.operations);
  }

  commit(): void {
    if (!this.activeTransaction) {
      throw new Error('No active transaction.');
    }
    this.activeTransaction = null;
  }

  recordFileMove(oldPath: string, newPath: string): void {
    if (!this.activeTransaction) {
      return; // If no active transaction, don't record operations
    }

    this.activeTransaction.operations.push({
      type: 'MOVE',
      path: newPath,
      originalPath: oldPath,
      timestamp: Date.now(),
    });
  }

  recordFileCreate(path: string): void {
    if (!this.activeTransaction) {
      return; // If no active transaction, don't record operations
    }

    this.activeTransaction.operations.push({
      type: 'CREATE',
      path,
      timestamp: Date.now(),
    });
  }

  recordFileDelete(path: string, originalContent?: string): void {
    if (!this.activeTransaction) {
      return; // If no active transaction, don't record operations
    }

    this.activeTransaction.operations.push({
      type: 'DELETE',
      path,
      originalContent,
      timestamp: Date.now(),
    });
  }

  recordFileModify(path: string, originalContent: string): void {
    if (!this.activeTransaction) {
      return; // If no active transaction, don't record operations
    }

    this.activeTransaction.operations.push({
      type: 'MODIFY',
      path,
      originalContent,
      timestamp: Date.now(),
    });
  }

  private async captureState(filePaths: string[]): Promise<FileSystemSnapshot> {
    const files = new Map<string, string | null>();
    for (const filePath of filePaths) {
      const content = await this.fileService.readFile(filePath);
      files.set(filePath, content);
    }
    return { files, operations: [] }; // operations will be set by saveCheckpoint
  }

  private async restoreState(snapshot: FileSystemSnapshot): Promise<void> {
    for (const [filePath, content] of snapshot.files.entries()) {
      if (content === null) {
        await this.fileService.deleteFile(filePath);
      } else {
        await this.fileService.writeFile(filePath, content);
      }
    }
  }

  private async rollbackOperations(checkpointOperations: FileOperation[]): Promise<void> {
    if (!this.activeTransaction) {
      return;
    }

    // Get operations that happened after the checkpoint
    const checkpointTimestamp = checkpointOperations.length > 0
      ? Math.max(...checkpointOperations.map(op => op.timestamp))
      : 0;

    const operationsToRollback = this.activeTransaction.operations
      .filter(op => op.timestamp > checkpointTimestamp)
      .reverse(); // Rollback in reverse order


    for (const operation of operationsToRollback) {
      try {
        switch (operation.type) {
          case 'CREATE':
            // Rollback create: delete the file
            if (existsSync(operation.path)) {
              unlinkSync(operation.path);
            }
            break;

          case 'DELETE':
            // Rollback delete: restore the file
            if (operation.originalContent !== undefined) {
              const dir = dirname(operation.path);
              if (!existsSync(dir)) {
                mkdirSync(dir, { recursive: true });
              }
              if (operation.originalContent !== null) {
                writeFileSync(operation.path, operation.originalContent, 'utf-8');
              }
            }
            break;

          case 'MOVE':
            // Rollback move: move the file back to original location
            if (operation.originalPath && existsSync(operation.path)) {
              const originalDir = dirname(operation.originalPath);
              if (!existsSync(originalDir)) {
                mkdirSync(originalDir, { recursive: true });
              }
              renameSync(operation.path, operation.originalPath);
            }
            break;

          case 'MODIFY':
            // Rollback modify: restore original content
            if (operation.originalContent !== undefined && existsSync(operation.path)) {
              writeFileSync(operation.path, operation.originalContent, 'utf-8');
            }
            break;
        }
      } catch (error) {
        console.warn(`Failed to rollback operation ${operation.type} on ${operation.path}:`, error);
        // Continue with other rollback operations even if one fails
      }
    }

    // Reset operations list to checkpoint state
    this.activeTransaction.operations = [...checkpointOperations];
  }
}
