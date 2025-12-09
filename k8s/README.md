# Kubernetes Deployment

Complete guide for deploying AvocadoDB to Kubernetes.

## Quick Start

Deploy AvocadoDB to Kubernetes in under 5 minutes:

```bash
# Using kubectl
kubectl apply -f k8s/

# Using kustomize
kubectl apply -k k8s/

# Verify deployment
kubectl get pods -l app=avocadodb
kubectl get svc avocadodb
```

## Prerequisites

- Kubernetes cluster (v1.20+)
- `kubectl` configured
- Persistent storage provider (for PVC)
- (Optional) Ingress controller (nginx, traefik, etc.)

## Architecture

```
┌─────────────────┐
│   Ingress       │  (Optional: external access)
└────────┬────────┘
         │
┌────────▼────────┐
│   Service       │  (ClusterIP on port 8765)
│   (avocadodb)   │
└────────┬────────┘
         │
    ┌────▼──────┬──────┬──────┐
    │           │      │      │
┌───▼───┐  ┌───▼───┐  ┌───▼───┐
│ Pod 1 │  │ Pod 2 │  │ Pod 3 │  (Deployment replicas)
└───┬───┘  └───┬───┘  └───┬───┘
    │          │          │
    │    ┌─────▼──────┐   │
    └────►    PVC     ◄───┘  (Shared storage)
         └────────────┘
```

## Files

- **deployment.yaml**: Main deployment with 3 replicas
- **service.yaml**: ClusterIP service
- **configmap.yaml**: Configuration (log level, embedding model)
- **persistent-volume.yaml**: PVC for data storage
- **ingress.yaml**: Ingress rules (optional)
- **secrets.yaml.example**: Example secrets file
- **kustomization.yaml**: Kustomize configuration

## Deployment Steps

### 1. Create Namespace (Optional)

```bash
kubectl create namespace avocadodb
kubectl config set-context --current --namespace=avocadodb
```

### 2. Create Secrets (if using OpenAI)

```bash
# From literal
kubectl create secret generic avocadodb-secrets \
  --from-literal=openai_api_key=sk-your-key-here

# From file
kubectl create secret generic avocadodb-secrets \
  --from-file=openai_api_key=./openai.key

# Verify
kubectl get secret avocadodb-secrets
```

### 3. Deploy Resources

```bash
# Deploy everything
kubectl apply -f k8s/

# Or using kustomize
kubectl apply -k k8s/

# Verify
kubectl get all -l app=avocadodb
```

### 4. Verify Deployment

```bash
# Check pods
kubectl get pods -l app=avocadodb
kubectl describe pod -l app=avocadodb

# Check services
kubectl get svc avocadodb

# Check logs
kubectl logs -l app=avocadodb --tail=100 -f

# Check health
kubectl exec -it deployment/avocadodb -- sh -c "curl http://localhost:8765/health"
```

### 5. Access the Service

```bash
# Port forward for local access
kubectl port-forward svc/avocadodb 8765:8765

# Test
curl http://localhost:8765/health

# Or get service URL (if LoadBalancer)
kubectl get svc avocadodb -o jsonpath='{.status.loadBalancer.ingress[0].ip}'
```

## Configuration

### Environment Variables

Modify `configmap.yaml`:

```yaml
data:
  log_level: "info"           # trace, debug, info, warn, error
  embedding_model: "nomic"    # minilm, nomic, bgelarge
  embedding_provider: "local" # local or openai
```

Apply changes:
```bash
kubectl apply -f k8s/configmap.yaml
kubectl rollout restart deployment/avocadodb
```

### Resource Limits

Modify `deployment.yaml`:

```yaml
resources:
  requests:
    memory: "512Mi"
    cpu: "250m"
  limits:
    memory: "2Gi"
    cpu: "1000m"
```

### Scaling

```bash
# Manual scaling
kubectl scale deployment avocadodb --replicas=5

# Auto-scaling (HPA)
kubectl autoscale deployment avocadodb \
  --cpu-percent=70 \
  --min=3 \
  --max=10

# Verify
kubectl get hpa
```

## Storage

### Dynamic Provisioning

Most cloud providers support dynamic provisioning:

```yaml
# persistent-volume.yaml
spec:
  storageClassName: fast-ssd  # gp2 (AWS), pd-ssd (GCP), managed-premium (Azure)
  resources:
    requests:
      storage: 10Gi
```

### Static Provisioning

For on-premise or custom storage:

