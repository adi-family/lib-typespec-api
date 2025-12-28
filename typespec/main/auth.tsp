/**
 * Authentication & Authorization API
 *
 * Handles user authentication, sessions, and tokens.
 */

import "@typespec/http";
import "@typespec/rest";

using TypeSpec.Http;
using TypeSpec.Rest;

namespace AdiFamily.Auth;

/**
 * User model
 */
model User {
  id: uuid;
  email: email;
  name?: string;
  avatarUrl?: url;
  emailVerified: boolean;
  ...Timestamps;
}

/**
 * Session model
 */
model Session {
  id: uuid;
  userId: uuid;
  expiresAt: utcDateTime;
  userAgent?: string;
  ipAddress?: string;
  ...Timestamps;
}

/**
 * Authentication token response
 */
model TokenResponse {
  accessToken: string;
  tokenType: "Bearer";
  expiresIn: int32;
  refreshToken?: string;
}

/**
 * Email login request
 */
model EmailLoginRequest {
  @minLength(1)
  email: email;
}

/**
 * Email verification request
 */
model VerifyEmailRequest {
  email: email;
  @minLength(6)
  @maxLength(6)
  code: string;
}

/**
 * Token refresh request
 */
model RefreshTokenRequest {
  refreshToken: string;
}

/**
 * Authentication routes
 */
@route("/auth")
@tag("Authentication")
interface AuthService {
  /**
   * Request email login code
   */
  @post
  @route("/login/email")
  loginWithEmail(@body body: EmailLoginRequest): {
    @statusCode statusCode: 200;
    @body body: SuccessResponse;
  } | ApiError;

  /**
   * Verify email code and get tokens
   */
  @post
  @route("/verify")
  verifyEmail(@body body: VerifyEmailRequest): {
    @statusCode statusCode: 200;
    @body body: TokenResponse;
  } | ApiError;

  /**
   * Refresh access token
   */
  @post
  @route("/refresh")
  refreshToken(@body body: RefreshTokenRequest): {
    @statusCode statusCode: 200;
    @body body: TokenResponse;
  } | ApiError;

  /**
   * Logout and invalidate session
   */
  @post
  @route("/logout")
  logout(): {
    @statusCode statusCode: 200;
    @body body: SuccessResponse;
  } | ApiError;

  /**
   * Get current user
   */
  @get
  @route("/me")
  getCurrentUser(): {
    @statusCode statusCode: 200;
    @body body: User;
  } | ApiError;
}
