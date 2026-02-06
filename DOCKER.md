# Docker Usage Guide for APChat

This document explains how to use the APChat Docker container published to GitHub Container Registry.

## Quick Start

```bash
# Pull the latest image
docker pull ghcr.io/ayourtch/apchat:latest

# Run with help to see available options
docker run --rm ghcr.io/ayourtch/apchat:latest --help
```

## Basic Usage

The Docker container uses the `apchat` binary as its entrypoint, so you can pass any apchat CLI arguments directly to `docker run`.

### Interactive REPL Mode

```bash
docker run -it --rm \
  -e GROQ_API_KEY=$GROQ_API_KEY \
  -e ANTHROPIC_AUTH_TOKEN_BLU=$ANTHROPIC_AUTH_TOKEN_BLU \
  ghcr.io/ayourtch/apchat:latest -i
```

### Multi-Agent Mode

```bash
docker run -it --rm \
  -e GROQ_API_KEY=$GROQ_API_KEY \
  ghcr.io/ayourtch/apchat:latest --agents -i
```

### One-Shot Task Mode

```bash
docker run --rm \
  -e GROQ_API_KEY=$GROQ_API_KEY \
  ghcr.io/ayourtch/apchat:latest \
  --task "Explain the difference between Rust traits and interfaces"
```

### Web Server Mode

```bash
docker run -d -p 8080:8080 \
  -e GROQ_API_KEY=$GROQ_API_KEY \
  ghcr.io/ayourtch/apchat:latest \
  --web --bind 0.0.0.0:8080
```

## Environment Variables

APChat requires API keys for the LLM providers you want to use. Pass these as environment variables:

```bash
# For Groq models
-e GROQ_API_KEY=your_groq_api_key

# For Anthropic Claude (supports multiple model slots)
-e ANTHROPIC_AUTH_TOKEN_BLU=your_claude_key_for_blu_model
-e ANTHROPIC_AUTH_TOKEN_GRN=your_claude_key_for_grn_model
-e ANTHROPIC_AUTH_TOKEN_RED=your_claude_key_for_red_model

# For OpenAI
-e OPENAI_API_KEY=your_openai_api_key
```

### Using an Environment File

For convenience, create a `.env` file with your API keys:

```bash
# .env file
GROQ_API_KEY=gsk_...
ANTHROPIC_AUTH_TOKEN_BLU=sk-ant-...
ANTHROPIC_AUTH_TOKEN_GRN=sk-ant-...
OPENAI_API_KEY=sk-...
```

Then run with `--env-file`:

```bash
docker run -it --rm \
  --env-file .env \
  ghcr.io/ayourtch/apchat:latest -i
```

## Volume Mounting

### Persistent Configuration

Mount a directory to persist configuration and session data:

```bash
docker run -it --rm \
  -v $(pwd)/apchat-data:/home/apchat/.apchat \
  -e GROQ_API_KEY=$GROQ_API_KEY \
  ghcr.io/ayourtch/apchat:latest -i
```

### Working with Local Files

The container uses `/workspace` as the working directory. Mount your project directory there to work with local code:

```bash
# Mount current directory to /workspace
docker run -it --rm \
  -v $(pwd):/workspace \
  -e GROQ_API_KEY=$GROQ_API_KEY \
  ghcr.io/ayourtch/apchat:latest -i

# Or with a specific project directory
docker run -it --rm \
  -v /path/to/your/project:/workspace \
  -e GROQ_API_KEY=$GROQ_API_KEY \
  ghcr.io/ayourtch/apchat:latest --task "Analyze the codebase"
```

All file operations (read_file, write_file, edit_file, etc.) will work relative to `/workspace`.

### Custom Policy Files

Mount a custom policy file:

```bash
docker run -it --rm \
  -v $(pwd)/my-policy.json:/config/policy.json \
  -e GROQ_API_KEY=$GROQ_API_KEY \
  ghcr.io/ayourtch/apchat:latest -i --policy-file /config/policy.json
```

## Common CLI Options

All apchat CLI options can be passed to the container:

