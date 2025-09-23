import { randomUUID } from 'node:crypto';
import type WebSocket from 'ws';
import type { FileDelta } from '../fs/delta.js';
import type {
  EnhancedClientSession,
  FuseOperationResponse,
  WorkspaceInfo,
} from '../types/session.js';

export interface ClientSession {
  id: string;
  projectId: string;
  projectRoot: string;
  socket: WebSocket;
  initialized: boolean;
}

export interface InitializeMessage {
  method: 'initialize';
  project: string;
  projectRoot: string;
  token?: string; // JWT authentication token
  id?: string;
}

export interface ReconnectMessage {
  method: 'reconnect';
  sessionId: string;
  id?: string;
}

export interface MCPMessage {
  id?: string;
  method: string;
  params?: any;
  result?: any;
  error?: any;
}

export interface DeltaWriteRequest {
  path: string;
  delta: FileDelta;
}

export interface DeltaWriteResponse {
  success: boolean;
  usedDelta: boolean;
  finalSize?: number;
}

export class WebSocketTransport {
  private sessions = new Map<string, ClientSession>();
  private enhancedSessions = new Map<string, EnhancedClientSession>();
  private pendingRequests = new Map<string, { resolve: Function; reject: Function }>();

  constructor(
    private onMessage: (session: ClientSession, message: MCPMessage) => Promise<any>,
    private onSessionReconnect?: (sessionId: string, socket: WebSocket) => ClientSession | null,
    private onSessionDisconnect?: (sessionId: string) => void,
    private validateToken?: (token: string, projectId: string) => Promise<boolean>,
    private onWorkspaceCreate?: (session: ClientSession) => Promise<WorkspaceInfo>,
    private onFuseResponse?: (sessionId: string, response: FuseOperationResponse) => void
  ) {}

