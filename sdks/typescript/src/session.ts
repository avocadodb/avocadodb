/**
 * Session Management for AvocadoDB TypeScript SDK
 *
 * Provides conversation session management with:
 * - Multi-turn conversation tracking
 * - Context compilation in session context
 * - Conversation history retrieval
 * - Session replay for debugging
 *
 * @example
 * ```typescript
 * const db = new AvocadoDB('http://localhost:8765');
 *
 * // Create a session
 * const session = await db.createSession({
 *   userId: 'alice',
 *   title: 'Project Q&A'
 * });
 *
 * // Multi-turn conversation
 * const result = await session.compile('What is AvocadoDB?');
 * await session.addMessage('assistant', 'AvocadoDB is...');
 *
 * // Get conversation history
 * const history = await session.getHistory();
 *
 * // Replay for debugging
 * const replay = await session.replay();
 * ```
 */

import { WorkingSet } from './client';

/**
 * Message role in a conversation
 */
export type MessageRole = 'user' | 'assistant' | 'system' | 'tool';

/**
 * A message in a conversation session
 */
export interface Message {
  /** Message ID */
  id: string;

  /** Session ID */
  sessionId: string;

  /** Message role */
  role: MessageRole;

  /** Message content */
  content: string;

  /** Sequence number (0-indexed) */
  sequenceNumber: number;

  /** Creation timestamp */
  createdAt: string;

  /** Optional metadata */
  metadata?: Record<string, any>;
}

/**
 * Session metadata
 */
export interface SessionInfo {
  /** Session ID */
  id: string;

  /** Optional user identifier */
  userId?: string;

  /** Optional session title */
  title?: string;

  /** Creation timestamp */
  createdAt: string;

  /** Last update timestamp */
  updatedAt: string;

  /** Last message timestamp */
  lastMessageAt?: string;

  /** Optional metadata */
  metadata?: Record<string, any>;
}

/**
 * A conversation turn (user query + optional assistant response)
 */
export interface SessionTurn {
  /** User message */
  userMessage: Message;

  /** Compiled context (if any) */
  workingSet?: WorkingSet;

  /** Assistant response (if any) */
  assistantMessage?: Message;
}

/**
 * Session replay data for debugging
 */
export interface SessionReplay {
  /** Session metadata */
  session: SessionInfo;

  /** Conversation turns */
  turns: SessionTurn[];
}

/**
 * Options for creating a session
 */
export interface CreateSessionOptions {
  /** Optional user identifier */
  userId?: string;

  /** Optional session title */
  title?: string;

  /** Optional metadata */
  metadata?: Record<string, any>;
}

/**
 * Options for compiling in session context
 */
export interface SessionCompileOptions {
  /** Token budget (default: 8000) */
  budget?: number;

  /** Optional metadata */
  metadata?: Record<string, any>;
}

/**
 * Result of session compilation
 */
export interface SessionCompileResult {
  /** User message that was added */
  message: Message;

  /** Compiled context */
  workingSet: WorkingSet;
}

/**
 * Session class for managing conversations
 */
export class Session {
  private baseUrl: string;
  private project: string;

  /** Session ID */
  readonly id: string;

  /** User ID (if set) */
  userId?: string;

  /** Session title (if set) */
  title?: string;

  /** Creation timestamp */
  readonly createdAt: string;

  /** Last update timestamp */
  updatedAt: string;

  constructor(baseUrl: string, project: string, data: SessionInfo) {
    this.baseUrl = baseUrl;
    this.project = project;
    this.id = data.id;
    this.userId = data.userId;
    this.title = data.title;
    this.createdAt = data.createdAt;
    this.updatedAt = data.updatedAt;
  }

