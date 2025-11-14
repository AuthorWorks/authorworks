# Technical Specification: 2B - User Service

## Overview

The User Service is responsible for all user-related functionality in the AuthorWorks platform. This includes user registration, authentication, profile management, subscription handling, and user preference storage. It serves as the central source of truth for user identity and related data.

## Objectives

- Provide secure user authentication and authorization
- Manage user profiles and associated metadata
- Support various authentication methods (email/password, OAuth, SSO)
- Enable subscription and membership tier management
- Store and retrieve user preferences
- Track user activity and engagement metrics

## Requirements

### 1. Core User Management

#### User Data Model

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub username: String,
    pub display_name: String,
    pub password_hash: Option<String>,
    pub auth_provider: Option<AuthProvider>,
    pub auth_provider_user_id: Option<String>,
    pub profile: Profile,
    pub roles: Vec<String>,
    pub status: UserStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_login_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthProvider {
    Local,
    Google,
    GitHub,
    Apple,
    Twitter,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UserStatus {
    Active,
    Inactive,
    Suspended,
    Unverified,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
    pub website: Option<String>,
    pub location: Option<String>,
    pub social_links: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreference {
    pub user_id: Uuid,
    pub key: String,
    pub value: serde_json::Value,
    pub updated_at: DateTime<Utc>,
}
```

#### Database Schema

```sql
CREATE TABLE users (
    id UUID PRIMARY KEY,
    email VARCHAR(255) UNIQUE NOT NULL,
    username VARCHAR(50) UNIQUE NOT NULL,
    display_name VARCHAR(100) NOT NULL,
    password_hash VARCHAR(255),
    auth_provider VARCHAR(20),
    auth_provider_user_id VARCHAR(255),
    status VARCHAR(20) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL,
    last_login_at TIMESTAMP WITH TIME ZONE
);

CREATE TABLE profiles (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    bio TEXT,
    avatar_url VARCHAR(255),
    website VARCHAR(255),
    location VARCHAR(255),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL
);

CREATE TABLE social_links (
    user_id UUID REFERENCES users(id) ON DELETE CASCADE,
    platform VARCHAR(50) NOT NULL,
    url VARCHAR(255) NOT NULL,
    PRIMARY KEY (user_id, platform)
);

CREATE TABLE user_roles (
    user_id UUID REFERENCES users(id) ON DELETE CASCADE,
    role VARCHAR(50) NOT NULL,
    PRIMARY KEY (user_id, role)
);

CREATE TABLE user_preferences (
    user_id UUID REFERENCES users(id) ON DELETE CASCADE,
    key VARCHAR(100) NOT NULL,
    value JSONB NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL,
    PRIMARY KEY (user_id, key)
);

CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_users_username ON users(username);
CREATE INDEX idx_users_auth_provider ON users(auth_provider, auth_provider_user_id) WHERE auth_provider IS NOT NULL;
```

#### API Endpoints

```
POST /v1/users/register                - Register a new user
POST /v1/users/login                   - Login with username/password
POST /v1/users/oauth/{provider}        - Login with OAuth provider
POST /v1/users/verify-email/{token}    - Verify email address
POST /v1/users/forgot-password         - Initiate password reset
POST /v1/users/reset-password/{token}  - Reset password with token
GET  /v1/users/me                      - Get current user profile
PUT  /v1/users/me                      - Update current user profile
DELETE /v1/users/me                    - Delete user account
GET  /v1/users/{id}                    - Get public profile by ID
GET  /v1/users/preferences             - Get all user preferences
GET  /v1/users/preferences/{key}       - Get specific preference
PUT  /v1/users/preferences/{key}       - Update specific preference
```

### 2. Authentication System

#### JWT-Based Authentication

The User Service will implement JWT-based authentication:

```rust
pub struct AuthService {
    user_repository: Arc<dyn UserRepository>,
    token_service: Arc<dyn TokenService>,
    password_service: Arc<dyn PasswordService>,
}

impl AuthService {
    pub async fn login(&self, credentials: LoginCredentials) -> Result<AuthTokens, Error> {
        // Verify credentials
        let user = self.user_repository.find_by_email(&credentials.email).await?;
        
        if user.status != UserStatus::Active {
            return Err(Error::AccountInactive);
        }
        
        if let Some(hash) = &user.password_hash {
            if !self.password_service.verify_password(&credentials.password, hash)? {
                return Err(Error::InvalidCredentials);
            }
        } else {
            return Err(Error::InvalidCredentials);
        }
        
        // Update last login timestamp
        self.user_repository.update_last_login(&user.id).await?;
        
        // Generate tokens
        let access_token = self.token_service.generate_access_token(&user)?;
        let refresh_token = self.token_service.generate_refresh_token(&user)?;
        
        Ok(AuthTokens {
            access_token,
            refresh_token,
            token_type: "Bearer".to_string(),
            expires_in: 3600,
        })
    }
    
    pub async fn register(&self, registration: RegistrationData) -> Result<User, Error> {
        // Validate registration data
        self.validate_registration(&registration).await?;
        
        // Hash password
        let password_hash = self.password_service.hash_password(&registration.password)?;
        
        // Create user
        let user = User {
            id: Uuid::new_v4(),
            email: registration.email,
            username: registration.username,
            display_name: registration.display_name,
            password_hash: Some(password_hash),
            auth_provider: Some(AuthProvider::Local),
            auth_provider_user_id: None,
            profile: Profile::default(),
            roles: vec!["user".to_string()],
            status: UserStatus::Unverified,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_login_at: None,
        };
        
        let created_user = self.user_repository.create(user).await?;
        
        // Send verification email
        // ...
        
        Ok(created_user)
    }
    
    // ... other authentication methods
}

pub struct TokenService {
    secret_key: Vec<u8>,
    access_token_expiry: Duration,
    refresh_token_expiry: Duration,
    issuer: String,
}

impl TokenService {
    pub fn generate_access_token(&self, user: &User) -> Result<String, Error> {
        let now = Utc::now();
        let expiry = now + self.access_token_expiry;
        
        let claims = Claims {
            sub: user.id.to_string(),
            exp: expiry.timestamp() as usize,
            iat: now.timestamp() as usize,
            iss: self.issuer.clone(),
            roles: user.roles.clone(),
        };
        
        let token = encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(&self.secret_key),
        )?;
        
        Ok(token)
    }
    
    // ... refresh token generation and validation
}
```

#### OAuth Integration

```rust
pub struct OAuthService {
    providers: HashMap<AuthProvider, Box<dyn OAuthProvider>>,
    user_repository: Arc<dyn UserRepository>,
    token_service: Arc<dyn TokenService>,
}

impl OAuthService {
    pub fn get_authorization_url(&self, provider: AuthProvider) -> Result<String, Error> {
        let oauth_provider = self.providers.get(&provider)
            .ok_or(Error::UnsupportedProvider)?;
            
        oauth_provider.get_authorization_url()
    }
    
    pub async fn handle_callback(
        &self, 
        provider: AuthProvider, 
        code: String,
    ) -> Result<AuthTokens, Error> {
        let oauth_provider = self.providers.get(&provider)
            .ok_or(Error::UnsupportedProvider)?;
            
        // Exchange code for tokens
        let oauth_tokens = oauth_provider.exchange_code(code).await?;
        
        // Get user profile from provider
        let profile = oauth_provider.get_user_profile(&oauth_tokens.access_token).await?;
        
        // Find or create user
        let user = self.find_or_create_user(provider, profile).await?;
        
        // Generate auth tokens
        let access_token = self.token_service.generate_access_token(&user)?;
        let refresh_token = self.token_service.generate_refresh_token(&user)?;
        
        Ok(AuthTokens {
            access_token,
            refresh_token,
            token_type: "Bearer".to_string(),
            expires_in: 3600,
        })
    }
    
    async fn find_or_create_user(
        &self,
        provider: AuthProvider,
        profile: OAuthUserProfile,
    ) -> Result<User, Error> {
        // Try to find user by provider ID
        if let Some(user) = self.user_repository
            .find_by_auth_provider(provider.clone(), &profile.id).await? {
            return Ok(user);
        }
        
        // Try to find user by email
        if let Some(email) = &profile.email {
            if let Some(user) = self.user_repository.find_by_email(email).await? {
                // Link existing account with provider
                let updated_user = self.user_repository
                    .link_auth_provider(&user.id, provider.clone(), &profile.id)
                    .await?;
                return Ok(updated_user);
            }
        }
        
        // Create new user
        let new_user = User {
            id: Uuid::new_v4(),
            email: profile.email.unwrap_or_else(|| format!("{}@{}", profile.id, provider)),
            username: self.generate_username_from_profile(&profile)?,
            display_name: profile.name.unwrap_or_else(|| "User".to_string()),
            password_hash: None,
            auth_provider: Some(provider.clone()),
            auth_provider_user_id: Some(profile.id),
            profile: Profile {
                avatar_url: profile.avatar_url,
                ..Profile::default()
            },
            roles: vec!["user".to_string()],
            status: UserStatus::Active,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_login_at: Some(Utc::now()),
        };
        
        let created_user = self.user_repository.create(new_user).await?;
        Ok(created_user)
    }
}
```

### 3. Profile Management

```rust
pub struct ProfileService {
    user_repository: Arc<dyn UserRepository>,
    profile_repository: Arc<dyn ProfileRepository>,
    storage_service: Arc<dyn StorageService>,
}

impl ProfileService {
    pub async fn get_profile(&self, user_id: &Uuid) -> Result<Profile, Error> {
        let profile = self.profile_repository.find_by_user_id(user_id).await?;
        Ok(profile)
    }
    
    pub async fn update_profile(
        &self, 
        user_id: &Uuid, 
        update: ProfileUpdate,
    ) -> Result<Profile, Error> {
        let mut profile = self.profile_repository.find_by_user_id(user_id).await?;
        
        // Update fields
        if let Some(bio) = update.bio {
            profile.bio = Some(bio);
        }
        
        if let Some(website) = update.website {
            profile.website = Some(website);
        }
        
        if let Some(location) = update.location {
            profile.location = Some(location);
        }
        
        if let Some(social_links) = update.social_links {
            profile.social_links = social_links;
        }
        
        // Save updated profile
        let updated_profile = self.profile_repository.update(user_id, &profile).await?;
        Ok(updated_profile)
    }
    
    pub async fn upload_avatar(
        &self, 
        user_id: &Uuid, 
        avatar: UploadedFile,
    ) -> Result<String, Error> {
        // Validate image
        if !["image/jpeg", "image/png", "image/gif"].contains(&avatar.content_type.as_str()) {
            return Err(Error::UnsupportedMediaType);
        }
        
        if avatar.content.len() > 5 * 1024 * 1024 {
            return Err(Error::FileTooLarge);
        }
        
        // Upload to storage
        let path = format!("avatars/{}/{}", user_id, Uuid::new_v4());
        let url = self.storage_service.upload_file(&path, &avatar.content).await?;
        
        // Update profile
        let mut profile = self.profile_repository.find_by_user_id(user_id).await?;
        profile.avatar_url = Some(url.clone());
        self.profile_repository.update(user_id, &profile).await?;
        
        Ok(url)
    }
}
```

### 4. User Preferences

```rust
pub struct PreferenceService {
    preference_repository: Arc<dyn PreferenceRepository>,
}

impl PreferenceService {
    pub async fn get_preferences(&self, user_id: &Uuid) -> Result<HashMap<String, serde_json::Value>, Error> {
        let preferences = self.preference_repository.find_all_by_user_id(user_id).await?;
        
        let mut result = HashMap::new();
        for pref in preferences {
            result.insert(pref.key, pref.value);
        }
        
        Ok(result)
    }
    
    pub async fn get_preference(
        &self, 
        user_id: &Uuid, 
        key: &str,
    ) -> Result<Option<serde_json::Value>, Error> {
        let preference = self.preference_repository.find_by_user_id_and_key(user_id, key).await?;
        Ok(preference.map(|p| p.value))
    }
    
    pub async fn set_preference(
        &self, 
        user_id: &Uuid, 
        key: &str, 
        value: serde_json::Value,
    ) -> Result<(), Error> {
        let preference = UserPreference {
            user_id: user_id.clone(),
            key: key.to_string(),
            value,
            updated_at: Utc::now(),
        };
        
        self.preference_repository.save(preference).await?;
        Ok(())
    }
}
```

### 5. Subscription Management

```rust
pub struct SubscriptionService {
    subscription_repository: Arc<dyn SubscriptionRepository>,
    payment_service: Arc<dyn PaymentService>,
    user_repository: Arc<dyn UserRepository>,
}

impl SubscriptionService {
    pub async fn get_user_subscription(&self, user_id: &Uuid) -> Result<Option<Subscription>, Error> {
        self.subscription_repository.find_active_by_user_id(user_id).await
    }
    
    pub async fn create_subscription(
        &self, 
        user_id: &Uuid, 
        plan_id: &str,
        payment_method_id: &str,
    ) -> Result<Subscription, Error> {
        // Verify user exists
        let user = self.user_repository.find_by_id(user_id).await?;
        
        // Check if user already has active subscription
        if let Some(subscription) = self.subscription_repository.find_active_by_user_id(user_id).await? {
            return Err(Error::SubscriptionAlreadyExists(subscription));
        }
        
        // Create payment intent with payment provider
        let payment_intent = self.payment_service.create_subscription_payment(
            user_id,
            plan_id,
            payment_method_id,
        ).await?;
        
        // Create subscription record
        let subscription = Subscription {
            id: Uuid::new_v4(),
            user_id: user_id.clone(),
            plan_id: plan_id.to_string(),
            status: SubscriptionStatus::Active,
            payment_provider: payment_intent.provider,
            payment_provider_subscription_id: payment_intent.subscription_id,
            current_period_start: Utc::now(),
            current_period_end: payment_intent.current_period_end,
            cancel_at_period_end: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            cancelled_at: None,
        };
        
        let created_subscription = self.subscription_repository.create(subscription).await?;
        
        // Update user roles based on subscription plan
        self.update_user_roles_for_plan(&user, plan_id).await?;
        
        Ok(created_subscription)
    }
    
    pub async fn cancel_subscription(
        &self, 
        user_id: &Uuid, 
        cancel_immediately: bool,
    ) -> Result<Subscription, Error> {
        // Find active subscription
        let subscription = self.subscription_repository
            .find_active_by_user_id(user_id)
            .await?
            .ok_or(Error::SubscriptionNotFound)?;
        
        // Cancel with payment provider
        self.payment_service.cancel_subscription(
            &subscription.payment_provider_subscription_id,
            cancel_immediately,
        ).await?;
        
        // Update subscription record
        let updated_subscription = if cancel_immediately {
            self.subscription_repository.update_status(
                &subscription.id,
                SubscriptionStatus::Cancelled,
                Some(Utc::now()),
            ).await?
        } else {
            self.subscription_repository.mark_cancel_at_period_end(
                &subscription.id,
            ).await?
        };
        
        // If cancelled immediately, update user roles
        if cancel_immediately {
            let user = self.user_repository.find_by_id(user_id).await?;
            self.remove_subscription_roles(&user).await?;
        }
        
        Ok(updated_subscription)
    }
    
    // Helper methods for role management
    async fn update_user_roles_for_plan(
        &self, 
        user: &User, 
        plan_id: &str,
    ) -> Result<(), Error> {
        let mut roles = user.roles.clone();
        
        // Remove any existing subscription roles
        roles.retain(|r| !r.starts_with("subscription:"));
        
        // Add role for new plan
        roles.push(format!("subscription:{}", plan_id));
        
        // Update user
        self.user_repository.update_roles(&user.id, roles).await?;
        
        Ok(())
    }
    
    async fn remove_subscription_roles(&self, user: &User) -> Result<(), Error> {
        let mut roles = user.roles.clone();
        
        // Remove any subscription roles
        roles.retain(|r| !r.starts_with("subscription:"));
        
        // Update user
        self.user_repository.update_roles(&user.id, roles).await?;
        
        Ok(())
    }
}
```

## Implementation Steps

1. Set up project structure and database models
2. Implement core user management functionality
3. Create JWT-based authentication system
4. Add OAuth provider integrations
5. Implement profile management
6. Add user preferences storage and retrieval
7. Create subscription management system
8. Implement webhooks for payment provider events
9. Add metrics collection and monitoring
10. Write comprehensive tests

## Technical Decisions

### Why PostgreSQL for User Data?

PostgreSQL was chosen for storing user data because:
- Strong data integrity guarantees with transactions and constraints
- Advanced JSON support for flexible user metadata
- Excellent indexing capabilities for query performance
- Mature ecosystem with proven reliability
- Native UUID support for identifiers

### Why JWT for Authentication?

JWT-based authentication was selected because:
- Stateless authentication model fitting microservices architecture
- Reduced database load for token validation
- Support for distributed verification across services
- Ability to include user claims directly in token
- Industry standard with solid library support

## Success Criteria

The User Service will be considered successfully implemented when:

1. Users can register, authenticate, and manage their profiles
2. Multiple authentication methods are supported (local, OAuth)
3. User preferences are correctly stored and retrieved
4. Subscription management works as expected
5. All API endpoints are secured appropriately
6. Service handles high load with minimal latency (<100ms average response time)
7. Comprehensive test coverage (>85%)
8. Proper error handling and meaningful error messages 