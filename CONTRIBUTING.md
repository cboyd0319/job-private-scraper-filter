# Contributing to Private Job Scraper & Filter

Thank you for your interest in contributing! This document provides guidelines for contributing to this project.

## 🤝 How to Contribute

### Reporting Issues

1. **Search existing issues** first to avoid duplicates
2. **Use the issue template** when creating new issues
3. **Provide detailed information**:
   - Operating system and Python version
   - Steps to reproduce the issue
   - Expected vs actual behavior
   - Error messages and logs (remove any sensitive data)

### Submitting Changes

1. **Fork** the repository
2. **Create a feature branch** from `main`:
   ```bash
   git checkout -b feature/your-feature-name
   ```
3. **Make your changes** following the coding standards below
4. **Test thoroughly** including:
   - All existing functionality still works
   - New features work as expected
   - Edge cases are handled
5. **Commit with clear messages**:
   ```bash
   git commit -m "Add feature: brief description of what you added"
   ```
6. **Push to your fork** and create a Pull Request

## 📋 Coding Standards

### Python Code Style

- Follow **PEP 8** style guidelines
- Use **type hints** for function parameters and return values
- Write **docstrings** for all functions and classes
- Use **meaningful variable names**
- Keep functions focused and under 50 lines when possible

### Error Handling

- Use **comprehensive try/catch blocks**
- Log errors appropriately with context
- Fail gracefully and provide recovery mechanisms
- Use custom exceptions from `utils.errors` when appropriate

### Testing

- Test all new functionality
- Include both positive and negative test cases
- Test error conditions and recovery
- Verify cross-platform compatibility when possible

## 🏗️ Development Setup

1. **Clone your fork**:
   ```bash
   git clone https://github.com/YOUR_USERNAME/job-private-scraper-filter.git
   cd job-private-scraper-filter
   ```

2. **Create virtual environment**:
   ```bash
   python -m venv .venv
   source .venv/bin/activate  # Linux/macOS
   # or
   .venv\Scripts\activate     # Windows
   ```

3. **Install dependencies**:
   ```bash
   pip install -r requirements.txt
   python -m playwright install chromium
   ```

4. **Set up configuration**:
   ```bash
   cp .env.example .env
   cp user_prefs.example.json user_prefs.json
   # Edit these files with your test settings
   ```

## 🧪 Testing Your Changes

### Basic Functionality Test

```bash
# Test configuration loading
python agent.py --mode health

# Test notification system (with dummy credentials)
python agent.py --mode test

# Test database operations
python -c "from database import init_db; init_db(); print('Database OK')"
```

### Manual Testing Checklist

- [ ] Configuration loads without errors
- [ ] All agent modes work (`poll`, `digest`, `test`, `cleanup`, `health`)
- [ ] Error handling works properly
- [ ] No hardcoded secrets or credentials
- [ ] Cross-platform paths (use `pathlib`)
- [ ] Graceful degradation when services are unavailable

## 📝 Documentation

- **Update README.md** if you change functionality
- **Add docstrings** to new functions and classes
- **Update configuration examples** if you add new settings
- **Document breaking changes** in pull request description

## 🔍 Areas for Contribution

### High Priority

- **Additional job board scrapers** (Ashby, SmartRecruiters, etc.)
- **Enhanced error recovery** mechanisms
- **Performance optimizations** for large job sets
- **Additional notification channels** (Discord, Teams, etc.)

### Medium Priority

- **UI improvements** for the web interface
- **Advanced filtering** options
- **Analytics and reporting** features
- **Docker containerization**

### Documentation

- **Tutorial videos** or guides
- **Configuration examples** for specific companies
- **Troubleshooting guides**
- **API documentation**

## 🛠️ Technical Architecture

### Key Components

- **`agent.py`**: Main orchestrator and entry point
- **`database.py`**: SQLite database management with SQLModel
- **`sources/`**: Job board scrapers (one per platform)
- **`utils/`**: Core utilities (config, resilience, health, etc.)
- **`notify/`**: Notification systems (Slack, email)
- **`matchers/`**: Job scoring and filtering logic

### Design Principles

- **Resilience First**: System should gracefully handle all failure modes
- **Privacy by Design**: No data should ever leave the user's machine
- **Zero Configuration**: Should work out of the box with minimal setup
- **Cross-Platform**: Must work on Windows, macOS, and Linux
- **Self-Healing**: System should automatically recover from common issues

## 🎯 Pull Request Guidelines

### Before Submitting

- [ ] Code follows project style guidelines
- [ ] All tests pass
- [ ] Documentation is updated
- [ ] No sensitive data in commits
- [ ] Commit messages are clear and descriptive

### Pull Request Description

Include:
- **What** you changed
- **Why** you made the change
- **How** to test the changes
- **Any breaking changes**
- **Screenshots** if UI changes

## 📞 Getting Help

- **GitHub Issues**: For bugs and feature requests
- **GitHub Discussions**: For questions and general discussion
- **Code Review**: Maintainers will provide feedback on pull requests

## 🏆 Recognition

Contributors will be:
- Listed in the README.md file
- Mentioned in release notes for significant contributions
- Given credit in commit messages and pull requests

Thank you for helping make this project better! 🎉