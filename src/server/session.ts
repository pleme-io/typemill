import type { ClientSession } from '../transports/websocket.js';

export interface DisconnectedSession {
  session: ClientSession;
  disconnectedAt: Date;
  timeoutId: NodeJS.Timeout;
}

export class SessionManager {
  private sessions = new Map<string, ClientSession>();
  private projectSessions = new Map<string, Set<string>>(); // projectId -> sessionIds
  private disconnectedSessions = new Map<string, DisconnectedSession>(); // sessionId -> disconnected session
  private readonly RECONNECTION_TIMEOUT_MS = 60000; // 60 seconds

  addSession(session: ClientSession): void {
    this.sessions.set(session.id, session);

    // Track sessions by project
    if (!this.projectSessions.has(session.projectId)) {
      this.projectSessions.set(session.projectId, new Set());
    }
    this.projectSessions.get(session.projectId)?.add(session.id);
  }

  /**
   * Handle session disconnection with recovery grace period
   */
  handleDisconnection(
    sessionId: string,
    onSessionExpired?: (session: ClientSession) => void
  ): void {
    const session = this.sessions.get(sessionId);
    if (!session) {
      return;
    }

    // Move session to disconnected state
    this.sessions.delete(sessionId);

    // Set up timeout for permanent removal
    const timeoutId = setTimeout(() => {
      this.permanentlyRemoveSession(sessionId);
      if (onSessionExpired) {
        onSessionExpired(session);
      }
    }, this.RECONNECTION_TIMEOUT_MS);

    // Store in disconnected sessions for potential reconnection
    this.disconnectedSessions.set(sessionId, {
      session,
      disconnectedAt: new Date(),
      timeoutId,
    });

    console.log(
      `Session ${sessionId} (project: ${session.projectId}) disconnected. Grace period: ${this.RECONNECTION_TIMEOUT_MS}ms`
    );
  }

  /**
   * Attempt to reconnect a session
   */
  reconnectSession(sessionId: string, newSocket: any): ClientSession | null {
    const disconnectedSession = this.disconnectedSessions.get(sessionId);
    if (!disconnectedSession) {
      return null;
    }

    // Clear the timeout
    clearTimeout(disconnectedSession.timeoutId);
    this.disconnectedSessions.delete(sessionId);

    // Update the session with new socket
    const reconnectedSession: ClientSession = {
      ...disconnectedSession.session,
      socket: newSocket,
      initialized: true,
    };

    // Add back to active sessions
    this.sessions.set(sessionId, reconnectedSession);

    const reconnectionDuration = Date.now() - disconnectedSession.disconnectedAt.getTime();
    console.log(
      `Session ${sessionId} (project: ${reconnectedSession.projectId}) reconnected after ${reconnectionDuration}ms`
    );

    return reconnectedSession;
  }

  /**
   * Allow client to reconnect with same project (even if session ID is lost)
   */
  findReconnectableSession(projectId: string, projectRoot: string): ClientSession | null {
    for (const [sessionId, disconnectedSession] of this.disconnectedSessions.entries()) {
      if (
        disconnectedSession.session.projectId === projectId &&
        disconnectedSession.session.projectRoot === projectRoot
      ) {
        // Clear the timeout
        clearTimeout(disconnectedSession.timeoutId);
        this.disconnectedSessions.delete(sessionId);

        return disconnectedSession.session;
      }
    }
    return null;
  }

  /**
   * Permanently remove session (original removeSession logic)
   */
  private permanentlyRemoveSession(sessionId: string): void {
    // Clean up disconnected session if it exists
    const disconnectedSession = this.disconnectedSessions.get(sessionId);
    if (disconnectedSession) {
      clearTimeout(disconnectedSession.timeoutId);
      this.disconnectedSessions.delete(sessionId);

      // Remove from project tracking
      const projectSessions = this.projectSessions.get(disconnectedSession.session.projectId);
      if (projectSessions) {
        projectSessions.delete(sessionId);
        if (projectSessions.size === 0) {
          this.projectSessions.delete(disconnectedSession.session.projectId);
        }
      }

      console.log(
        `Session ${sessionId} (project: ${disconnectedSession.session.projectId}) permanently removed after timeout`
      );
    }
  }

  /**
   * Legacy method for immediate removal (backwards compatibility)
   */
  removeSession(sessionId: string): void {
    this.permanentlyRemoveSession(sessionId);

    // Also remove from active sessions
    const session = this.sessions.get(sessionId);
    if (session) {
      this.sessions.delete(sessionId);

      // Remove from project tracking
      const projectSessions = this.projectSessions.get(session.projectId);
      if (projectSessions) {
        projectSessions.delete(sessionId);
        if (projectSessions.size === 0) {
          this.projectSessions.delete(session.projectId);
        }
      }
    }
  }

  getSession(sessionId: string): ClientSession | undefined {
    return this.sessions.get(sessionId);
  }

  getSessionsForProject(projectId: string): ClientSession[] {
    const sessionIds = this.projectSessions.get(projectId);
    if (!sessionIds) {
      return [];
    }

    return Array.from(sessionIds)
      .map((id) => this.sessions.get(id))
      .filter((session): session is ClientSession => session !== undefined);
  }

  getAllSessions(): ClientSession[] {
    return Array.from(this.sessions.values());
  }

  getActiveProjects(): string[] {
    return Array.from(this.projectSessions.keys());
  }

  hasActiveProject(projectId: string): boolean {
    return this.projectSessions.has(projectId) && this.projectSessions.get(projectId)?.size > 0;
  }

  /**
   * Get statistics about sessions and disconnections
   */
  getStats(): {
    activeSessions: number;
    disconnectedSessions: number;
    activeProjects: number;
    oldestDisconnection?: Date;
  } {
    let oldestDisconnection: Date | undefined;
    for (const disconnected of this.disconnectedSessions.values()) {
      if (!oldestDisconnection || disconnected.disconnectedAt < oldestDisconnection) {
        oldestDisconnection = disconnected.disconnectedAt;
      }
    }

    return {
      activeSessions: this.sessions.size,
      disconnectedSessions: this.disconnectedSessions.size,
      activeProjects: this.projectSessions.size,
      oldestDisconnection,
    };
  }

  /**
   * Clean up resources on shutdown
   */
  shutdown(): void {
    // Clear all reconnection timeouts
    for (const disconnected of this.disconnectedSessions.values()) {
      clearTimeout(disconnected.timeoutId);
    }

    this.sessions.clear();
    this.projectSessions.clear();
    this.disconnectedSessions.clear();

    console.log('SessionManager shutdown complete');
  }
}
