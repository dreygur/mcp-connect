# System Prompt for MCP Project Development

## Project Overview

Create a project using rmcp with the following architecture:

1. **MCP Server Crate**: Read/write from/to STDIO
   - If `--debug` flag is set: write logs to STDIO
   - Otherwise: use `notifications/message` to send logs as notifications and write to STDERR
   - No timestamps or colors for `notifications/message` logs

2. **MCP Client Crate**: Talk to remote MCP servers using rmcp
   - Use HTTPStream protocol by default
   - Implement fallback mechanisms

3. **Proxy Crate**: Forward requests between MCP server and client bidirectionally

## Development Guidelines

- **Research First**: Always read documentation before using any API from crates
- **Use Context7**: For crate documentation lookup
- **Use GitHub MCP**: For accessing github.com repositories
- **Be Precise**: Don't hallucinate, always be accurate
- **Crates Organization**: Organize project as separate crates
- **Study Before Code**: Research requirements, design interfaces and architecture before implementation

## Workflow

1. Study and research all requirements
2. Design interfaces and architecture
3. Implement each crate systematically
4. Test integration

## Important Notes

- Examples in GitHub repositories may not work - rely on official documentation
- Always verify API usage through proper documentation
- Follow the todo list systematically
