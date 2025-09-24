/**
 * FUSE filesystem types and interfaces
 * Consolidated type definitions for FUSE operations
 */

import type { Stats } from 'node:fs';
import type { FuseOperationResponse } from './session.js';

/**
 * Enhanced FUSE stats that includes all required properties
 */
export interface FuseStats {
  mode: number;
  size: number;
  mtime: Date;
  atime: Date;
  ctime: Date;
  uid: number;
  gid: number;
  dev: number;
  ino: number;
  nlink: number;
  rdev: number;
  blksize: number;
  blocks: number;
}

/**
 * FUSE callback function types
 */
export type FuseErrorCallback = (errno: number) => void;
export type FuseReaddirCallback = (errno: number, files?: string[]) => void;
export type FuseGetattrCallback = (errno: number, stats?: Stats | FuseStats) => void;
export type FuseOpenCallback = (errno: number, fd?: number) => void;
export type FuseReadCallback = (bytesRead: number) => void;
export type FuseWriteCallback = (bytesWritten: number) => void;

/**
 * FUSE mount options
 */
export interface MountOptions {
  debug?: boolean;
  allowOther?: boolean;
  allowRoot?: boolean;
  defaultPermissions?: boolean;
  [key: string]: unknown;
}

/**
 * Async operation callback with proper typing
 */
export interface AsyncOperationCallback<T = unknown> {
  resolve: (value: T) => void;
  reject: (reason?: Error) => void;
  timeout: NodeJS.Timeout;
}

/**
 * File operation response types
 */
export type FileOperationResult = string[] | FuseStats | number | Buffer | undefined;

/**
 * FUSE operation handler response
 */
export type FuseHandlerResponse = FuseOperationResponse | Error;
