/**
 * Common Types and Models
 *
 * Shared types used across the API.
 */

import "@typespec/http";

using TypeSpec.Http;

namespace AdiFamily;

/**
 * Standard error response
 */
@error
model ApiError {
  @statusCode statusCode: int32;
  code: string;
  message: string;
  details?: Record<string>;
}

/**
 * Pagination parameters
 */
model PaginationParams {
  @query page?: int32 = 1;
  @query limit?: int32 = 20;
  @query sort?: string;
  @query order?: "asc" | "desc" = "desc";
}

/**
 * Paginated response wrapper
 */
model PaginatedResponse<T> {
  items: T[];
  total: int32;
  page: int32;
  limit: int32;
  hasMore: boolean;
}

/**
 * Timestamp fields for entities
 */
model Timestamps {
  createdAt: utcDateTime;
  updatedAt: utcDateTime;
}

/**
 * UUID identifier type
 */
@format("uuid")
scalar uuid extends string;

/**
 * Email address type
 */
@format("email")
scalar email extends string;

/**
 * URL type
 */
@format("uri")
scalar url extends string;

/**
 * Standard success response
 */
model SuccessResponse {
  success: boolean;
  message?: string;
}

/**
 * Health check response
 */
model HealthResponse {
  status: "healthy" | "degraded" | "unhealthy";
  version: string;
  uptime: int64;
  services: Record<ServiceHealth>;
}

model ServiceHealth {
  status: "up" | "down" | "unknown";
  latency?: int32;
  message?: string;
}
