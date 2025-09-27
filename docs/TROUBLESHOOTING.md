# Troubleshooting Guide

This guide helps you diagnose and resolve common issues with the Private Job Scraper & Filter.

## 🔍 Quick Diagnosis

First, run the built-in health check:

```bash
python agent.py --mode health
```

This will show you the system status and highlight any critical issues.

## 🚨 Common Issues

### Installation Problems

#### Python Not Found
```
'python' is not recognized as an internal or external command
```

**Solutions**:
1. **Windows**: Reinstall Python with "Add Python to PATH" checked
2. **macOS/Linux**: Try `python3` instead of `python`
3. **Verify installation**: `python --version` should show 3.11+

#### Permission Denied Errors
```
PermissionError: [Errno 13] Permission denied
```

**Solutions**:
1. **Use virtual environments**: `python -m venv .venv`
2. **Check file permissions**: Files should be readable/writable by your user
3. **Windows**: Run PowerShell as Administrator for initial setup only

#### Playwright Installation Fails
```
Error installing Playwright browsers
```

**Solutions**:
1. **Install system dependencies**: `python -m playwright install-deps`
2. **Check internet connection**: Ensure firewall allows downloads
3. **Manual installation**: `python -m playwright install chromium --force`

### Configuration Issues

#### No Jobs Found
```
Polling completed: 0 jobs found, 0 new
```

**Diagnosis**:
1. Check `user_prefs.json` configuration
2. Verify company URLs are accessible
3. Check if `title_allowlist` is too restrictive

**Solutions**:
1. **Test job board URLs** manually in browser
2. **Broaden title allowlist**: Add more general terms
3. **Check location constraints**: Ensure they match job locations
4. **Verify company configuration**:
   ```json
   {
     "id": "company-name",
     "board_type": "greenhouse",  // Must match actual platform
     "url": "https://boards.greenhouse.io/company"
   }
   ```

#### Invalid Configuration
```
Configuration error: Invalid JSON in user_prefs.json
```

