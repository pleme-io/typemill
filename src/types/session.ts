/**
 * Session type definitions for WebSocket/enhanced session functionality
 * Extracted from enhanced-session.ts during Phase 1 refactoring
 */

export interface SessionOptions {
  projectRoot?: string;
  workspaceDir?: string;
  sessionId?: string;
  timeout?: number;
}

export interface SessionState {
  id: string;
  projectRoot: string;
  workspaceDir: string;
  isActive: boolean;
  lastActivity: Date;
  connectionCount: number;
}

export interface SessionContext {
  session: SessionState;
  options: SessionOptions;
  cleanup: () => Promise<void>;
}

export interface SessionManager {
  createSession: (options: SessionOptions) => Promise<SessionContext>;
  getSession: (sessionId: string) => SessionContext | null;
  destroySession: (sessionId: string) => Promise<void>;
  listSessions: () => SessionState[];
}