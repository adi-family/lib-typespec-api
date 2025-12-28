/**
 * ADI Family API - TypeSpec Definition
 *
 * This is the main entry point for the API schema.
 * All models, operations, and services are defined here using TypeSpec.
 */

import "@typespec/http";
import "@typespec/rest";
import "@typespec/openapi";
import "@typespec/versioning";

using TypeSpec.Http;
using TypeSpec.Rest;
using TypeSpec.Versioning;

/**
 * ADI Family API Service
 *
 * A comprehensive API for the ADI platform providing
 * task management, agent orchestration, and integrations.
 */
@service({
  title: "ADI Family API",
})
@server("https://api.adi.family", "Production server")
@versioned(Versions)
namespace AdiFamily;

/**
 * API Versions
 */
enum Versions {
  @useDependency(TypeSpec.Versioning.Versions.v1)
  v1: "v1",
}

// Import sub-modules
import "./common.tsp";
import "./auth.tsp";
import "./tasks.tsp";
import "./agents.tsp";
import "./integrations.tsp";
