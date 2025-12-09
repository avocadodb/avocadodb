# Docker Deployment Guide

Complete guide for deploying AvocadoDB using Docker and Kubernetes.

## Quick Start

The fastest way to get AvocadoDB running:

```bash
# Clone the repository
git clone https://github.com/avocadodb/avocadodb
cd avocadodb

# Start with Docker Compose
docker-compose up -d

# Check status
docker-compose ps

# View logs
docker-compose logs -f avocado-server
```

That's it! AvocadoDB is now running on `http://localhost:8765`

## Docker Image

### Using Pre-built Images

Once published to Docker Hub:

```bash
# Pull the latest version
docker pull avocadodb/avocadodb:latest

# Pull a specific version
docker pull avocadodb/avocadodb:v0.1.0

# Run the container
docker run -d \
  -p 8765:8765 \
  -v avocado-data:/data \
  --name avocadodb \
  avocadodb/avocadodb:latest
```

### Building from Source

```bash
# Build the Docker image
docker build -t avocadodb:local .

# Build with custom tag
docker build -t myorg/avocadodb:v1.0 .

# Build for multiple architectures (requires buildx)
docker buildx build \
  --platform linux/amd64,linux/arm64 \
  -t avocadodb/avocadodb:latest \
  --push .
```

## Configuration

### Environment Variables

Configure AvocadoDB using environment variables:

| Variable | Default | Description |
|----------|---------|-------------|
| `PORT` | `8765` | HTTP server port |
| `RUST_LOG` | `info` | Log level (trace, debug, info, warn, error) |
| `AVOCADODB_EMBEDDING_PROVIDER` | `local` | Embedding provider: `local` or `openai` |
| `AVOCADODB_EMBEDDING_MODEL` | `minilm` | Model: `minilm` (384d), `nomic` (768d), `bgelarge` (1024d) |
| `OPENAI_API_KEY` | - | OpenAI API key (only if using OpenAI embeddings) |

### Example with Custom Configuration

```bash
docker run -d \
  -p 8765:8765 \
  -v avocado-data:/data \
  -e PORT=9000 \
  -e RUST_LOG=debug \
  -e AVOCADODB_EMBEDDING_MODEL=nomic \
  --name avocadodb \
  avocadodb/avocadodb:latest
```

### Using OpenAI Embeddings

```bash
docker run -d \
  -p 8765:8765 \
  -v avocado-data:/data \
  -e AVOCADODB_EMBEDDING_PROVIDER=openai \
  -e OPENAI_API_KEY=sk-... \
  --name avocadodb \
  avocadodb/avocadodb:latest
```

## Docker Compose

### Basic Setup

```yaml
version: '3.8'

services:
  avocado-server:
    image: avocadodb/avocadodb:latest
    ports:
      - "8765:8765"
    volumes:
      - avocado-data:/data
    environment:
      - RUST_LOG=info
      - AVOCADODB_EMBEDDING_MODEL=nomic
    restart: unless-stopped

volumes:
  avocado-data:
```

### Development Setup

Mount local directories for development:

```yaml
version: '3.8'

services:
  avocado-server:
    build: .
    ports:
      - "8765:8765"
    volumes:
      - ./local-data:/data
    environment:
      - RUST_LOG=debug
    restart: unless-stopped
```

### Managing the Stack

```bash
# Start services
docker-compose up -d

# View logs
docker-compose logs -f

# Stop services
docker-compose down

# Stop and remove volumes (WARNING: deletes all data)
docker-compose down -v

# Rebuild and restart
docker-compose up -d --build

# Scale (if configured for horizontal scaling)
docker-compose up -d --scale avocado-server=3
```

## Data Persistence

### Volumes

AvocadoDB stores data in `/data` inside the container. Use Docker volumes for persistence:

```bash
# Create named volume
docker volume create avocado-data

# Use named volume
docker run -d \
  -v avocado-data:/data \
  avocadodb/avocadodb:latest

# Use bind mount (local directory)
docker run -d \
  -v $(pwd)/my-data:/data \
  avocadodb/avocadodb:latest
```

### Backup and Restore

```bash
# Backup: Copy database from container
docker cp avocadodb:/data/my-project/.avocado ./backup/

# Restore: Copy database to container
docker cp ./backup/.avocado avocadodb:/data/my-project/

# Backup entire volume
docker run --rm \
  -v avocado-data:/data \
  -v $(pwd)/backups:/backup \
  alpine tar czf /backup/avocado-backup-$(date +%Y%m%d).tar.gz -C /data .

# Restore from backup
docker run --rm \
  -v avocado-data:/data \
  -v $(pwd)/backups:/backup \
  alpine tar xzf /backup/avocado-backup-20250117.tar.gz -C /data
```

