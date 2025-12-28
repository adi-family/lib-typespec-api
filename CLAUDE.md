lib-typespec-api, typespec, codegen, parser, rust, python, typescript, openapi

## Overview
- Pure Rust TypeSpec parser (no npm/Node.js)
- Generates Python, TypeScript, Rust code
- Generates OpenAPI 3.0 specs (JSON + YAML)
- Supports client and server generation

## Structure
- `src/lexer.rs` - Logos-based tokenizer
- `src/parser.rs` - Recursive descent parser
- `src/ast.rs` - Abstract syntax tree types
- `src/codegen/` - Language-specific generators
- `src/bin/generate.rs` - CLI binary
- `typespec/main/` - Example TypeSpec definitions

## CLI Usage
```bash
tsp-gen input.tsp -l python -o out -s both
tsp-gen input.tsp -l typescript -o out -s client
tsp-gen input.tsp -l rust -o out -s server
tsp-gen input.tsp -l openapi -o out          # generates openapi.json + openapi.yaml
```

## Supported TypeSpec Features
- `import`, `using`, `namespace`
- `model`, `enum`, `union`, `interface`, `scalar`, `alias`
- Decorators: `@route`, `@get`, `@post`, `@put`, `@patch`, `@delete`
- Parameter decorators: `@path`, `@query`, `@body`
- Type params: `Model<T>`, arrays: `Type[]`, optional: `field?`
- Spread operator: `...BaseModel`

## Adding New Language
1. Create `src/codegen/{lang}.rs`
2. Implement `generate()` function
3. Add to `src/codegen/mod.rs`
4. Add `Language` variant and match arm