  handleConnection(socket: WebSocket): void {
    let session: ClientSession | null = null;

    socket.on('message', async (data: Buffer) => {
      try {
        const message = JSON.parse(data.toString()) as MCPMessage;

        if (!session && message.method === 'initialize') {
          const initMsg = message as unknown as InitializeMessage;

          // Validate JWT token if authentication is enabled
          if (this.validateToken && initMsg.token) {
            try {
              const isValid = await this.validateToken(initMsg.token, initMsg.project);
              if (!isValid) {
                socket.send(
                  JSON.stringify({
                    id: message.id,
                    error: { code: -32000, message: 'Authentication failed: Invalid token' },
                  })
                );
                socket.close(1008, 'Authentication failed');
                return;
              }
            } catch (error) {
              socket.send(
                JSON.stringify({
                  id: message.id,
                  error: { code: -32000, message: 'Authentication failed: Token validation error' },
                })
              );
              socket.close(1008, 'Authentication error');
              return;
            }
          } else if (this.validateToken) {
            // Authentication required but no token provided
            socket.send(
              JSON.stringify({
                id: message.id,
                error: { code: -32000, message: 'Authentication required: No token provided' },
              })
            );
            socket.close(1008, 'Authentication required');
            return;
          }

          session = {
            id: randomUUID(),
            projectId: initMsg.project,
            projectRoot: initMsg.projectRoot,
            socket,
            initialized: true,
          };

          this.sessions.set(session.id, session);

          // Create workspace and enhanced session if workspace manager available
          let workspaceInfo: WorkspaceInfo | undefined;
          if (this.onWorkspaceCreate) {
            try {
              workspaceInfo = await this.onWorkspaceCreate(session);

              // Create enhanced session
              const enhancedSession: EnhancedClientSession = {
                ...session,
                globalProjectId: workspaceInfo.globalProjectId,
                workspaceId: workspaceInfo.workspaceId,
                fuseMount: workspaceInfo.fuseMount,
                workspaceDir: workspaceInfo.workspaceDir,
              };

              this.enhancedSessions.set(session.id, enhancedSession);
            } catch (error) {
              console.error('Failed to create workspace for session:', error);
              // Continue without workspace - fallback to basic session
            }
          }

          // Send initialize response
          socket.send(
            JSON.stringify({
              id: message.id,
              result: {
                sessionId: session.id,
                workspace: workspaceInfo
                  ? {
                      workspaceId: workspaceInfo.workspaceId,
                      globalProjectId: workspaceInfo.globalProjectId,
                      fuseMount: workspaceInfo.fuseMount,
                    }
                  : undefined,
              },
            })
          );

          return;
        }

        if (!session && message.method === 'reconnect') {
          const reconnectMsg = message as unknown as ReconnectMessage;

          // Try to reconnect using session manager
          if (this.onSessionReconnect) {
            const reconnectedSession = this.onSessionReconnect(reconnectMsg.sessionId, socket);
            if (reconnectedSession) {
              session = reconnectedSession;
              this.sessions.set(session.id, session);

              // Send reconnect success response
              socket.send(
                JSON.stringify({
                  id: message.id,
                  result: {
                    sessionId: session.id,
                    reconnected: true,
                    projectId: session.projectId,
                  },
                })
              );

              return;
            }
          }

          // Reconnection failed
          socket.send(
            JSON.stringify({
              id: message.id,
              error: {
                code: -1,
                message: 'Session not found or expired. Please initialize a new session.',
              },
            })
          );
          return;
        }

        if (!session) {
          socket.send(
            JSON.stringify({
              id: message.id,
              error: {
                code: -1,
                message: 'Session not initialized. Send initialize message first.',
              },
            })
          );
          return;
        }

        // Handle FUSE operation responses
        if (message.method?.startsWith('fuse/') && this.onFuseResponse) {
          this.onFuseResponse(session.id, message as unknown as FuseOperationResponse);
          return;
        }

        // Handle response to our request
        if (message.id && this.pendingRequests.has(message.id)) {
          const pending = this.pendingRequests.get(message.id)!;
          this.pendingRequests.delete(message.id);

          if (message.error) {
            pending.reject(new Error(message.error.message));
          } else {
            pending.resolve(message.result);
          }
          return;
        }

        // Handle regular MCP tool request
        try {
          const result = await this.onMessage(session, message);

          if (message.id) {
            socket.send(
              JSON.stringify({
                id: message.id,
                result,
              })
            );
          }
        } catch (error) {
          if (message.id) {
            socket.send(
              JSON.stringify({
                id: message.id,
                error: {
                  code: -1,
                  message: error instanceof Error ? error.message : 'Unknown error',
                },
              })
            );
          }
        }
      } catch (parseError) {
        socket.send(
          JSON.stringify({
            error: {
              code: -32700,
              message: 'Parse error',
            },
          })
        );
      }
    });

    socket.on('close', () => {
      if (session) {
        this.sessions.delete(session.id);
        this.enhancedSessions.delete(session.id);
        // Notify session manager about disconnection
        if (this.onSessionDisconnect) {
          this.onSessionDisconnect(session.id);
        }
      }
    });

    socket.on('error', (error) => {
      console.error('WebSocket error:', error);
      if (session) {
        this.sessions.delete(session.id);
        this.enhancedSessions.delete(session.id);
        // Notify session manager about disconnection
        if (this.onSessionDisconnect) {
          this.onSessionDisconnect(session.id);
        }
      }
    });
  }

  async sendRequest(session: ClientSession, method: string, params: any): Promise<any> {
    return new Promise((resolve, reject) => {
      const id = randomUUID();

      this.pendingRequests.set(id, { resolve, reject });

      const message = {
        id,
        method,
        params,
      };

      session.socket.send(JSON.stringify(message));

      // Timeout after 30 seconds
      setTimeout(() => {
        if (this.pendingRequests.has(id)) {
          this.pendingRequests.delete(id);
          reject(new Error('Request timeout'));
        }
      }, 30000);
    });
  }

  getSessions(): ClientSession[] {
    return Array.from(this.sessions.values());
  }

  getSession(id: string): ClientSession | undefined {
    return this.sessions.get(id);
  }

  getEnhancedSession(id: string): EnhancedClientSession | undefined {
    return this.enhancedSessions.get(id);
  }

  getEnhancedSessions(): EnhancedClientSession[] {
    return Array.from(this.enhancedSessions.values());
  }
}