  /**
   * Add a message to the session
   *
   * @param role - Message role
   * @param content - Message content
   * @param metadata - Optional metadata
   * @returns The created message
   */
  async addMessage(
    role: MessageRole,
    content: string,
    metadata?: Record<string, any>
  ): Promise<Message> {
    const response = await fetch(
      `${this.baseUrl}/sessions/${this.id}/messages`,
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          role,
          content,
          metadata,
          project: this.project,
        }),
      }
    );

    if (!response.ok) {
      throw new Error(`Failed to add message: ${response.statusText}`);
    }

    const data = (await response.json()) as { message: any };
    return this.parseMessage(data.message);
  }

  /**
   * Compile context for a query in session context
   *
   * This automatically adds a user message and compiles context.
   *
   * @param query - User query
   * @param options - Compilation options
   * @returns Compilation result
   */
  async compile(
    query: string,
    options: SessionCompileOptions = {}
  ): Promise<SessionCompileResult> {
    const response = await fetch(
      `${this.baseUrl}/sessions/${this.id}/compile`,
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          query,
          token_budget: options.budget || 8000,
          metadata: options.metadata,
          project: this.project,
        }),
      }
    );

    if (!response.ok) {
      throw new Error(`Failed to compile: ${response.statusText}`);
    }

    const data = (await response.json()) as { message: any; working_set: any };

    return {
      message: this.parseMessage(data.message),
      workingSet: new WorkingSet(data.working_set),
    };
  }

  /**
   * Get formatted conversation history
   *
   * @param maxTokens - Optional token limit (keeps recent messages)
   * @returns Formatted conversation history
   */
  async getHistory(maxTokens?: number): Promise<string> {
    const url = new URL(`${this.baseUrl}/sessions/${this.id}/history`);
    url.searchParams.append('project', this.project);

    if (maxTokens) {
      url.searchParams.append('max_tokens', maxTokens.toString());
    }

    const response = await fetch(url.toString());

    if (!response.ok) {
      throw new Error(`Failed to get history: ${response.statusText}`);
    }

    const data = (await response.json()) as { history: string };
    return data.history;
  }

  /**
   * Replay session for debugging
   *
   * Returns structured data about each conversation turn,
   * including compiled context and citations.
   *
   * @returns Session replay data
   */
  async replay(): Promise<SessionReplay> {
    const url = new URL(`${this.baseUrl}/sessions/${this.id}/replay`);
    url.searchParams.append('project', this.project);

    const response = await fetch(url.toString());

    if (!response.ok) {
      throw new Error(`Failed to replay session: ${response.statusText}`);
    }

    const data = (await response.json()) as { session: any; turns: any[] };

    return {
      session: this.parseSessionInfo(data.session),
      turns: data.turns.map((turn: any) => ({
        userMessage: this.parseMessage(turn.user_message),
        workingSet: turn.working_set
          ? new WorkingSet(turn.working_set)
          : undefined,
        assistantMessage: turn.assistant_message
          ? this.parseMessage(turn.assistant_message)
          : undefined,
      })),
    };
  }

  /**
   * Delete this session
   */
  async delete(): Promise<void> {
    const url = new URL(`${this.baseUrl}/sessions/${this.id}`);
    url.searchParams.append('project', this.project);

    const response = await fetch(url.toString(), {
      method: 'DELETE',
    });

    if (!response.ok) {
      throw new Error(`Failed to delete session: ${response.statusText}`);
    }
  }

  /**
   * Refresh session data from server
   */
  async refresh(): Promise<void> {
    const url = new URL(`${this.baseUrl}/sessions/${this.id}`);
    url.searchParams.append('project', this.project);

    const response = await fetch(url.toString());

    if (!response.ok) {
      throw new Error(`Failed to refresh session: ${response.statusText}`);
    }

    const data = (await response.json()) as { session: any };
    const sessionData = this.parseSessionInfo(data.session);

    this.userId = sessionData.userId;
    this.title = sessionData.title;
    this.updatedAt = sessionData.updatedAt;
  }

  /**
   * Get a string representation of the session
   */
  toString(): string {
    return `Session(id=${this.id}, user=${this.userId || 'N/A'}, title=${this.title || 'N/A'})`;
  }

  private parseMessage(data: any): Message {
    return {
      id: data.id,
      sessionId: data.session_id,
      role: data.role,
      content: data.content,
      sequenceNumber: data.sequence_number,
      createdAt: data.created_at,
      metadata: data.metadata,
    };
  }

  private parseSessionInfo(data: any): SessionInfo {
    return {
      id: data.id,
      userId: data.user_id,
      title: data.title,
      createdAt: data.created_at,
      updatedAt: data.updated_at,
      lastMessageAt: data.last_message_at,
      metadata: data.metadata,
    };
  }
}

/**
 * Session management methods for AvocadoDB client
 *
 * These methods should be added to the main AvocadoDB class.
 */
export class SessionManager {
  constructor(private baseUrl: string, private project: string) {}

  /**
   * Create a new session
   *
   * @param options - Session options
   * @returns The created session
   */
  async createSession(options: CreateSessionOptions = {}): Promise<Session> {
    const response = await fetch(`${this.baseUrl}/sessions`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        user_id: options.userId,
        title: options.title,
        metadata: options.metadata,
        project: this.project,
      }),
    });

    if (!response.ok) {
      throw new Error(`Failed to create session: ${response.statusText}`);
    }

    const data = (await response.json()) as { session: any };
    return new Session(this.baseUrl, this.project, data.session);
  }

  /**
   * List sessions
   *
   * @param userId - Optional user ID filter
   * @param limit - Maximum number of sessions to return
   * @returns Array of session info
   */
  async listSessions(
    userId?: string,
    limit: number = 50
  ): Promise<SessionInfo[]> {
    const url = new URL(`${this.baseUrl}/sessions`);
    url.searchParams.append('project', this.project);

    if (userId) {
      url.searchParams.append('user_id', userId);
    }

    url.searchParams.append('limit', limit.toString());

    const response = await fetch(url.toString());

    if (!response.ok) {
      throw new Error(`Failed to list sessions: ${response.statusText}`);
    }

    const data = (await response.json()) as { sessions: any[] };
    return data.sessions.map((s: any) => ({
      id: s.id,
      userId: s.user_id,
      title: s.title,
      createdAt: s.created_at,
      updatedAt: s.updated_at,
      lastMessageAt: s.last_message_at,
      metadata: s.metadata,
    }));
  }

  /**
   * Get a session by ID
   *
   * @param sessionId - Session ID
   * @returns The session
   */
  async getSession(sessionId: string): Promise<Session> {
    const url = new URL(`${this.baseUrl}/sessions/${sessionId}`);
    url.searchParams.append('project', this.project);

    const response = await fetch(url.toString());

    if (!response.ok) {
      throw new Error(`Failed to get session: ${response.statusText}`);
    }

    const data = (await response.json()) as { session: any };
    return new Session(this.baseUrl, this.project, data.session);
  }
}
