# lib-typespec-api

Pure Rust TypeSpec parser and multi-language code generator.

## Features

- **Native TypeSpec parsing** - No npm/Node.js required
- **Multi-language output** - Python, TypeScript, Rust
- **OpenAPI 3.0 export** - Generate JSON and YAML specs
- **Client & Server** - Generate both sides from one schema

## Usage

### CLI

```bash
# Generate Python client + server
tsp-gen typespec/main/*.tsp -l python -o generated -s both

# Generate TypeScript client only
tsp-gen typespec/main/*.tsp -l typescript -o generated -s client

# Generate Rust server only
tsp-gen typespec/main/*.tsp -l rust -o generated -s server

# Generate OpenAPI 3.0 spec (JSON + YAML)
tsp-gen typespec/main/*.tsp -l openapi -o generated -p "My API"
```

### As Library

```rust
use typespec_api::{parse, codegen::{Generator, Language, Side}};

let source = std::fs::read_to_string("api.tsp")?;
let file = parse(&source)?;

let generator = Generator::new(&file, Path::new("generated"), "my_api");
generator.generate(Language::Rust, Side::Both)?;
```

## TypeSpec Syntax Support

```tsp
import "@typespec/http";

using TypeSpec.Http;

@route("/users")
interface UserService {
    @get
    list(): User[];

    @get
    @route("/{id}")
    get(@path id: string): User;

    @post
    create(@body body: CreateUserRequest): User;
}

model User {
    id: string;
    name: string;
    email?: string;
}

enum Status {
    active,
    inactive,
}
```

## Generated Code

### Python

```python
from api.client import Client

async with Client("https://api.example.com", access_token="...") as client:
    users = await client.user_service.list()
    user = await client.user_service.create(body=CreateUserRequest(
        name="Alice",
        email="alice@example.com"
    ))
```

### TypeScript

```typescript
import { Client } from './api';

const client = new Client({ baseUrl: 'https://api.example.com', accessToken: '...' });
const users = await client.userService.list();
const user = await client.userService.create({ name: 'Alice', email: 'alice@example.com' });
```

### Rust

```rust
use api::client::{BaseClient, UserServiceClient};

let client = BaseClient::new("https://api.example.com").with_token("...");
let user_client = UserServiceClient::new(&client);

let users = user_client.list().await?;
let user = user_client.create(&CreateUserRequest {
    name: "Alice".to_string(),
    email: Some("alice@example.com".to_string()),
}).await?;
```

## Building

```bash
cargo build -p lib-typespec-api
```

## License

MIT
