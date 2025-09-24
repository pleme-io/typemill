import { randomUUID } from 'node:crypto';
import type { FileService } from '../../services/file-service.js';
import type { FileSystemSnapshot, Transaction } from './types.js';

export class TransactionManager {
  private activeTransaction: Transaction | null = null;

  constructor(private fileService: FileService) {}

  beginTransaction(): Transaction {
    if (this.activeTransaction) {
      throw new Error('A transaction is already in progress.');
    }
    const transactionId = randomUUID();
    this.activeTransaction = {
      id: transactionId,
      checkpoints: new Map(),
    };
    return this.activeTransaction;
  }

  async saveCheckpoint(name: string): Promise<void> {
    if (!this.activeTransaction) {
      throw new Error('No active transaction.');
    }
    const trackedFiles = this.fileService.getTrackedFiles();
    const snapshot = await this.captureState(trackedFiles);
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
  }

  commit(): void {
    if (!this.activeTransaction) {
      throw new Error('No active transaction.');
    }
    this.activeTransaction = null;
  }

  private async captureState(filePaths: string[]): Promise<FileSystemSnapshot> {
    const files = new Map<string, string | null>();
    for (const filePath of filePaths) {
      const content = await this.fileService.readFile(filePath);
      files.set(filePath, content);
    }
    return { files };
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
}
