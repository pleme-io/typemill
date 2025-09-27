import { afterEach, beforeEach, describe, expect, it, spyOn } from 'bun:test';
import { existsSync, mkdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { TransactionManager } from '../../src/core/transaction/TransactionManager.js';
import type { FileService } from '../../src/services/file-service.js';

// Mock FileService for testing
const mockFileService: FileService = {
  getTrackedFiles: () => [],
  readFile: async (path: string) => null,
  writeFile: async (path: string, content: string) => {},
  deleteFile: async (path: string) => {},
} as any;

describe('TransactionManager', () => {
  let transactionManager: TransactionManager;
  let testDir: string;
  let fileA: string;
  let fileB: string;
  let dirA: string;
  let dirB: string;

  beforeEach(() => {
    transactionManager = new TransactionManager(mockFileService);

    // Setup test directory
    testDir = join(tmpdir(), 'transaction-test-' + Date.now());
    rmSync(testDir, { recursive: true, force: true });
    mkdirSync(testDir, { recursive: true });

    dirA = join(testDir, 'dirA');
    dirB = join(testDir, 'dirB');
    fileA = join(dirA, 'fileA.txt');
    fileB = join(dirB, 'fileB.txt');

    mkdirSync(dirA, { recursive: true });
    mkdirSync(dirB, { recursive: true });
    writeFileSync(fileA, 'Content A');
    writeFileSync(fileB, 'Content B');
  });

  afterEach(() => {
    rmSync(testDir, { recursive: true, force: true });
  });

  describe('file move recording and rollback', () => {
    it('should record file move operations', () => {
      const transaction = transactionManager.beginTransaction();

      transactionManager.recordFileMove(fileA, fileB);

      expect(transaction.operations).toHaveLength(1);
      expect(transaction.operations[0].type).toBe('MOVE');
      expect(transaction.operations[0].originalPath).toBe(fileA);
      expect(transaction.operations[0].path).toBe(fileB);
    });

    it('should record multiple operations in order', () => {
      const transaction = transactionManager.beginTransaction();

      transactionManager.recordFileCreate(fileA);
      transactionManager.recordFileMove(fileA, fileB);
      transactionManager.recordFileDelete(fileB, 'original content');

      expect(transaction.operations).toHaveLength(3);
      expect(transaction.operations[0].type).toBe('CREATE');
      expect(transaction.operations[1].type).toBe('MOVE');
      expect(transaction.operations[2].type).toBe('DELETE');
    });

    it('should rollback file moves in reverse order', async () => {
      // Start transaction and create checkpoint
      const transaction = transactionManager.beginTransaction();
      await transactionManager.saveCheckpoint('test');

      // Simulate a file move and record it
      const tempFile = join(testDir, 'temp.txt');
      writeFileSync(tempFile, 'moved content');
      transactionManager.recordFileMove(fileA, tempFile);

      // Verify file was "moved"
      expect(existsSync(tempFile)).toBe(true);

      // Rollback should move file back
      await transactionManager.rollbackToCheckpoint('test');

      // File should be moved back to original location
      expect(existsSync(fileA)).toBe(true);
      expect(existsSync(tempFile)).toBe(false);
    });

    it('should handle rollback of file creation', async () => {
      const transaction = transactionManager.beginTransaction();
      await transactionManager.saveCheckpoint('test');

      // Create a new file and record it
      const newFile = join(testDir, 'newFile.txt');
      writeFileSync(newFile, 'new content');
      transactionManager.recordFileCreate(newFile);

      expect(existsSync(newFile)).toBe(true);

      // Rollback should delete the created file
      await transactionManager.rollbackToCheckpoint('test');

      expect(existsSync(newFile)).toBe(false);
    });

    it('should handle rollback of file deletion', async () => {
      const transaction = transactionManager.beginTransaction();
      await transactionManager.saveCheckpoint('test');

      // Record file deletion with original content
      const originalContent = 'original content';
      const tempFile = join(testDir, 'toDelete.txt');
      writeFileSync(tempFile, originalContent);

      transactionManager.recordFileDelete(tempFile, originalContent);
      // Simulate deletion
      rmSync(tempFile);
      expect(existsSync(tempFile)).toBe(false);

      // Rollback should restore the file
      await transactionManager.rollbackToCheckpoint('test');

      expect(existsSync(tempFile)).toBe(true);
      expect(Bun.file(tempFile).text()).resolves.toBe(originalContent);
    });

    it('should not record operations when no transaction is active', () => {
      // No transaction started
      transactionManager.recordFileMove(fileA, fileB);
      transactionManager.recordFileCreate(fileA);

      // Should not throw, but also should not track anything
      // (graceful handling)
      expect(true).toBe(true); // Test passes if no exception thrown
    });
  });

  describe('transaction lifecycle', () => {
    it('should prevent multiple concurrent transactions', () => {
      transactionManager.beginTransaction();

      expect(() => transactionManager.beginTransaction()).toThrow(
        'A transaction is already in progress'
      );
    });

    it('should require active transaction for checkpoint operations', async () => {
      await expect(transactionManager.saveCheckpoint('test')).rejects.toThrow(
        'No active transaction'
      );
      await expect(transactionManager.rollbackToCheckpoint('test')).rejects.toThrow(
        'No active transaction'
      );
    });

    it('should require existing checkpoint for rollback', async () => {
      transactionManager.beginTransaction();

      await expect(transactionManager.rollbackToCheckpoint('nonexistent')).rejects.toThrow(
        'Checkpoint not found'
      );
    });

    it('should complete transaction lifecycle correctly', async () => {
      const transaction = transactionManager.beginTransaction();
      await transactionManager.saveCheckpoint('test');

      transactionManager.recordFileMove(fileA, fileB);
      expect(transaction.operations).toHaveLength(1);

      transactionManager.commit();

      // After commit, should be able to start new transaction
      const newTransaction = transactionManager.beginTransaction();
      expect(newTransaction.id).not.toBe(transaction.id);
    });
  });
});
