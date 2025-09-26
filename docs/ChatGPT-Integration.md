# ChatGPT API Integration Guide

This guide explains how to enable AI-enhanced job scoring with ChatGPT API integration. The system is designed to be **completely optional** - it works perfectly without AI, and AI can be toggled on/off easily.

## 🎯 Overview

The ChatGPT integration adds intelligent scoring and analysis on top of the existing rule-based system:

- **Hybrid Scoring**: Combines rule-based filtering with AI analysis
- **Smart Summaries**: AI-generated insights about job relevance
- **Fuzzy Matching**: Better detection of relevant jobs that rules might miss
- **Cost-Controlled**: Built-in token limits and rate limiting
- **Fallback Safe**: Always falls back to rules-only if AI fails

## 🚀 Super Easy Setup

### Step 1: Install Optional Dependencies

```bash
# In your virtual environment
pip install openai>=1.40.0 tiktoken>=0.7.0
```

Or uncomment the lines in `requirements.txt`:
```bash
pip install -r requirements.txt
```

### Step 2: Get OpenAI API Key

1. Go to [platform.openai.com](https://platform.openai.com)
2. Create an account or sign in
3. Navigate to API Keys
4. Create a new API key
5. Copy the key (starts with `sk-`)

### Step 3: Enable in Configuration

Edit your `.env` file:
```bash
# Enable ChatGPT integration
LLM_ENABLED=true
OPENAI_API_KEY=sk-your-actual-api-key-here

# Optional: Customize settings
OPENAI_MODEL=gpt-4o-mini  # Recommended for cost efficiency
LLM_TEMPERATURE=0.0       # Deterministic scoring
LLM_MAX_TOKENS=500        # Limit response length
LLM_MAX_DAILY_TOKENS=50000  # Daily usage cap
LLM_MAX_RPM=20            # Rate limit (requests per minute)
```

### Step 4: Test the Integration

```bash
python agent.py --mode test
```

Look for messages like:
- `✅ LLM integration initialized successfully`
- `Processing job: [Job Title] (score: 0.87, method: hybrid_50r_50ai, tokens: 245)`

## 🎛️ Configuration Options

### Environment Variables (`.env`)

| Variable | Default | Description |
|----------|---------|-------------|
| `LLM_ENABLED` | `false` | Enable/disable AI integration |
| `OPENAI_API_KEY` | - | Your OpenAI API key |
| `OPENAI_MODEL` | `gpt-4o-mini` | AI model to use |
| `LLM_TEMPERATURE` | `0.0` | Response randomness (0.0 = deterministic) |
| `LLM_MAX_TOKENS` | `500` | Max tokens per response |
| `LLM_MAX_DAILY_TOKENS` | `50000` | Daily usage limit |
| `LLM_MAX_RPM` | `20` | Requests per minute limit |
| `LLM_FALLBACK_TO_RULES` | `true` | Fall back to rules if AI fails |

### User Preferences (`user_prefs.json`)

```json
{
  "use_llm": true,           // Enable AI for this user
  "llm_weight": 0.5,         // 0.5 = equal weight rules+AI, 0.8 = mostly AI
  // ... other settings
}
```

## 💰 Cost Management

The system includes multiple cost controls:

### 1. **Pre-filtering with Rules**
- AI only analyzes jobs that pass basic rule filters
- Saves ~90% of API calls by filtering out obvious non-matches

### 2. **Token Limits**
- Daily caps prevent runaway costs
- Per-request limits keep responses focused
- Automatic fallback to rules-only when limits hit

### 3. **Rate Limiting**
- Respects OpenAI rate limits
- Configurable requests per minute
- Graceful degradation under load

### 4. **Model Selection**
- `gpt-4o-mini` recommended for best cost/performance
- ~$0.15 per 1M input tokens (very affordable)
- Typical job analysis: 200-400 tokens (~$0.0001 per job)

### Example Costs
- **100 jobs/day** with AI analysis: ~$0.01-0.04/day
- **1000 jobs/day**: ~$0.10-0.40/day
- Most users: **<$5/month**

## 🔧 How It Works

### 1. **Two-Stage Scoring**
```
Job → Rules Filter → AI Analysis → Combined Score
```

### 2. **Hybrid Score Calculation**
```
Final Score = (Rules Score × Rules Weight) + (AI Score × AI Weight)
```

### 3. **Example Flow**
```
1. Job: "Senior Product Security Engineer at Cloudflare"
2. Rules: 0.8 (title match + location + keywords)
3. AI: 0.92 (analyzes full description, finds relevant tech stack)
4. Final: (0.8 × 0.5) + (0.92 × 0.5) = 0.86
```

## 📊 Enhanced Notifications

With AI enabled, you get richer notifications:

### Slack Alerts
```
Senior Product Security Engineer at Cloudflare
📍 Remote (US)
📈 Score: 91% (Rules: 80% • AI: 92%)
🤖 AI Summary: Strong security role with Zero Trust focus
✅ Matched: Security Engineer, Remote location
🧠 AI Insights: Mentions Kubernetes, IAM, relevant experience level
```

### Email Digest
- Color-coded scoring breakdowns
- AI summaries highlighted in blue boxes
- Organized reasons (Rules vs AI insights)
- Enhanced job descriptions

## 🔍 Monitoring & Debugging

### Check AI Usage
```bash
python agent.py --mode health
```

Shows:
- Daily token usage
- Rate limit status
- AI success/failure rates

### Logs
```bash
tail -f data/logs/scraper_YYYYMMDD.log
```

Look for:
- `LLM scored 'Job Title': 0.85 (245 tokens)`
- `Token usage: +245 tokens (daily total: 1250)`
- `Rate limiting: waiting 2.3s for api.openai.com`

## 🛠️ Troubleshooting

### AI Not Working?

1. **Check API Key**
   ```bash
   python -c "import openai; print('OpenAI installed')"
   ```

2. **Verify Environment**
   ```bash
   python agent.py --mode test --verbose
   ```

3. **Check Logs**
   ```bash
   grep -i "llm\|openai\|token" data/logs/scraper_*.log
   ```

### Common Issues

**"OpenAI package not installed"**
```bash
pip install openai>=1.40.0
```

**"LLM enabled but OPENAI_API_KEY not found"**
- Check `.env` file has correct API key
- Verify no spaces around the `=` sign

**"Daily token limit reached"**
- Wait until tomorrow, or increase `LLM_MAX_DAILY_TOKENS`
- Consider reducing `LLM_MAX_TOKENS` per request

**"Rate limited"**
- Normal behavior, system will wait and retry
- Reduce `LLM_MAX_RPM` if needed

## 🔒 Security & Privacy

### Data Privacy
- Only sends public job titles/descriptions to OpenAI
- No personal information transmitted
- API calls use HTTPS encryption

### API Key Security
- Store in `.env` file only
- Never commit to version control
- Restrict file permissions: `chmod 600 .env`

### Audit Trail
- All AI interactions logged
- Token usage tracked
- Failed requests recorded

## 🎛️ Advanced Configuration

### Custom Prompts
The AI prompt can be customized by modifying `utils/llm.py`:

```python
def create_scoring_prompt(job: Dict, preferences: Dict) -> str:
    # Customize the prompt here
    pass
```

### Different Models
```bash
# Use GPT-4 for higher quality (more expensive)
OPENAI_MODEL=gpt-4o

# Use different providers (if compatible)
OPENAI_BASE_URL=https://api.your-provider.com/v1
```

### Batch Processing
For high-volume usage, consider:
- Increasing `LLM_MAX_RPM` (if your API tier allows)
- Using `gpt-3.5-turbo` for faster processing
- Adjusting `llm_weight` to rely more on rules

## 📈 Performance Tips

### Optimal Settings
```bash
# For cost optimization
OPENAI_MODEL=gpt-4o-mini
LLM_MAX_TOKENS=300
LLM_TEMPERATURE=0.0

# For quality optimization
OPENAI_MODEL=gpt-4o
LLM_MAX_TOKENS=500
LLM_TEMPERATURE=0.1
```

### User Preferences
```json
{
  "llm_weight": 0.3,  // Mostly rules, AI as tiebreaker
  "llm_weight": 0.7,  // Mostly AI, rules as filter
  "llm_weight": 0.5   // Balanced approach (recommended)
}
```

---

**The beauty of this integration: it's completely optional and can be enabled/disabled anytime without affecting core functionality!** 🚀