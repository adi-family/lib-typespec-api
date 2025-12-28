/**
 * Agents API
 *
 * AI agent management and orchestration.
 */

import "@typespec/http";
import "@typespec/rest";

using TypeSpec.Http;
using TypeSpec.Rest;

namespace AdiFamily.Agents;

/**
 * Agent status values
 */
enum AgentStatus {
  idle,
  running,
  paused,
  stopped,
  error,
}

/**
 * Agent run status
 */
enum RunStatus {
  pending,
  running,
  completed,
  failed,
  cancelled,
  timeout,
}

/**
 * Agent configuration
 */
model AgentConfig {
  model: string;
  temperature?: float32;
  maxTokens?: int32;
  systemPrompt?: string;
  tools?: string[];
  timeout?: int32;
}

/**
 * Agent model
 */
model Agent {
  id: uuid;
  name: string;
  description?: string;
  status: AgentStatus;
  config: AgentConfig;
  ownerId: uuid;
  lastRunAt?: utcDateTime;
  runCount: int32;
  ...Timestamps;
}

/**
 * Agent run model
 */
model AgentRun {
  id: uuid;
  agentId: uuid;
  status: RunStatus;
  input: string;
  output?: string;
  error?: string;
  tokensUsed?: int32;
  durationMs?: int32;
  startedAt: utcDateTime;
  completedAt?: utcDateTime;
  ...Timestamps;
}

/**
 * Agent run message
 */
model RunMessage {
  role: "user" | "assistant" | "system" | "tool";
  content: string;
  toolCallId?: string;
  toolName?: string;
  timestamp: utcDateTime;
}

/**
 * Create agent request
 */
model CreateAgentRequest {
  @minLength(1)
  @maxLength(100)
  name: string;

  @maxLength(500)
  description?: string;

  config: AgentConfig;
}

/**
 * Update agent request
 */
model UpdateAgentRequest {
  @maxLength(100)
  name?: string;

  @maxLength(500)
  description?: string;

  config?: AgentConfig;
}

/**
 * Run agent request
 */
model RunAgentRequest {
  @minLength(1)
  input: string;

  context?: Record<string>;
  stream?: boolean = false;
}

/**
 * Agents API
 */
@route("/agents")
@tag("Agents")
interface AgentService {
  /**
   * List all agents
   */
  @get
  list(...PaginationParams): {
    @statusCode statusCode: 200;
    @body body: PaginatedResponse<Agent>;
  } | ApiError;

  /**
   * Create a new agent
   */
  @post
  create(@body body: CreateAgentRequest): {
    @statusCode statusCode: 201;
    @body body: Agent;
  } | ApiError;

  /**
   * Get an agent by ID
   */
  @get
  @route("/{id}")
  get(@path id: uuid): {
    @statusCode statusCode: 200;
    @body body: Agent;
  } | ApiError;

  /**
   * Update an agent
   */
  @patch
  @route("/{id}")
  update(@path id: uuid, @body body: UpdateAgentRequest): {
    @statusCode statusCode: 200;
    @body body: Agent;
  } | ApiError;

  /**
   * Delete an agent
   */
  @delete
  @route("/{id}")
  delete(@path id: uuid): {
    @statusCode statusCode: 204;
  } | ApiError;

  /**
   * Run an agent
   */
  @post
  @route("/{id}/run")
  run(@path id: uuid, @body body: RunAgentRequest): {
    @statusCode statusCode: 200;
    @body body: AgentRun;
  } | ApiError;

  /**
   * List agent runs
   */
  @get
  @route("/{id}/runs")
  listRuns(@path id: uuid, ...PaginationParams): {
    @statusCode statusCode: 200;
    @body body: PaginatedResponse<AgentRun>;
  } | ApiError;

  /**
   * Get a specific run
   */
  @get
  @route("/{id}/runs/{runId}")
  getRun(@path id: uuid, @path runId: uuid): {
    @statusCode statusCode: 200;
    @body body: AgentRun;
  } | ApiError;

  /**
   * Get run messages
   */
  @get
  @route("/{id}/runs/{runId}/messages")
  getRunMessages(@path id: uuid, @path runId: uuid): {
    @statusCode statusCode: 200;
    @body body: RunMessage[];
  } | ApiError;

  /**
   * Cancel a running agent
   */
  @post
  @route("/{id}/runs/{runId}/cancel")
  cancelRun(@path id: uuid, @path runId: uuid): {
    @statusCode statusCode: 200;
    @body body: AgentRun;
  } | ApiError;
}
