/**
 * JWT-based authentication system for CodeFlow Buddy
 * Provides secure token-based authentication for client connections
 */

import { randomBytes } from 'node:crypto';
import jwt from 'jsonwebtoken';
import { logger } from '../core/logger.js';

export interface AuthConfig {
  secretKey: string;
  tokenExpiry: string; // e.g., '24h', '7d'
  issuer: string;
  audience: string;
}

export interface ProjectCredentials {
  projectId: string;
  secretKey: string;
}

export interface TokenPayload {
  projectId: string;
  sessionId?: string;
  permissions: string[];
  iat: number;
  exp: number;
  iss: string;
  aud: string;
}

export interface AuthRequest {
  projectId: string;
  secretKey: string;
  sessionId?: string;
}

export interface AuthResponse {
  token: string;
  expiresAt: Date;
  permissions: string[];
}

export class JWTAuthenticator {
  private readonly defaultPermissions = [
    'file:read',
    'file:write',
    'lsp:query',
    'lsp:symbol',
    'session:manage',
  ];

  constructor(private config: AuthConfig) {
    if (!config.secretKey || config.secretKey.length < 32) {
      throw new Error('JWT secret key must be at least 32 characters long');
    }

    logger.info('JWT authenticator initialized', {
      component: 'JWTAuthenticator',
      issuer: config.issuer,
      audience: config.audience,
      tokenExpiry: config.tokenExpiry,
    });
  }

  /**
   * Generate a JWT token for project authentication
   */
  async generateToken(request: AuthRequest): Promise<AuthResponse> {
    try {
      // Validate project credentials
      if (!this.validateProjectCredentials(request.projectId, request.secretKey)) {
        throw new Error('Invalid project credentials');
      }

      const now = Math.floor(Date.now() / 1000);
      const payload: TokenPayload = {
        projectId: request.projectId,
        sessionId: request.sessionId,
        permissions: this.getProjectPermissions(request.projectId),
        iat: now,
        exp: now + this.parseExpiry(this.config.tokenExpiry),
        iss: this.config.issuer,
        aud: this.config.audience,
      };

      const token = jwt.sign(payload, this.config.secretKey, {
        algorithm: 'HS256',
      });

      const expiresAt = new Date(payload.exp * 1000);

      logger.info('JWT token generated', {
        component: 'JWTAuthenticator',
        projectId: request.projectId,
        sessionId: request.sessionId,
        expiresAt: expiresAt.toISOString(),
        permissions: payload.permissions,
      });

      return {
        token,
        expiresAt,
        permissions: payload.permissions,
      };
    } catch (error) {
      logger.error('Failed to generate JWT token', error as Error, {
        component: 'JWTAuthenticator',
        projectId: request.projectId,
      });
      throw new Error('Authentication failed');
    }
  }

  /**
   * Verify and decode a JWT token
   */
  async verifyToken(token: string): Promise<TokenPayload> {
    try {
      const decoded = jwt.verify(token, this.config.secretKey, {
        algorithms: ['HS256'],
        issuer: this.config.issuer,
        audience: this.config.audience,
      }) as TokenPayload;

      logger.debug('JWT token verified', {
        component: 'JWTAuthenticator',
        projectId: decoded.projectId,
        sessionId: decoded.sessionId,
        expiresAt: new Date(decoded.exp * 1000).toISOString(),
      });

      return decoded;
    } catch (error) {
      if (error instanceof jwt.TokenExpiredError) {
        logger.warn('JWT token expired', {
          component: 'JWTAuthenticator',
          expiredAt: error.expiredAt?.toISOString(),
        });
        throw new Error('Token expired');
      }

      if (error instanceof jwt.JsonWebTokenError) {
        logger.warn('Invalid JWT token', {
          component: 'JWTAuthenticator',
          error: error.message,
        });
        throw new Error('Invalid token');
      }

      logger.error('JWT verification failed', error as Error, {
        component: 'JWTAuthenticator',
      });
      throw new Error('Token verification failed');
    }
  }

  /**
   * Check if a token has specific permission
   */
  hasPermission(payload: TokenPayload, permission: string): boolean {
    return payload.permissions.includes(permission);
  }

  /**
   * Validate project credentials (in production, this would check against a database)
   */
  private validateProjectCredentials(projectId: string, secretKey: string): boolean {
    // For now, simple validation - in production, verify against secure storage
    if (!projectId || !secretKey) {
      return false;
    }

    // Basic format validation
    if (projectId.length < 3 || secretKey.length < 16) {
      return false;
    }

    // In production, you would:
    // 1. Hash the secret key and compare with stored hash
    // 2. Check if project exists and is active
    // 3. Validate rate limiting, IP restrictions, etc.

    return true;
  }

  /**
   * Get permissions for a specific project
   */
  private getProjectPermissions(projectId: string): string[] {
    // In production, this would be fetched from a database based on project settings
    // For now, return default permissions for all projects
    return [...this.defaultPermissions];
  }

  /**
   * Parse expiry string to seconds
   */
  private parseExpiry(expiry: string): number {
    const unit = expiry.slice(-1);
    const value = Number.parseInt(expiry.slice(0, -1), 10);

    if (Number.isNaN(value)) {
      throw new Error(`Invalid expiry format: ${expiry}`);
    }

    switch (unit) {
      case 's':
        return value;
      case 'm':
        return value * 60;
      case 'h':
        return value * 60 * 60;
      case 'd':
        return value * 24 * 60 * 60;
      default:
        throw new Error(`Invalid expiry unit: ${unit}. Use s, m, h, or d`);
    }
  }

  /**
   * Generate a secure secret key for project setup
   */
  static generateSecretKey(): string {
    return randomBytes(32).toString('hex');
  }

  /**
   * Create default auth configuration
   */
  static createDefaultConfig(): AuthConfig {
    return {
      secretKey: process.env.JWT_SECRET || JWTAuthenticator.generateSecretKey(),
      tokenExpiry: process.env.JWT_EXPIRY || '24h',
      issuer: process.env.JWT_ISSUER || 'codeflow-buddy',
      audience: process.env.JWT_AUDIENCE || 'codeflow-clients',
    };
  }
}
