# Contributing Guide

Thank you for your interest in contributing to MCP Connect! This guide will help you get started.

## Getting Started

1. **Fork the repository** on GitHub
2. **Clone your fork**:
   ```bash
   git clone https://github.com/yourusername/mcp-connect.git
   cd mcp-connect
   ```
3. **Add upstream remote**:
   ```bash
   git remote add upstream https://github.com/originalowner/mcp-connect.git
   ```

## Development Setup

### Prerequisites

- Rust 1.75 or later
- Cargo (latest stable)
- OpenSSL 3.x development libraries

### Build

```bash
# Clone and build
git clone https://github.com/yourusername/mcp-connect.git
cd mcp-connect
cargo build

# Run tests
cargo test

# Check code
cargo check --workspace
```

## Development Workflow

1. **Create a branch**:
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. **Make your changes**

3. **Run tests**:
   ```bash
   cargo test
   cargo clippy
   cargo fmt --check
   ```

4. **Commit your changes**:
   ```bash
   git add .
   git commit -m "Add feature: description"
   ```

5. **Push to your fork**:
   ```bash
   git push origin feature/your-feature-name
   ```

6. **Open a Pull Request** on GitHub

## Code Style

### Formatting

We use `rustfmt` for code formatting:

```bash
cargo fmt
```

### Linting

We use `clippy` for linting:

```bash
cargo clippy --workspace
```

### Code Standards

- Follow Rust naming conventions
- Use `anyhow::Result` for error handling in application code
- Use `thiserror` for library error types
- Add documentation comments for public APIs
- Write tests for new features

## Project Structure

```
crates/
├── mcp-types/      # Shared types and traits
├── mcp-server/     # Server implementation
├── mcp-client/     # Client implementation
├── mcp-proxy/      # Proxy and routing
├── mcp-registry/   # Registry client
├── mcp-config/     # Configuration management
└── mcp-connect/    # CLI application
```

## Adding New Features

### Adding a New Command

1. Add command to `Commands` enum in `main.rs`
2. Implement handler in `commands/` directory
3. Add tests
4. Update documentation

### Adding IDE Support

1. Add IDE type to `IdeType` enum in `generate.rs`
2. Implement `generate_<ide>_config` function
3. Update help text
4. Add documentation in `docs/ide-setup/`

### Adding Transport Support

1. Implement transport trait in `mcp-client/src/transport/`
2. Add to transport factory
3. Update configuration types
4. Add tests

## Testing

### Unit Tests

```bash
# Run all tests
cargo test

# Run tests for specific crate
cargo test -p mcp-client

# Run with output
cargo test -- --nocapture
```

### Integration Tests

```bash
# Run integration tests
cargo test --test integration

# Run example tests
cargo test --example minimal_server_test
```

### Manual Testing

Test commands manually:

```bash
cargo run --bin mcp-connect -- serve --debug
```

## Documentation

### Code Documentation

- Add doc comments to public APIs
- Use `///` for module and item documentation
- Include examples in doc comments where helpful

### User Documentation

When adding features:
1. Update README.md if needed
2. Add/update relevant docs in `docs/`
3. Update help text in CLI commands

## Commit Messages

Follow conventional commit format:

```
type(scope): subject

body (optional)

footer (optional)
```

**Types**:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code style changes
- `refactor`: Code refactoring
- `test`: Test additions/changes
- `chore`: Maintenance tasks

**Examples**:
```
feat(registry): add custom registry source support

docs(config): update configuration reference

fix(client): handle connection timeout properly
```

## Pull Request Process

1. **Update documentation** for any new features
2. **Add tests** for new functionality
3. **Ensure all tests pass**
4. **Update CHANGELOG.md** (if applicable)
5. **Describe changes** clearly in PR description

### PR Checklist

- [ ] Code follows project style guidelines
- [ ] Tests added/updated
- [ ] Documentation updated
- [ ] All tests pass
- [ ] Code is properly formatted
- [ ] No clippy warnings
- [ ] PR description is clear

## Reporting Issues

### Bug Reports

Include:
- Description of the bug
- Steps to reproduce
- Expected behavior
- Actual behavior
- Environment (OS, Rust version, etc.)
- Relevant logs or error messages

### Feature Requests

Include:
- Clear description of the feature
- Use case or motivation
- Proposed implementation (if any)
- Examples of how it would be used

## Code Review

- Be respectful and constructive
- Provide specific, actionable feedback
- Ask questions rather than making demands
- Appreciate contributions from others

## Questions?

- Open an issue for questions
- Check existing documentation
- Review existing code for patterns

Thank you for contributing to MCP Connect! 🎉