## Networking

### Port Mapping

```bash
# Map to different host port
docker run -d -p 9000:8765 avocadodb/avocadodb:latest

# Bind to specific interface
docker run -d -p 127.0.0.1:8765:8765 avocadodb/avocadodb:latest

# Multiple containers on different ports
docker run -d -p 8765:8765 --name avocado-prod avocadodb/avocadodb:latest
docker run -d -p 8766:8765 --name avocado-dev avocadodb/avocadodb:latest
```

### Custom Networks

```bash
# Create network
docker network create avocado-net

# Run with custom network
docker run -d \
  --network avocado-net \
  --name avocadodb \
  avocadodb/avocadodb:latest

# Connect from another container
docker run -it \
  --network avocado-net \
  curlimages/curl:latest \
  curl http://avocadodb:8765/health
```

## Production Deployment

### Best Practices

1. **Use specific version tags** (not `latest`)
   ```bash
   docker run -d avocadodb/avocadodb:v0.1.0
   ```

2. **Set resource limits**
   ```bash
   docker run -d \
     --memory="2g" \
     --cpus="2.0" \
     avocadodb/avocadodb:v0.1.0
   ```

3. **Enable health checks**
   ```bash
   docker run -d \
     --health-cmd="curl -f http://localhost:8765/health || exit 1" \
     --health-interval=30s \
     --health-timeout=3s \
     --health-retries=3 \
     avocadodb/avocadodb:v0.1.0
   ```

4. **Use restart policies**
   ```bash
   docker run -d \
     --restart=unless-stopped \
     avocadodb/avocadodb:v0.1.0
   ```

5. **Run as non-root** (already configured in Dockerfile)

6. **Enable logging driver**
   ```bash
   docker run -d \
     --log-driver=json-file \
     --log-opt max-size=10m \
     --log-opt max-file=3 \
     avocadodb/avocadodb:v0.1.0
   ```

### Production docker-compose.yml

```yaml
version: '3.8'

services:
  avocado-server:
    image: avocadodb/avocadodb:v0.1.0
    container_name: avocadodb-prod
    ports:
      - "127.0.0.1:8765:8765"
    volumes:
      - avocado-data:/data
    environment:
      - PORT=8765
      - RUST_LOG=warn
      - AVOCADODB_EMBEDDING_MODEL=nomic
    restart: unless-stopped
    deploy:
      resources:
        limits:
          cpus: '2.0'
          memory: 2G
        reservations:
          cpus: '1.0'
          memory: 1G
    healthcheck:
      test: ["CMD", "/bin/sh", "-c", "command -v curl >/dev/null && curl -f http://localhost:8765/health || exit 1"]
      interval: 30s
      timeout: 3s
      retries: 3
      start_period: 10s
    logging:
      driver: "json-file"
      options:
        max-size: "10m"
        max-file: "3"
    networks:
      - avocado-network

networks:
  avocado-network:
    driver: bridge

volumes:
  avocado-data:
    driver: local
```

### Reverse Proxy (Nginx)

```nginx
# /etc/nginx/sites-available/avocadodb
upstream avocadodb {
    server localhost:8765;
}

server {
    listen 80;
    server_name avocadodb.example.com;

    location / {
        proxy_pass http://avocadodb;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;

        # Timeout settings
        proxy_connect_timeout 60s;
        proxy_send_timeout 60s;
        proxy_read_timeout 60s;
    }
}
```

### Reverse Proxy (Traefik with Docker Compose)

```yaml
version: '3.8'

services:
  traefik:
    image: traefik:v2.10
    command:
      - "--api.insecure=true"
      - "--providers.docker=true"
      - "--entrypoints.web.address=:80"
    ports:
      - "80:80"
      - "8080:8080"
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock

  avocado-server:
    image: avocadodb/avocadodb:latest
    labels:
      - "traefik.enable=true"
      - "traefik.http.routers.avocado.rule=Host(`avocadodb.example.com`)"
      - "traefik.http.services.avocado.loadbalancer.server.port=8765"
    volumes:
      - avocado-data:/data
    restart: unless-stopped

volumes:
  avocado-data:
```

## Monitoring

### Health Checks

```bash
# Check health
curl http://localhost:8765/health

# Response
{"status":"ok","service":"avocadodb-daemon"}

# Docker health status
docker inspect --format='{{.State.Health.Status}}' avocadodb
```

### Logs

```bash
# Follow logs
docker logs -f avocadodb

# Last 100 lines
docker logs --tail 100 avocadodb

# Logs since specific time
docker logs --since 1h avocadodb

# With timestamps
docker logs -t avocadodb
```

