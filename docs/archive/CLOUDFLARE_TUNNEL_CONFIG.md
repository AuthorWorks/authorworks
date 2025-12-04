# Cloudflare Tunnel Configuration for AuthorWorks

## Overview
This document outlines the Cloudflare tunnel configuration required to expose the AuthorWorks application running on your Aleph homelab server to the internet via the `leopaska.xyz` domain.

## Required DNS Records

Configure the following DNS records in your Cloudflare dashboard for the `leopaska.xyz` domain:

### Main Application
- **Type**: CNAME
- **Name**: `authorworks`
- **Target**: `<tunnel-id>.cfargotunnel.com`
- **Proxy Status**: Proxied (orange cloud)

### API Endpoint
- **Type**: CNAME
- **Name**: `api.authorworks`
- **Target**: `<tunnel-id>.cfargotunnel.com`
- **Proxy Status**: Proxied (orange cloud)

### Tenant Subdomains
- **Type**: CNAME
- **Name**: `tenant1.authorworks`
- **Target**: `<tunnel-id>.cfargotunnel.com`
- **Proxy Status**: Proxied (orange cloud)

- **Type**: CNAME
- **Name**: `tenant2.authorworks`
- **Target**: `<tunnel-id>.cfargotunnel.com`
- **Proxy Status**: Proxied (orange cloud)

## Cloudflare Tunnel Configuration

### 1. Install cloudflared on Aleph Server
```bash
# Download and install cloudflared
curl -L --output cloudflared.deb https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-amd64.deb
sudo dpkg -i cloudflared.deb
```

### 2. Authenticate with Cloudflare
```bash
cloudflared tunnel login
```

### 3. Create Tunnel
```bash
cloudflared tunnel create authorworks-tunnel
```

### 4. Configure Tunnel
Create `/etc/cloudflared/config.yml`:
```yaml
tunnel: <tunnel-id>
credentials-file: /etc/cloudflared/<tunnel-id>.json

ingress:
  # Main AuthorWorks application
  - hostname: authorworks.leopaska.xyz
    service: http://localhost:80
    originRequest:
      httpHostHeader: authorworks.leopaska.xyz
  
  # API endpoint
  - hostname: api.authorworks.leopaska.xyz
    service: http://localhost:80
    originRequest:
      httpHostHeader: api.authorworks.leopaska.xyz
  
  # Tenant 1
  - hostname: tenant1.authorworks.leopaska.xyz
    service: http://localhost:80
    originRequest:
      httpHostHeader: tenant1.authorworks.leopaska.xyz
  
  # Tenant 2
  - hostname: tenant2.authorworks.leopaska.xyz
    service: http://localhost:80
    originRequest:
      httpHostHeader: tenant2.authorworks.leopaska.xyz
  
  # Catch-all rule (required)
  - service: http_status:404
```

### 5. Install as System Service
```bash
sudo cloudflared service install
sudo systemctl enable cloudflared
sudo systemctl start cloudflared
```

### 6. Verify Tunnel Status
```bash
sudo systemctl status cloudflared
cloudflared tunnel info authorworks-tunnel
```

## K3s Integration

The tunnel will route traffic to your K3s cluster's ingress controller. Ensure:

1. **Nginx Ingress Controller** is installed and running
2. **cert-manager** is configured for SSL certificates
3. **AuthorWorks ingress resources** are properly configured

### Check Ingress Status
```bash
kubectl get ingress -n authorworks
kubectl get certificates -n authorworks
```

## Security Considerations

### 1. Firewall Configuration
Ensure your Aleph server firewall only allows:
- Outbound HTTPS (443) to Cloudflare
- Local cluster traffic
- No direct inbound traffic (tunnel handles this)

### 2. SSL/TLS
- Cloudflare handles external SSL termination
- Internal traffic can use HTTP (encrypted by tunnel)
- cert-manager provides cluster-internal certificates

### 3. Access Control
Configure Cloudflare Access rules if needed:
- Geographic restrictions
- IP allowlists
- Authentication requirements

## Monitoring and Troubleshooting

### Check Tunnel Health
```bash
# View tunnel logs
sudo journalctl -u cloudflared -f

# Test connectivity
curl -H "Host: authorworks.leopaska.xyz" http://localhost:80/health
```

### Common Issues

1. **DNS Propagation**: Allow 24-48 hours for full DNS propagation
2. **Certificate Issues**: Check cert-manager logs if SSL fails
3. **Ingress Routing**: Verify K3s ingress controller is routing correctly

### Health Check Endpoints
- Main: `https://authorworks.leopaska.xyz/health`
- API: `https://api.authorworks.leopaska.xyz/health`
- Tenant 1: `https://tenant1.authorworks.leopaska.xyz/health`
- Tenant 2: `https://tenant2.authorworks.leopaska.xyz/health`

## Backup Configuration

Store these files securely:
- `/etc/cloudflared/config.yml`
- `/etc/cloudflared/<tunnel-id>.json`
- Tunnel ID and credentials

## Performance Optimization

### Cloudflare Settings
- Enable **Brotli Compression**
- Configure **Caching Rules** for static assets
- Set appropriate **Security Level**
- Enable **HTTP/3** if supported

### Tunnel Optimization
```yaml
# Add to config.yml for better performance
originRequest:
  connectTimeout: 30s
  tlsTimeout: 10s
  tcpKeepAlive: 30s
  keepAliveConnections: 10
  keepAliveTimeout: 90s
```

This configuration ensures your AuthorWorks application is securely and efficiently exposed to the internet through Cloudflare's global network.
