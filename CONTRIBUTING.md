# Contributing to GhostDesk

Thank you for your interest in contributing to GhostDesk! This document provides guidelines and instructions for contributing to the project.

## Code of Conduct

This project adheres to a [Code of Conduct](CODE_OF_CONDUCT.md). By participating, you are expected to uphold this code.

## Getting Started

### Prerequisites

- Python 3.12+
- Docker and Docker Compose
- Git

### Development Setup

1. **Fork the repository** on GitHub
2. **Clone your fork:**
   ```bash
   git clone https://github.com/YOUR-USERNAME/GhostDesk.git
   cd GhostDesk
   ```

3. **Create a development branch:**
   ```bash
   git checkout -b feature/your-feature-name
   ```

4. **Set up development environment:**
   ```bash
   # Install dependencies
   uv sync

   # Start the development environment
   docker-compose up -d
   ```

5. **Run tests:**
   ```bash
   pytest
   ```

## Making Changes

### Code Style

- Follow PEP 8 guidelines for Python code
- Use type hints where appropriate
- Keep functions and methods focused and concise
- Write clear, descriptive commit messages

### Commits

- Use atomic commits (one logical change per commit)
- Write commit messages in the present tense ("Add feature" not "Added feature")
- Reference issues in commit messages when applicable (e.g., "Fix #123")

### Testing

- Write tests for new functionality
- Ensure all tests pass before submitting a PR:
  ```bash
  pytest
  ```
- Maintain or improve code coverage

## Submitting Changes

### Creating a Pull Request

1. **Push your branch** to your fork:
   ```bash
   git push origin feature/your-feature-name
   ```

2. **Create a Pull Request** on GitHub with:
   - Clear title describing the change
   - Description of what was changed and why
   - Reference to any related issues (#123)
   - Test instructions if applicable

3. **PR Requirements:**
   - All CI checks must pass
   - At least one approval required
   - No merge conflicts

### PR Review Process

- A maintainer will review your PR
- Respond to feedback and make requested changes
- Keep the PR up to date with the main branch

## Reporting Issues

### Bug Reports

Include:
- Clear description of the issue
- Steps to reproduce
- Expected vs actual behavior
- Environment details (OS, Python version, etc.)
- Screenshots or error logs if applicable

### Feature Requests

Include:
- Clear description of the feature
- Use cases and motivation
- Possible implementation approach (optional)

## Project Structure

```
GhostDesk/
├── src/ghostdesk/     # Main source code
├── tests/             # Test suite
├── .devcontainer/     # Development container config
├── .docker/           # Docker configuration
├── .github/workflows/ # CI/CD workflows
└── README.md          # Project documentation
```

## Security

For security vulnerabilities, please refer to [SECURITY.md](SECURITY.md) for responsible disclosure guidelines.

## Questions?

Check existing issues and discussions, or open a new one.

## License

By contributing, you agree that your contributions will be licensed under the same AGPL-3.0 with Commons Clause as the project. See [LICENSE](LICENSE) for details.

Thank you for contributing to GhostDesk!