### Stats

```bash
# Container resource usage
docker stats avocadodb

# Detailed stats
docker stats --no-stream avocadodb
```

## Troubleshooting

### Container won't start

```bash
# Check logs
docker logs avocadodb

# Inspect container
docker inspect avocadodb

# Check if port is already in use
sudo netstat -tulpn | grep 8765
```

### Permission issues

```bash
# Fix volume permissions
docker run --rm \
  -v avocado-data:/data \
  alpine chown -R 1000:1000 /data
```

### Cannot connect to server

```bash
# Check if container is running
docker ps

# Check health
curl http://localhost:8765/health

# Test from inside container
docker exec -it avocadodb /bin/sh -c "curl http://localhost:8765/health"

# Check network
docker network inspect bridge
```

### Performance issues

```bash
# Check resource usage
docker stats avocadodb

# Increase resources
docker update \
  --memory="4g" \
  --cpus="4.0" \
  avocadodb

# Or recreate with new limits
docker run -d \
  --memory="4g" \
  --cpus="4.0" \
  -p 8765:8765 \
  -v avocado-data:/data \
  --name avocadodb-new \
  avocadodb/avocadodb:latest
```

### Database corruption

```bash
# Backup current data
docker cp avocadodb:/data/my-project/.avocado ./corrupted-backup/

# Stop container
docker stop avocadodb

# Remove corrupted volume
docker volume rm avocado-data

# Recreate and restore from backup
docker volume create avocado-data
docker run -d \
  -v avocado-data:/data \
  --name avocadodb \
  avocadodb/avocadodb:latest

# Copy backup back (if recoverable)
docker cp ./good-backup/.avocado avocadodb:/data/my-project/
```

## Security

### Best Practices

1. **Don't expose to internet directly** - Use reverse proxy with authentication
2. **Use secrets for sensitive data**
   ```bash
   docker secret create openai_key ./openai.key
   docker service create \
     --secret openai_key \
     --env OPENAI_API_KEY_FILE=/run/secrets/openai_key \
     avocadodb/avocadodb:latest
   ```
3. **Regular updates**
   ```bash
   docker pull avocadodb/avocadodb:latest
   docker-compose up -d
   ```
4. **Scan images for vulnerabilities**
   ```bash
   docker scan avocadodb/avocadodb:latest
   ```

## Advanced Usage

### Multi-Project Setup

Run multiple isolated instances:

```bash
# Project 1
docker run -d \
  -p 8765:8765 \
  -v project1-data:/data \
  --name avocado-project1 \
  avocadodb/avocadodb:latest

# Project 2
docker run -d \
  -p 8766:8765 \
  -v project2-data:/data \
  --name avocado-project2 \
  avocadodb/avocadodb:latest
```

### Custom Embedding Models

Cache models in a volume for faster startups:

```bash
# Create model cache volume
docker volume create fastembed-cache

# Run with cache
docker run -d \
  -v avocado-data:/data \
  -v fastembed-cache:/home/avocado/.cache/fastembed \
  -e AVOCADODB_EMBEDDING_MODEL=nomic \
  avocadodb/avocadodb:latest
```

### Using with Docker Swarm

```bash
# Initialize swarm
docker swarm init

# Create service
docker service create \
  --name avocadodb \
  --replicas 3 \
  --publish 8765:8765 \
  --mount type=volume,src=avocado-data,dst=/data \
  avocadodb/avocadodb:latest

# Scale service
docker service scale avocadodb=5

# Update service
docker service update \
  --image avocadodb/avocadodb:v0.2.0 \
  avocadodb
```

## Image Information

### Size Optimization

The Docker image is optimized for small size:

- **Multi-stage build** reduces final image size
- **Debian slim** base image (~70MB)
- **Stripped binaries** remove debug symbols
- **Layer caching** for fast rebuilds

Expected sizes:
- Builder stage: ~2.5GB (not in final image)
- Final image: ~80-100MB

### Architecture Support

Built for multiple architectures:
- `linux/amd64` (x86_64)
- `linux/arm64` (ARM64/Apple Silicon)

## Next Steps

- [Kubernetes Deployment](./KUBERNETES.md) - Deploy to Kubernetes
- [API Documentation](./API.md) - HTTP API reference
- [Performance Tuning](./performance.md) - Optimize for your workload
- [Monitoring](./monitoring.md) - Set up monitoring and alerting

## Support

- Issues: https://github.com/avocadodb/avocadodb/issues
- Documentation: https://avocadodb.dev/docs
- Docker Hub: https://hub.docker.com/r/avocadodb/avocadodb
