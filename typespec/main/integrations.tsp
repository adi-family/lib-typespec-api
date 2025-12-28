/**
 * Integrations API
 *
 * Third-party service integrations (GitHub, Linear, Slack, etc.)
 */

import "@typespec/http";
import "@typespec/rest";

using TypeSpec.Http;
using TypeSpec.Rest;

namespace AdiFamily.Integrations;

/**
 * Integration provider types
 */
enum IntegrationProvider {
  github,
  gitlab,
  linear,
  jira,
  slack,
  discord,
  notion,
  confluence,
  asana,
  trello,
}

/**
 * Integration status
 */
enum IntegrationStatus {
  connected,
  disconnected,
  error,
  pending,
}

/**
 * Integration model
 */
model Integration {
  id: uuid;
  provider: IntegrationProvider;
  name: string;
  status: IntegrationStatus;
  accountId?: string;
  accountName?: string;
  scopes: string[];
  metadata?: Record<string>;
  lastSyncAt?: utcDateTime;
  errorMessage?: string;
  ...Timestamps;
}

/**
 * OAuth authorization request
 */
model OAuthAuthorizeRequest {
  provider: IntegrationProvider;
  redirectUri: url;
  scopes?: string[];
  state?: string;
}

/**
 * OAuth authorization response
 */
model OAuthAuthorizeResponse {
  authorizationUrl: url;
  state: string;
}

/**
 * OAuth callback request
 */
model OAuthCallbackRequest {
  provider: IntegrationProvider;
  code: string;
  state: string;
}

/**
 * Sync status
 */
model SyncStatus {
  integrationId: uuid;
  status: "idle" | "syncing" | "completed" | "failed";
  progress?: int32;
  itemsSynced?: int32;
  totalItems?: int32;
  startedAt?: utcDateTime;
  completedAt?: utcDateTime;
  error?: string;
}

/**
 * Webhook event
 */
model WebhookEvent {
  id: uuid;
  integrationId: uuid;
  provider: IntegrationProvider;
  eventType: string;
  payload: Record<unknown>;
  processedAt?: utcDateTime;
  error?: string;
  ...Timestamps;
}

/**
 * Integrations API
 */
@route("/integrations")
@tag("Integrations")
interface IntegrationService {
  /**
   * List all integrations
   */
  @get
  list(...PaginationParams): {
    @statusCode statusCode: 200;
    @body body: PaginatedResponse<Integration>;
  } | ApiError;

  /**
   * Get an integration by ID
   */
  @get
  @route("/{id}")
  get(@path id: uuid): {
    @statusCode statusCode: 200;
    @body body: Integration;
  } | ApiError;

  /**
   * Delete/disconnect an integration
   */
  @delete
  @route("/{id}")
  delete(@path id: uuid): {
    @statusCode statusCode: 204;
  } | ApiError;

  /**
   * Initiate OAuth flow
   */
  @post
  @route("/oauth/authorize")
  authorize(@body body: OAuthAuthorizeRequest): {
    @statusCode statusCode: 200;
    @body body: OAuthAuthorizeResponse;
  } | ApiError;

  /**
   * Handle OAuth callback
   */
  @post
  @route("/oauth/callback")
  callback(@body body: OAuthCallbackRequest): {
    @statusCode statusCode: 200;
    @body body: Integration;
  } | ApiError;

  /**
   * Trigger sync for an integration
   */
  @post
  @route("/{id}/sync")
  sync(@path id: uuid): {
    @statusCode statusCode: 200;
    @body body: SyncStatus;
  } | ApiError;

  /**
   * Get sync status
   */
  @get
  @route("/{id}/sync/status")
  getSyncStatus(@path id: uuid): {
    @statusCode statusCode: 200;
    @body body: SyncStatus;
  } | ApiError;

  /**
   * List webhook events
   */
  @get
  @route("/{id}/webhooks")
  listWebhooks(@path id: uuid, ...PaginationParams): {
    @statusCode statusCode: 200;
    @body body: PaginatedResponse<WebhookEvent>;
  } | ApiError;
}

/**
 * Provider-specific operations
 */
@route("/integrations/github")
@tag("GitHub")
interface GitHubService {
  /**
   * List repositories
   */
  @get
  @route("/repos")
  listRepos(...PaginationParams): {
    @statusCode statusCode: 200;
    @body body: PaginatedResponse<GitHubRepo>;
  } | ApiError;

  /**
   * List issues for a repository
   */
  @get
  @route("/repos/{owner}/{repo}/issues")
  listIssues(
    @path owner: string,
    @path repo: string,
    ...PaginationParams
  ): {
    @statusCode statusCode: 200;
    @body body: PaginatedResponse<GitHubIssue>;
  } | ApiError;

  /**
   * Create an issue
   */
  @post
  @route("/repos/{owner}/{repo}/issues")
  createIssue(
    @path owner: string,
    @path repo: string,
    @body body: CreateGitHubIssueRequest
  ): {
    @statusCode statusCode: 201;
    @body body: GitHubIssue;
  } | ApiError;
}

/**
 * GitHub repository model
 */
model GitHubRepo {
  id: int64;
  name: string;
  fullName: string;
  description?: string;
  private: boolean;
  htmlUrl: url;
  defaultBranch: string;
  stargazersCount: int32;
  forksCount: int32;
  openIssuesCount: int32;
  ...Timestamps;
}

/**
 * GitHub issue model
 */
model GitHubIssue {
  id: int64;
  number: int32;
  title: string;
  body?: string;
  state: "open" | "closed";
  htmlUrl: url;
  labels: string[];
  assignees: string[];
  ...Timestamps;
}

/**
 * Create GitHub issue request
 */
model CreateGitHubIssueRequest {
  @minLength(1)
  title: string;
  body?: string;
  labels?: string[];
  assignees?: string[];
}
