# AuthorWorks Authentication with Logto

AuthorWorks uses [Logto](https://logto.io/) for authentication and identity management.

## Overview

Logto provides:
- User registration and login
- OAuth2/OIDC compliance
- Social logins (Google, GitHub, Apple, Twitter)
- Multi-factor authentication
- User management admin console
- Passwordless authentication

## Architecture

```
┌──────────┐     ┌───────────┐     ┌──────────┐     ┌──────────────┐
│  Client  │────▶│API Gateway│────▶│  Logto   │────▶│ User Service │
│  (SPA)   │     │  (Nginx)  │     │  (Auth)  │     │   (Rust)     │
└──────────┘     └───────────┘     └──────────┘     └──────────────┘
```

## Endpoints

| Endpoint | Description |
|----------|-------------|
| `/auth/logto/authorize` | Initiate Logto OAuth flow |
| `/auth/callback` | OAuth callback handler |
| `/auth/login` | Email/password login |
| `/auth/register` | New user registration |
| `/auth/logout` | Logout and invalidate session |
| `/auth/refresh` | Refresh access token |

## Setup Guide

### 1. Initial Configuration

When you first deploy AuthorWorks, Logto will be available at:
- **Main endpoint**: http://localhost:3001 (or https://auth.your-domain.com)
- **Admin console**: http://localhost:3002 (or https://auth-admin.your-domain.com)

### 2. Create Application in Logto

1. Visit the Logto Admin Console
2. Go to **Applications** → **Create Application**
3. Select **Traditional Web Application**
4. Configure:
   - **Name**: `AuthorWorks`
   - **Redirect URI**: `http://localhost:8080/auth/callback`
   - **Post sign-out redirect URI**: `http://localhost:8080`

### 3. Configure Environment Variables

```bash
# Copy the Client ID and Client Secret from Logto
LOGTO_ENDPOINT=http://localhost:3001
LOGTO_CLIENT_ID=your-client-id
LOGTO_CLIENT_SECRET=your-client-secret
LOGTO_REDIRECT_URI=http://localhost:8080/auth/callback
```

### 4. Configure Social Connectors (Optional)

In Logto Admin Console:
1. Go to **Connectors**
2. Add social providers:
   - Google
   - GitHub
   - Apple
   - Twitter

## Authentication Flow

### Login Flow

1. User clicks "Login" button
2. Client redirects to `/auth/logto/authorize`
3. User Service redirects to Logto authorize endpoint
4. User authenticates in Logto
5. Logto redirects to `/auth/callback` with authorization code
6. User Service exchanges code for tokens
7. User Service returns JWT tokens to client

### Token Structure

```json
{
  "access_token": "eyJhbGciOiJIUzI1NiIs...",
  "refresh_token": "dGhpcyBpcyBhIHJlZnJlc2ggdG9rZW4...",
  "token_type": "Bearer",
  "expires_in": 3600
}
```

### JWT Claims

```json
{
  "sub": "user-uuid",
  "email": "user@example.com",
  "roles": ["user"],
  "exp": 1699999999,
  "iat": 1699996399,
  "iss": "authorworks",
  "aud": "authorworks-api"
}
```

## API Authentication

### Protected Endpoints

Include the JWT in the Authorization header:

```bash
curl -H "Authorization: Bearer <access_token>" \
  http://localhost:8080/api/users/me
```

### Token Refresh

```bash
curl -X POST http://localhost:8080/auth/refresh \
  -H "Content-Type: application/json" \
  -d '{"refresh_token": "<refresh_token>"}'
```

## Frontend Integration

### Login Button

```rust
// Leptos example
view! {
    <a href="/auth/logto/authorize" class="btn-login">
        "Login with Logto"
    </a>
}
```

### Protected Routes

```rust
// Check authentication state
let user = use_context::<UserContext>();

view! {
    {move || match user.get() {
        Some(user) => view! { <Dashboard user=user/> },
        None => view! { <Redirect path="/auth/logto/authorize"/> }
    }}
}
```

## Security Best Practices

1. **Always use HTTPS** in production
2. **Rotate secrets** regularly
3. **Validate tokens** on every request
4. **Use short-lived access tokens** (1 hour)
5. **Implement refresh token rotation**
6. **Store tokens securely** (httpOnly cookies or secure storage)

## Troubleshooting

### "Invalid redirect URI"
- Ensure the redirect URI in Logto matches exactly
- Check for trailing slashes

### "Token validation failed"
- Verify JWT_SECRET matches
- Check token expiration
- Ensure clock sync between services

### "CORS errors"
- Add your frontend origin to allowed origins
- Configure CORS headers in Nginx

## Multi-Environment Configuration

| Environment | Logto Endpoint | Redirect URI |
|-------------|----------------|--------------|
| Local | http://localhost:3001 | http://localhost:8080/auth/callback |
| Homelab | https://auth.authorworks.leopaska.xyz | https://authorworks.leopaska.xyz/auth/callback |
| Production | https://auth.authorworks.io | https://authorworks.io/auth/callback |