```yaml
apiVersion: v1
kind: PersistentVolume
metadata:
  name: avocadodb-pv
spec:
  capacity:
    storage: 10Gi
  accessModes:
    - ReadWriteOnce
  hostPath:
    path: /mnt/data/avocadodb
```

### Shared Storage (Multi-Replica)

For multiple replicas with shared data:

```yaml
spec:
  accessModes:
    - ReadWriteMany
  storageClassName: nfs-storage  # or efs-storage, azurefile
```

## Ingress

### NGINX Ingress

```bash
# Install NGINX ingress controller
kubectl apply -f https://raw.githubusercontent.com/kubernetes/ingress-nginx/main/deploy/static/provider/cloud/deploy.yaml

# Update ingress.yaml with your domain
# host: avocadodb.example.com

# Deploy ingress
kubectl apply -f k8s/ingress.yaml

# Get ingress IP
kubectl get ingress avocadodb
```

### TLS/HTTPS (cert-manager)

```bash
# Install cert-manager
kubectl apply -f https://github.com/cert-manager/cert-manager/releases/download/v1.13.0/cert-manager.yaml

# Create ClusterIssuer
cat <<EOF | kubectl apply -f -
apiVersion: cert-manager.io/v1
kind: ClusterIssuer
metadata:
  name: letsencrypt-prod
spec:
  acme:
    server: https://acme-v02.api.letsencrypt.org/directory
    email: your-email@example.com
    privateKeySecretRef:
      name: letsencrypt-prod
    solvers:
    - http01:
        ingress:
          class: nginx
EOF

# Update ingress.yaml
# annotations:
#   cert-manager.io/cluster-issuer: "letsencrypt-prod"
# tls:
#   - hosts:
#     - avocadodb.example.com
#     secretName: avocadodb-tls
```

## Monitoring

### Readiness and Liveness Probes

Already configured in `deployment.yaml`:

```yaml
livenessProbe:
  httpGet:
    path: /health
    port: http
  initialDelaySeconds: 10
  periodSeconds: 30

readinessProbe:
  httpGet:
    path: /health
    port: http
  initialDelaySeconds: 5
  periodSeconds: 10
```

### Prometheus Metrics (Future)

```yaml
# Add to deployment.yaml annotations
metadata:
  annotations:
    prometheus.io/scrape: "true"
    prometheus.io/port: "8765"
    prometheus.io/path: "/metrics"
```

### Logs

```bash
# Follow logs from all pods
kubectl logs -f -l app=avocadodb --all-containers=true

# Logs from specific pod
kubectl logs -f avocadodb-xxxxx-yyyyy

# Previous container logs (after crash)
kubectl logs -p avocadodb-xxxxx-yyyyy

# Export logs to file
kubectl logs -l app=avocadodb > avocadodb.log
```

## Backup and Restore

### Backup

```bash
# Using kubectl cp
POD=$(kubectl get pod -l app=avocadodb -o jsonpath='{.items[0].metadata.name}')
kubectl cp $POD:/data ./backup/

# Using volume snapshot (if supported)
cat <<EOF | kubectl apply -f -
apiVersion: snapshot.storage.k8s.io/v1
kind: VolumeSnapshot
metadata:
  name: avocadodb-snapshot
spec:
  volumeSnapshotClassName: csi-hostpath-snapclass
  source:
    persistentVolumeClaimName: avocadodb-pvc
EOF
```

### Restore

```bash
# Copy data back to pod
kubectl cp ./backup/ $POD:/data/

# Or restore from snapshot
cat <<EOF | kubectl apply -f -
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: avocadodb-pvc-restored
spec:
  dataSource:
    name: avocadodb-snapshot
    kind: VolumeSnapshot
    apiGroup: snapshot.storage.k8s.io
  accessModes:
    - ReadWriteOnce
  resources:
    requests:
      storage: 10Gi
EOF
```

## Environments (Kustomize Overlays)

Create environment-specific configurations:

```bash
k8s/
├── base/
│   ├── deployment.yaml
│   ├── service.yaml
│   └── kustomization.yaml
└── overlays/
    ├── dev/
    │   ├── kustomization.yaml
    │   └── replicas.yaml
    ├── staging/
    │   └── kustomization.yaml
    └── production/
        ├── kustomization.yaml
        ├── replicas.yaml
        └── resources.yaml
```