**Solutions**:
1. **Validate JSON syntax**: Use [jsonlint.com](https://jsonlint.com)
2. **Copy from example**: `cp user_prefs.example.json user_prefs.json`
3. **Common JSON errors**:
   - Missing commas between items
   - Trailing commas (not allowed in JSON)
   - Unmatched brackets or quotes

### Notification Problems

#### Slack Notifications Not Working
```
Error sending Slack alert: 404 Client Error
```

**Diagnosis**:
1. Verify webhook URL in `.env` file
2. Test webhook URL manually
3. Check Slack app permissions

**Solutions**:
1. **Create new webhook**:
   - Go to your Slack workspace settings
   - Add "Incoming Webhooks" app
   - Copy the webhook URL (starts with `https://hooks.slack.com/`)
2. **Update `.env` file**:
   ```
   SLACK_WEBHOOK_URL=https://hooks.slack.com/services/YOUR/WEBHOOK/URL
   ```

#### Email Notifications Failing
```
Error sending email: 535 Authentication failed
```

**Solutions**:
1. **Gmail users**: Use App Password, not regular password
   - Enable 2-Factor Authentication
   - Generate App Password in Google Account settings
   - Use the generated password in `.env`
2. **Check SMTP settings**:
   ```
   SMTP_HOST=smtp.gmail.com
   SMTP_PORT=587
   SMTP_USER=your_email@gmail.com
   SMTP_PASS=your_app_password
   ```

### Scraping Issues

#### Timeout Errors
```
ScrapingException: Timeout waiting for page to load
```

**Solutions**:
1. **Increase timeout**: Edit `user_prefs.json`, add `"timeout_seconds": 60`
2. **Check internet connection**: Ensure stable network
3. **Verify website accessibility**: Test URLs in browser

#### JavaScript Pages Not Loading
```
No jobs found on JavaScript-heavy page
```

**Solutions**:
1. **Use generic_js board type**:
   ```json
   {
     "board_type": "generic_js",
     "url": "https://careers.company.com"
   }
   ```
2. **Install browser dependencies**: `python -m playwright install-deps`

#### Rate Limiting
```
HTTP 429: Too Many Requests
```

**Solutions**:
1. **Reduce polling frequency**: Modify scheduled tasks to run less often
2. **Limit companies per run**: Set `"max_companies_per_run": 5` in config
3. **Wait and retry**: The system will automatically back off

### Database Issues

#### Database Corruption
```
Database integrity check failed
```

**Solutions**:
1. **Automatic recovery**: Run `python agent.py --mode health` for guided recovery
2. **Manual backup restore**:
   ```bash
   # List available backups
   ls data/backups/

   # Restore from backup (replace with actual backup name)
   cp data/backups/jobs_backup_YYYYMMDD_HHMMSS.sqlite data/jobs.sqlite
   ```

#### Database Locked
```
database is locked
```

**Solutions**:
1. **Check for running instances**: Only one instance should run at a time
2. **Restart the system**: Kill any hanging processes
3. **Remove lock file**: `rm data/.scraper.lock` (if exists)

### Performance Issues

#### High Memory Usage
```
Memory usage warning in health check
```

**Solutions**:
1. **Reduce batch size**: Set `"max_companies_per_run": 3`
2. **Clean up old jobs**: Run `python agent.py --mode cleanup`
3. **Restart periodically**: Set up weekly system restarts

#### Slow Scraping
**Solutions**:
1. **Check network speed**: Ensure stable internet connection
2. **Reduce concurrent requests**: Modify rate limiting settings
3. **Use local database**: Ensure `data/` directory is on fast storage

## 🔧 Advanced Troubleshooting

### Enable Debug Logging

Add to your environment:
```bash
# In .env file
LOG_LEVEL=DEBUG
```

Then check detailed logs:
```bash
tail -f data/logs/scraper_YYYYMMDD.log
```

### Manual Testing

Test individual components:

```bash
# Test configuration loading
python -c "from utils.config import config_manager; config_manager.load_config(); print('Config OK')"

# Test database
python -c "from database import init_db; init_db(); print('Database OK')"

# Test specific job board
python -c "
from sources import greenhouse
jobs = greenhouse.scrape('https://boards.greenhouse.io/discord')
print(f'Found {len(jobs)} jobs')
"
```

### Health Check Details

The health check provides detailed system information:

```bash
python agent.py --mode health
```

Key metrics to watch:
- **CPU/Memory usage**: Should be reasonable for your system
- **Database status**: Should show "OK" with job counts
- **Configuration**: Should load without errors
- **Network connectivity**: Should access job boards successfully

### Log Analysis

Common log patterns to look for:

```bash
# Find errors
grep "ERROR" data/logs/scraper_*.log

# Find network issues
grep "timeout\|connection\|network" data/logs/scraper_*.log

# Find database issues
grep "database\|sqlite" data/logs/scraper_*.log
```

## 📞 Getting Additional Help

### Before Reporting Issues

1. **Run health check**: `python agent.py --mode health`
2. **Check recent logs**: `tail -50 data/logs/scraper_*.log`
3. **Try with example config**: Copy from `.example` files
4. **Test on a single company**: Reduce to one job board for testing

### When Reporting Issues

Include this information:
- **Operating system and version**
- **Python version**: `python --version`
- **Error messages** (remove any sensitive data)
- **Configuration** (remove secrets/credentials)
- **Health check output**
- **Recent log entries**

### Support Channels

1. **GitHub Issues**: [Create an issue](https://github.com/cboyd0319/job-private-scraper-filter/issues)
2. **Discussions**: [GitHub Discussions](https://github.com/cboyd0319/job-private-scraper-filter/discussions)
3. **Security Issues**: See [SECURITY.md](../SECURITY.md)

### Self-Help Resources

- **Documentation**: [docs/](./README.md)
- **Configuration Examples**: [user_prefs.example.json](../user_prefs.example.json)
- **Health Monitoring**: Built-in `--mode health`
- **Test Mode**: Built-in `--mode test`

---

**Remember**: The system is designed to be self-healing. Many issues will resolve automatically with time, retries, and built-in recovery mechanisms.