```bash
# Streaming responses (default)
docker run --rm ghcr.io/ayourtch/apchat:latest --stream

# Auto-confirm all operations
docker run --rm ghcr.io/ayourtch/apchat:latest -i --auto-confirm

# Verbose output
docker run --rm ghcr.io/ayourtch/apchat:latest -i --verbose

# Custom model configuration
docker run --rm ghcr.io/ayourtch/apchat:latest \
  --model-blu-model llama3-70b-8192 \
  --task "Your task"

# Using llama.cpp
docker run --rm ghcr.io/ayourtch/apchat:latest \
  --llama-cpp-url http://host.docker.internal:8080 \
  --task "Your task"
```

## Docker Compose Example

Create a `docker-compose.yml` file:

```yaml
version: '3.8'

services:
  apchat:
    image: ghcr.io/ayourtch/apchat:latest
    container_name: apchat
    stdin_open: true
    tty: true
    environment:
      - GROQ_API_KEY=${GROQ_API_KEY}
      - ANTHROPIC_AUTH_TOKEN_BLU=${ANTHROPIC_AUTH_TOKEN_BLU}
      - ANTHROPIC_AUTH_TOKEN_GRN=${ANTHROPIC_AUTH_TOKEN_GRN}
    volumes:
      - ./apchat-data:/home/apchat/.apchat
      - .:/workspace  # Mount current directory to /workspace
    command: ["--agents", "-i"]

  apchat-web:
    image: ghcr.io/ayourtch/apchat:latest
    container_name: apchat-web
    ports:
      - "8080:8080"
    environment:
      - GROQ_API_KEY=${GROQ_API_KEY}
    command: ["--web", "--bind", "0.0.0.0:8080"]
```

Run with:

```bash
# Interactive mode
docker-compose run --rm apchat

# Web server
docker-compose up apchat-web
```

## Building Locally

If you want to build the Docker image locally instead of pulling from GHCR:

```bash
# Build the image
docker build -t apchat:local .

# Run the locally built image
docker run -it --rm -e GROQ_API_KEY=$GROQ_API_KEY apchat:local -i
```

## Publishing to GitHub Container Registry

The Docker image is automatically built and published using GitHub Actions:

1. Go to your repository on GitHub
2. Click on "Actions" tab
3. Select "Build and Publish Docker Image" workflow
4. Click "Run workflow"
5. Optionally specify a custom tag (e.g., `v1.0.0`)
6. Click "Run workflow" button

The workflow will build the image for both `linux/amd64` and `linux/arm64` platforms and push to GHCR with the following tags:
- `latest` - Always updated with the most recent build
- `<branch>-<sha>` - Git commit SHA tagged with branch name
- Custom tag (if specified in the workflow input)

## Authenticating with GHCR

To pull private images from GitHub Container Registry:

```bash
# Create a GitHub Personal Access Token with read:packages scope
# Then login:
echo $GITHUB_TOKEN | docker login ghcr.io -u USERNAME --password-stdin

# Now you can pull the image
docker pull ghcr.io/ayourtch/apchat:latest
```

For public images, no authentication is required.

## Troubleshooting

### Permission Issues

If you encounter permission issues with mounted volumes:

```bash
# Run as your user
docker run -it --rm \
  -u $(id -u):$(id -g) \
  -v $(pwd):/workspace \
  ghcr.io/ayourtch/apchat:latest -i
```

### Network Issues with Local Services

To access services running on your host machine from inside the container:

```bash
# On Linux
--add-host=host.docker.internal:host-gateway

# Example with llama.cpp running on host
docker run --rm \
  --add-host=host.docker.internal:host-gateway \
  ghcr.io/ayourtch/apchat:latest \
  --llama-cpp-url http://host.docker.internal:8080
```

### Debugging

Run with shell access to debug:

```bash
docker run -it --rm \
  --entrypoint /bin/bash \
  ghcr.io/ayourtch/apchat:latest
```

## Image Tags

Available tags:
- `latest` - Most recent build from main branch
- `main-<sha>` - Specific commit from main branch
- `v*.*.*` - Semantic version tags (when manually specified)

To use a specific version:

```bash
docker pull ghcr.io/ayourtch/apchat:v1.0.0
```
