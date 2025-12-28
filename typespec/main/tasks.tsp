/**
 * Tasks API
 *
 * Task and project management operations.
 */

import "@typespec/http";
import "@typespec/rest";

using TypeSpec.Http;
using TypeSpec.Rest;

namespace AdiFamily.Tasks;

/**
 * Task priority levels
 */
enum TaskPriority {
  low,
  medium,
  high,
  urgent,
}

/**
 * Task status values
 */
enum TaskStatus {
  pending,
  inProgress: "in_progress",
  completed,
  cancelled,
  blocked,
}

/**
 * Project model
 */
model Project {
  id: uuid;
  name: string;
  description?: string;
  color?: string;
  icon?: string;
  ownerId: uuid;
  isArchived: boolean;
  taskCount: int32;
  ...Timestamps;
}

/**
 * Task model
 */
model Task {
  id: uuid;
  title: string;
  description?: string;
  status: TaskStatus;
  priority: TaskPriority;
  projectId?: uuid;
  assigneeId?: uuid;
  parentId?: uuid;
  dueDate?: utcDateTime;
  completedAt?: utcDateTime;
  labels: string[];
  subtaskCount: int32;
  completedSubtaskCount: int32;
  ...Timestamps;
}

/**
 * Create project request
 */
model CreateProjectRequest {
  @minLength(1)
  @maxLength(100)
  name: string;

  @maxLength(500)
  description?: string;

  color?: string;
  icon?: string;
}

/**
 * Update project request
 */
model UpdateProjectRequest {
  @maxLength(100)
  name?: string;

  @maxLength(500)
  description?: string;

  color?: string;
  icon?: string;
  isArchived?: boolean;
}

/**
 * Create task request
 */
model CreateTaskRequest {
  @minLength(1)
  @maxLength(200)
  title: string;

  @maxLength(5000)
  description?: string;

  status?: TaskStatus = TaskStatus.pending;
  priority?: TaskPriority = TaskPriority.medium;
  projectId?: uuid;
  assigneeId?: uuid;
  parentId?: uuid;
  dueDate?: utcDateTime;
  labels?: string[];
}

/**
 * Update task request
 */
model UpdateTaskRequest {
  @maxLength(200)
  title?: string;

  @maxLength(5000)
  description?: string;

  status?: TaskStatus;
  priority?: TaskPriority;
  projectId?: uuid;
  assigneeId?: uuid;
  dueDate?: utcDateTime;
  labels?: string[];
}

/**
 * Task filter parameters
 */
model TaskFilterParams {
  ...PaginationParams;
  @query status?: TaskStatus;
  @query priority?: TaskPriority;
  @query projectId?: uuid;
  @query assigneeId?: uuid;
  @query label?: string;
  @query search?: string;
  @query dueBefore?: utcDateTime;
  @query dueAfter?: utcDateTime;
}

/**
 * Projects API
 */
@route("/projects")
@tag("Projects")
interface ProjectService {
  /**
   * List all projects
   */
  @get
  list(...PaginationParams): {
    @statusCode statusCode: 200;
    @body body: PaginatedResponse<Project>;
  } | ApiError;

  /**
   * Create a new project
   */
  @post
  create(@body body: CreateProjectRequest): {
    @statusCode statusCode: 201;
    @body body: Project;
  } | ApiError;

  /**
   * Get a project by ID
   */
  @get
  @route("/{id}")
  get(@path id: uuid): {
    @statusCode statusCode: 200;
    @body body: Project;
  } | ApiError;

  /**
   * Update a project
   */
  @patch
  @route("/{id}")
  update(@path id: uuid, @body body: UpdateProjectRequest): {
    @statusCode statusCode: 200;
    @body body: Project;
  } | ApiError;

  /**
   * Delete a project
   */
  @delete
  @route("/{id}")
  delete(@path id: uuid): {
    @statusCode statusCode: 204;
  } | ApiError;

  /**
   * List tasks in a project
   */
  @get
  @route("/{id}/tasks")
  listTasks(@path id: uuid, ...TaskFilterParams): {
    @statusCode statusCode: 200;
    @body body: PaginatedResponse<Task>;
  } | ApiError;
}

/**
 * Tasks API
 */
@route("/tasks")
@tag("Tasks")
interface TaskService {
  /**
   * List all tasks
   */
  @get
  list(...TaskFilterParams): {
    @statusCode statusCode: 200;
    @body body: PaginatedResponse<Task>;
  } | ApiError;

  /**
   * Create a new task
   */
  @post
  create(@body body: CreateTaskRequest): {
    @statusCode statusCode: 201;
    @body body: Task;
  } | ApiError;

  /**
   * Get a task by ID
   */
  @get
  @route("/{id}")
  get(@path id: uuid): {
    @statusCode statusCode: 200;
    @body body: Task;
  } | ApiError;

  /**
   * Update a task
   */
  @patch
  @route("/{id}")
  update(@path id: uuid, @body body: UpdateTaskRequest): {
    @statusCode statusCode: 200;
    @body body: Task;
  } | ApiError;

  /**
   * Delete a task
   */
  @delete
  @route("/{id}")
  delete(@path id: uuid): {
    @statusCode statusCode: 204;
  } | ApiError;

  /**
   * List subtasks
   */
  @get
  @route("/{id}/subtasks")
  listSubtasks(@path id: uuid, ...PaginationParams): {
    @statusCode statusCode: 200;
    @body body: PaginatedResponse<Task>;
  } | ApiError;

  /**
   * Complete a task
   */
  @post
  @route("/{id}/complete")
  complete(@path id: uuid): {
    @statusCode statusCode: 200;
    @body body: Task;
  } | ApiError;

  /**
   * Reopen a task
   */
  @post
  @route("/{id}/reopen")
  reopen(@path id: uuid): {
    @statusCode statusCode: 200;
    @body body: Task;
  } | ApiError;
}