**overlays/production/kustomization.yaml**:
```yaml
apiVersion: kustomize.config.k8s.io/v1beta1
kind: Kustomization

namespace: avocadodb-prod

bases:
  - ../../base

replicas:
  - name: avocadodb
    count: 5

images:
  - name: avocadodb/avocadodb
    newTag: v0.1.0

patchesStrategicMerge:
  - replicas.yaml
  - resources.yaml
```

Deploy:
```bash
kubectl apply -k k8s/overlays/production
```

## Troubleshooting

### Pods not starting

```bash
# Check pod status
kubectl get pods -l app=avocadodb
kubectl describe pod -l app=avocadodb

# Check events
kubectl get events --sort-by=.metadata.creationTimestamp

# Check logs
kubectl logs -l app=avocadodb --tail=100
```

### PVC not binding

```bash
# Check PVC status
kubectl get pvc avocadodb-pvc
kubectl describe pvc avocadodb-pvc

# Check available storage classes
kubectl get storageclass

# Check PV
kubectl get pv
```

### Service not accessible

```bash
# Check service
kubectl get svc avocadodb
kubectl describe svc avocadodb

# Check endpoints
kubectl get endpoints avocadodb

# Test from another pod
kubectl run -it --rm debug --image=curlimages/curl --restart=Never -- \
  curl http://avocadodb:8765/health
```

### High resource usage

```bash
# Check resource usage
kubectl top pods -l app=avocadodb
kubectl top nodes

# Scale down
kubectl scale deployment avocadodb --replicas=1

# Update resource limits
kubectl set resources deployment avocadodb \
  --limits=cpu=2,memory=4Gi \
  --requests=cpu=1,memory=2Gi
```

## Security

### Network Policies

```yaml
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: avocadodb-netpol
spec:
  podSelector:
    matchLabels:
      app: avocadodb
  policyTypes:
  - Ingress
  ingress:
  - from:
    - namespaceSelector:
        matchLabels:
          name: ingress-nginx
    ports:
    - protocol: TCP
      port: 8765
```

### Pod Security

Already configured in `deployment.yaml`:
- Non-root user (UID 1000)
- Read-only root filesystem (where possible)
- Dropped all capabilities
- No privilege escalation

### RBAC

Create service account with minimal permissions:

```yaml
apiVersion: v1
kind: ServiceAccount
metadata:
  name: avocadodb
---
apiVersion: rbac.authorization.k8s.io/v1
kind: Role
metadata:
  name: avocadodb
rules:
- apiGroups: [""]
  resources: ["configmaps", "secrets"]
  verbs: ["get", "list"]
---
apiVersion: rbac.authorization.k8s.io/v1
kind: RoleBinding
metadata:
  name: avocadodb
roleRef:
  apiGroup: rbac.authorization.k8s.io
  kind: Role
  name: avocadodb
subjects:
- kind: ServiceAccount
  name: avocadodb
```

## Cloud-Specific Guides

### AWS EKS

```bash
# Create cluster
eksctl create cluster --name avocadodb --region us-west-2

# Use EBS for storage
# storageClassName: gp3

# Use ALB ingress
# See ingress.yaml for ALB annotations
```

### Google GKE

```bash
# Create cluster
gcloud container clusters create avocadodb --zone us-central1-a

# Use persistent disk
# storageClassName: pd-ssd

# Use GCE ingress (default) or NGINX
```

### Azure AKS

```bash
# Create cluster
az aks create --name avocadodb --resource-group myResourceGroup

# Use Azure disk
# storageClassName: managed-premium

# Use Azure Application Gateway or NGINX ingress
```

## CI/CD Integration

### GitHub Actions

See `.github/workflows/docker.yml` for building and pushing images.

Deploy step:
```yaml
- name: Deploy to Kubernetes
  run: |
    kubectl apply -k k8s/overlays/production
    kubectl rollout status deployment/avocadodb
```

### ArgoCD (GitOps)

```yaml
apiVersion: argoproj.io/v1alpha1
kind: Application
metadata:
  name: avocadodb
  namespace: argocd
spec:
  project: default
  source:
    repoURL: https://github.com/avocadodb/avocadodb
    targetRevision: main
    path: k8s/overlays/production
  destination:
    server: https://kubernetes.default.svc
    namespace: avocadodb
  syncPolicy:
    automated:
      prune: true
      selfHeal: true
```

## Next Steps

- [Docker Guide](./DOCKER.md) - Docker deployment
- [API Documentation](./API.md) - HTTP API reference
- [Performance Tuning](./performance.md) - Optimize for production

## Support

- Issues: https://github.com/avocadodb/avocadodb/issues
- Documentation: https://avocadodb.dev/docs
