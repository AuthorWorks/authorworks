# Technical Specification: 2E - Storage Service

## Overview

The Storage Service is responsible for managing all file storage operations in the AuthorWorks platform. It provides a consistent interface for uploading, retrieving, and managing various types of files, including user-generated content, media assets, and platform resources. This service abstracts the underlying storage systems and implements appropriate access controls, optimization strategies, and metadata management.

## Objectives

- Provide a unified API for file storage operations
- Abstract underlying storage providers (S3, local file system, etc.)
- Implement secure access controls for stored content
- Support efficient media handling including transforms and optimizations
- Enable metadata management for stored assets
- Implement versioning for stored files
- Support bandwidth optimization through CDN integration

## Requirements

### 1. Core Storage Data Models

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredFile {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub bucket: String,
    pub key: String,
    pub filename: String,
    pub content_type: String,
    pub size_bytes: u64,
    pub checksum: String,
    pub metadata: HashMap<String, String>,
    pub access_level: AccessLevel,
    pub status: FileStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_accessed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AccessLevel {
    Private,         // Owner only
    Restricted,      // Owner and specific users/roles
    Public,          // Anyone with link
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileStatus {
    Pending,         // Upload initiated but not completed
    Available,       // File is available for access
    Processing,      // File is being processed (e.g., optimization)
    Archived,        // File has been archived (slower access)
    Deleted,         // Marked for deletion but not yet removed
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileVersion {
    pub id: Uuid,
    pub file_id: Uuid,
    pub version_number: i32,
    pub size_bytes: u64,
    pub checksum: String,
    pub created_at: DateTime<Utc>,
    pub created_by: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoragePermission {
    pub id: Uuid,
    pub file_id: Uuid,
    pub grantee_id: Uuid,
    pub permission_type: PermissionType,
    pub created_at: DateTime<Utc>,
    pub created_by: Uuid,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PermissionType {
    Read,
    Write,
    Delete,
    Admin,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageQuota {
    pub user_id: Uuid,
    pub total_bytes_used: u64,
    pub max_bytes_allowed: u64,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedUrl {
    pub url: String,
    pub expires_at: DateTime<Utc>,
}
```

### 2. Database Schema

```sql
CREATE TABLE stored_files (
    id UUID PRIMARY KEY,
    owner_id UUID NOT NULL REFERENCES users(id),
    bucket VARCHAR(255) NOT NULL,
    key VARCHAR(1024) NOT NULL,
    filename VARCHAR(512) NOT NULL,
    content_type VARCHAR(255) NOT NULL,
    size_bytes BIGINT NOT NULL,
    checksum VARCHAR(64) NOT NULL,
    metadata JSONB NOT NULL DEFAULT '{}',
    access_level VARCHAR(20) NOT NULL,
    status VARCHAR(20) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL,
    last_accessed_at TIMESTAMP WITH TIME ZONE,
    UNIQUE (bucket, key)
);

CREATE TABLE file_versions (
    id UUID PRIMARY KEY,
    file_id UUID NOT NULL REFERENCES stored_files(id) ON DELETE CASCADE,
    version_number INTEGER NOT NULL,
    size_bytes BIGINT NOT NULL,
    checksum VARCHAR(64) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    created_by UUID NOT NULL REFERENCES users(id),
    UNIQUE (file_id, version_number)
);

CREATE TABLE storage_permissions (
    id UUID PRIMARY KEY,
    file_id UUID NOT NULL REFERENCES stored_files(id) ON DELETE CASCADE,
    grantee_id UUID NOT NULL REFERENCES users(id),
    permission_type VARCHAR(20) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    created_by UUID NOT NULL REFERENCES users(id),
    expires_at TIMESTAMP WITH TIME ZONE,
    UNIQUE (file_id, grantee_id, permission_type)
);

CREATE TABLE storage_quotas (
    user_id UUID PRIMARY KEY REFERENCES users(id),
    total_bytes_used BIGINT NOT NULL DEFAULT 0,
    max_bytes_allowed BIGINT NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL
);

-- Indexes
CREATE INDEX idx_stored_files_owner ON stored_files(owner_id);
CREATE INDEX idx_stored_files_bucket_key ON stored_files(bucket, key);
CREATE INDEX idx_stored_files_status ON stored_files(status);
CREATE INDEX idx_file_versions_file ON file_versions(file_id);
CREATE INDEX idx_storage_permissions_file ON storage_permissions(file_id);
CREATE INDEX idx_storage_permissions_grantee ON storage_permissions(grantee_id);
```

### 3. API Endpoints

```
# File Management
POST   /v1/files                        - Upload a file
GET    /v1/files                        - List files
GET    /v1/files/{id}                   - Get file metadata
PUT    /v1/files/{id}                   - Update file metadata
DELETE /v1/files/{id}                   - Delete file
GET    /v1/files/{id}/content           - Download file content
GET    /v1/files/{id}/url               - Generate signed URL for file

# File Versions
POST   /v1/files/{id}/versions          - Create new file version
GET    /v1/files/{id}/versions          - List file versions
GET    /v1/files/{id}/versions/{number} - Get specific version metadata
GET    /v1/files/{id}/versions/{number}/content - Download specific version

# Access Control
GET    /v1/files/{id}/permissions       - List permissions for file
POST   /v1/files/{id}/permissions       - Grant permission
DELETE /v1/files/{id}/permissions/{id}  - Revoke permission

# Storage Management
GET    /v1/storage/quota                - Get current user's storage quota
POST   /v1/storage/copy                 - Copy file to new location
POST   /v1/storage/move                 - Move file to new location
GET    /v1/storage/presigned-upload     - Get presigned URL for direct upload
POST   /v1/storage/complete-upload      - Complete multipart upload

# Folder Operations (virtual folders)
POST   /v1/folders                      - Create a folder
GET    /v1/folders                      - List folders
GET    /v1/folders/{path}               - List folder contents
DELETE /v1/folders/{path}               - Delete folder and contents
```

### 4. Storage Provider Interface

The Storage Service will implement a provider interface to abstract different storage backends:

```rust
#[async_trait]
pub trait StorageProvider: Send + Sync {
    async fn put_object(
        &self,
        bucket: &str,
        key: &str,
        content: &[u8],
        content_type: &str,
        metadata: &HashMap<String, String>,
    ) -> Result<StoredObjectInfo, StorageError>;
    
    async fn get_object(
        &self,
        bucket: &str,
        key: &str,
    ) -> Result<GetObjectResponse, StorageError>;
    
    async fn delete_object(
        &self,
        bucket: &str,
        key: &str,
    ) -> Result<(), StorageError>;
    
    async fn list_objects(
        &self,
        bucket: &str,
        prefix: &str,
        continuation_token: Option<&str>,
        max_keys: i32,
    ) -> Result<ListObjectsResponse, StorageError>;
    
    async fn copy_object(
        &self,
        source_bucket: &str,
        source_key: &str,
        dest_bucket: &str,
        dest_key: &str,
    ) -> Result<StoredObjectInfo, StorageError>;
    
    async fn generate_presigned_url(
        &self,
        bucket: &str,
        key: &str,
        expires_in: Duration,
        operation: UrlOperation,
    ) -> Result<String, StorageError>;
    
    async fn create_multipart_upload(
        &self,
        bucket: &str,
        key: &str,
        content_type: &str,
        metadata: &HashMap<String, String>,
    ) -> Result<String, StorageError>;
    
    async fn complete_multipart_upload(
        &self,
        bucket: &str,
        key: &str,
        upload_id: &str,
        parts: Vec<CompletedPart>,
    ) -> Result<StoredObjectInfo, StorageError>;
}

pub struct StoredObjectInfo {
    pub key: String,
    pub size_bytes: u64,
    pub etag: String,
    pub last_modified: DateTime<Utc>,
}

pub struct GetObjectResponse {
    pub content: Vec<u8>,
    pub content_type: String,
    pub size_bytes: u64,
    pub etag: String,
    pub last_modified: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
}

pub struct ListObjectsResponse {
    pub contents: Vec<StoredObjectInfo>,
    pub common_prefixes: Vec<String>,
    pub continuation_token: Option<String>,
    pub is_truncated: bool,
}

pub enum UrlOperation {
    Get,
    Put,
    Delete,
}

pub struct CompletedPart {
    pub part_number: i32,
    pub etag: String,
}
```

### 5. S3 Provider Implementation

```rust
pub struct S3Provider {
    client: S3Client,
    region: Region,
}

impl S3Provider {
    pub fn new(region: Region, credentials: AwsCredentials) -> Self {
        let client = S3Client::new_with(
            HttpClient::new().unwrap(),
            credentials,
            region.clone(),
        );
        
        Self {
            client,
            region,
        }
    }
}

#[async_trait]
impl StorageProvider for S3Provider {
    async fn put_object(
        &self,
        bucket: &str,
        key: &str,
        content: &[u8],
        content_type: &str,
        metadata: &HashMap<String, String>,
    ) -> Result<StoredObjectInfo, StorageError> {
        let request = PutObjectRequest {
            bucket: bucket.to_string(),
            key: key.to_string(),
            body: Some(ByteStream::from(content.to_vec())),
            content_type: Some(content_type.to_string()),
            metadata: Some(metadata.clone()),
            ..Default::default()
        };
        
        let response = self.client.put_object(request).await
            .map_err(|e| StorageError::ProviderError(format!("Failed to put object: {}", e)))?;
        
        let etag = response.e_tag
            .ok_or_else(|| StorageError::ProviderError("No ETag returned".to_string()))?;
        
        // Get object metadata to return complete info
        let head_request = HeadObjectRequest {
            bucket: bucket.to_string(),
            key: key.to_string(),
            ..Default::default()
        };
        
        let head_response = self.client.head_object(head_request).await
            .map_err(|e| StorageError::ProviderError(format!("Failed to get object metadata: {}", e)))?;
        
        let size_bytes = head_response.content_length.unwrap_or(0) as u64;
        let last_modified = head_response.last_modified
            .map(|t| DateTime::from(t))
            .unwrap_or_else(Utc::now);
        
        Ok(StoredObjectInfo {
            key: key.to_string(),
            size_bytes,
            etag: etag.trim_matches('"').to_string(),
            last_modified,
        })
    }
    
    async fn get_object(
        &self,
        bucket: &str,
        key: &str,
    ) -> Result<GetObjectResponse, StorageError> {
        let request = GetObjectRequest {
            bucket: bucket.to_string(),
            key: key.to_string(),
            ..Default::default()
        };
        
        let response = self.client.get_object(request).await
            .map_err(|e| StorageError::ProviderError(format!("Failed to get object: {}", e)))?;
        
        let content_type = response.content_type
            .unwrap_or_else(|| "application/octet-stream".to_string());
            
        let size_bytes = response.content_length.unwrap_or(0) as u64;
        
        let etag = response.e_tag
            .ok_or_else(|| StorageError::ProviderError("No ETag returned".to_string()))?
            .trim_matches('"')
            .to_string();
            
        let last_modified = response.last_modified
            .map(|t| DateTime::from(t))
            .unwrap_or_else(Utc::now);
            
        let metadata = response.metadata.unwrap_or_default();
        
        let body = response.body.ok_or_else(|| 
            StorageError::ProviderError("No body returned".to_string()))?;
            
        let content = body.collect().await
            .map_err(|e| StorageError::ProviderError(format!("Failed to read body: {}", e)))?
            .to_vec();
        
        Ok(GetObjectResponse {
            content,
            content_type,
            size_bytes,
            etag,
            last_modified,
            metadata,
        })
    }
    
    // Other method implementations are similar and follow AWS SDK patterns
    // For brevity, they are not included here
}
```

### 6. File Storage Service

```rust
pub struct FileStorageService {
    storage_provider: Arc<dyn StorageProvider>,
    file_repository: Arc<dyn FileRepository>,
    version_repository: Arc<dyn VersionRepository>,
    permission_repository: Arc<dyn PermissionRepository>,
    quota_repository: Arc<dyn QuotaRepository>,
}

impl FileStorageService {
    pub async fn upload_file(
        &self,
        owner_id: &Uuid,
        filename: &str,
        content: &[u8],
        content_type: &str,
        bucket: &str,
        access_level: AccessLevel,
        metadata: HashMap<String, String>,
    ) -> Result<StoredFile, StorageError> {
        // Check user quota
        self.check_user_quota(owner_id, content.len() as u64).await?;
        
        // Generate a unique key for the file
        let key = self.generate_file_key(owner_id, filename);
        
        // Calculate checksum
        let checksum = self.calculate_checksum(content);
        
        // Upload to storage provider
        let object_info = self.storage_provider.put_object(
            bucket,
            &key,
            content,
            content_type,
            &metadata,
        ).await?;
        
        // Create file record
        let file = StoredFile {
            id: Uuid::new_v4(),
            owner_id: *owner_id,
            bucket: bucket.to_string(),
            key,
            filename: filename.to_string(),
            content_type: content_type.to_string(),
            size_bytes: object_info.size_bytes,
            checksum,
            metadata,
            access_level,
            status: FileStatus::Available,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_accessed_at: None,
        };
        
        let created_file = self.file_repository.create(file).await?;
        
        // Create initial version record
        let version = FileVersion {
            id: Uuid::new_v4(),
            file_id: created_file.id,
            version_number: 1,
            size_bytes: object_info.size_bytes,
            checksum,
            created_at: Utc::now(),
            created_by: *owner_id,
        };
        
        self.version_repository.create(version).await?;
        
        // Update user quota
        self.quota_repository.increase_usage(owner_id, object_info.size_bytes).await?;
        
        Ok(created_file)
    }
    
    pub async fn download_file(
        &self,
        requester_id: &Uuid,
        file_id: &Uuid,
    ) -> Result<(Vec<u8>, StoredFile), StorageError> {
        // Get file metadata
        let file = self.file_repository.find_by_id(file_id).await?;
        
        // Check permissions
        self.check_access_permission(requester_id, &file, PermissionType::Read).await?;
        
        // Get object from storage
        let response = self.storage_provider.get_object(
            &file.bucket,
            &file.key,
        ).await?;
        
        // Update last accessed timestamp
        self.file_repository.update_last_accessed(file_id).await?;
        
        Ok((response.content, file))
    }
    
    pub async fn delete_file(
        &self,
        requester_id: &Uuid,
        file_id: &Uuid,
    ) -> Result<(), StorageError> {
        // Get file metadata
        let file = self.file_repository.find_by_id(file_id).await?;
        
        // Check permissions
        self.check_access_permission(requester_id, &file, PermissionType::Delete).await?;
        
        // Delete from storage
        self.storage_provider.delete_object(
            &file.bucket,
            &file.key,
        ).await?;
        
        // Update user quota
        self.quota_repository.decrease_usage(&file.owner_id, file.size_bytes).await?;
        
        // Delete database record
        self.file_repository.delete(file_id).await?;
        
        // Permissions and versions will cascade delete due to database constraints
        
        Ok(())
    }
    
    pub async fn generate_signed_url(
        &self,
        requester_id: &Uuid,
        file_id: &Uuid,
        operation: UrlOperation,
        expires_in: Duration,
    ) -> Result<SignedUrl, StorageError> {
        // Get file metadata
        let file = self.file_repository.find_by_id(file_id).await?;
        
        // Check permissions
        let permission = match operation {
            UrlOperation::Get => PermissionType::Read,
            UrlOperation::Put => PermissionType::Write,
            UrlOperation::Delete => PermissionType::Delete,
        };
        
        self.check_access_permission(requester_id, &file, permission).await?;
        
        // Generate URL
        let url = self.storage_provider.generate_presigned_url(
            &file.bucket,
            &file.key,
            expires_in,
            operation,
        ).await?;
        
        let expires_at = Utc::now() + expires_in;
        
        Ok(SignedUrl {
            url,
            expires_at,
        })
    }
    
    pub async fn create_new_version(
        &self,
        requester_id: &Uuid,
        file_id: &Uuid,
        content: &[u8],
    ) -> Result<FileVersion, StorageError> {
        // Get file metadata
        let file = self.file_repository.find_by_id(file_id).await?;
        
        // Check permissions
        self.check_access_permission(requester_id, &file, PermissionType::Write).await?;
        
        // Check quota for increased size
        let size_diff = content.len() as i64 - file.size_bytes as i64;
        if size_diff > 0 {
            self.check_user_quota(&file.owner_id, size_diff as u64).await?;
        }
        
        // Calculate checksum
        let checksum = self.calculate_checksum(content);
        
        // Get latest version number
        let latest_version = self.version_repository.get_latest_version(file_id).await?;
        let new_version_number = latest_version.version_number + 1;
        
        // Generate version-specific key
        let version_key = format!("{}.v{}", file.key, new_version_number);
        
        // Upload to storage provider
        let object_info = self.storage_provider.put_object(
            &file.bucket,
            &version_key,
            content,
            &file.content_type,
            &file.metadata,
        ).await?;
        
        // Create version record
        let version = FileVersion {
            id: Uuid::new_v4(),
            file_id: *file_id,
            version_number: new_version_number,
            size_bytes: object_info.size_bytes,
            checksum,
            created_at: Utc::now(),
            created_by: *requester_id,
        };
        
        let created_version = self.version_repository.create(version).await?;
        
        // Update main file record with new size
        self.file_repository.update_size_and_checksum(
            file_id,
            object_info.size_bytes,
            &checksum,
        ).await?;
        
        // Update user quota for size difference
        if size_diff > 0 {
            self.quota_repository.increase_usage(&file.owner_id, size_diff as u64).await?;
        } else if size_diff < 0 {
            self.quota_repository.decrease_usage(&file.owner_id, (-size_diff) as u64).await?;
        }
        
        Ok(created_version)
    }
    
    pub async fn grant_permission(
        &self,
        granter_id: &Uuid,
        file_id: &Uuid,
        grantee_id: &Uuid,
        permission_type: PermissionType,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<StoragePermission, StorageError> {
        // Get file metadata
        let file = self.file_repository.find_by_id(file_id).await?;
        
        // Check if granter is owner or has admin permission
        if file.owner_id != *granter_id {
            let has_admin = self.permission_repository
                .check_permission(file_id, granter_id, &PermissionType::Admin)
                .await?;
                
            if !has_admin {
                return Err(StorageError::AccessDenied("Only owner or admin can grant permissions".to_string()));
            }
        }
        
        // Create permission record
        let permission = StoragePermission {
            id: Uuid::new_v4(),
            file_id: *file_id,
            grantee_id: *grantee_id,
            permission_type,
            created_at: Utc::now(),
            created_by: *granter_id,
            expires_at,
        };
        
        let created_permission = self.permission_repository.create(permission).await?;
        Ok(created_permission)
    }
    
    // Helper methods
    
    async fn check_access_permission(
        &self,
        user_id: &Uuid,
        file: &StoredFile,
        required_permission: PermissionType,
    ) -> Result<(), StorageError> {
        // Owner has all permissions
        if file.owner_id == *user_id {
            return Ok(());
        }
        
        // Check if file is public (for read operations)
        if file.access_level == AccessLevel::Public && required_permission == PermissionType::Read {
            return Ok(());
        }
        
        // Check explicit permissions
        let has_permission = self.permission_repository
            .check_permission(&file.id, user_id, &required_permission)
            .await?;
            
        if has_permission {
            return Ok(());
        }
        
        // Check for admin permission, which grants all permissions
        if required_permission != PermissionType::Admin {
            let has_admin = self.permission_repository
                .check_permission(&file.id, user_id, &PermissionType::Admin)
                .await?;
                
            if has_admin {
                return Ok(());
            }
        }
        
        Err(StorageError::AccessDenied("Insufficient permissions".to_string()))
    }
    
    async fn check_user_quota(&self, user_id: &Uuid, additional_bytes: u64) -> Result<(), StorageError> {
        let quota = self.quota_repository.get_quota(user_id).await?;
        
        let new_usage = quota.total_bytes_used + additional_bytes;
        if new_usage > quota.max_bytes_allowed {
            return Err(StorageError::QuotaExceeded(
                format!("Storage quota exceeded. Current: {}, Max: {}", 
                    quota.total_bytes_used, 
                    quota.max_bytes_allowed
                )
            ));
        }
        
        Ok(())
    }
    
    fn generate_file_key(&self, user_id: &Uuid, filename: &str) -> String {
        let timestamp = Utc::now().timestamp();
        let random_suffix = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(8)
            .map(char::from)
            .collect::<String>();
            
        format!("user/{}/{}-{}-{}", user_id, timestamp, random_suffix, filename)
    }
    
    fn calculate_checksum(&self, data: &[u8]) -> String {
        let mut sha256 = Sha256::new();
        sha256.update(data);
        hex::encode(sha256.finalize())
    }
}
```

### 7. Folder Management

Implement virtual folder functionality to organize files:

```rust
pub struct FolderService {
    file_repository: Arc<dyn FileRepository>,
    permission_repository: Arc<dyn PermissionRepository>,
}

impl FolderService {
    pub async fn list_folder_contents(
        &self,
        user_id: &Uuid,
        folder_path: &str,
        recursive: bool,
    ) -> Result<FolderContents, StorageError> {
        // Normalize path
        let normalized_path = self.normalize_path(folder_path);
        
        // List files in folder
        let files = self.file_repository
            .find_by_path_prefix(user_id, &normalized_path, recursive)
            .await?;
            
        // Extract subfolders from file paths
        let subfolders = if !recursive {
            self.extract_immediate_subfolders(&files, &normalized_path)
        } else {
            Vec::new()
        };
        
        Ok(FolderContents {
            path: normalized_path,
            files,
            subfolders,
        })
    }
    
    pub async fn create_folder(
        &self,
        user_id: &Uuid,
        folder_path: &str,
    ) -> Result<(), StorageError> {
        // Normalize path
        let normalized_path = self.normalize_path(folder_path);
        
        // Create an empty folder marker file
        let marker_key = format!("{}/folder-marker", normalized_path.trim_end_matches('/'));
        
        // Validate that the folder doesn't already exist
        let existing = self.file_repository
            .find_by_path_prefix(user_id, &normalized_path, false)
            .await?;
            
        if !existing.is_empty() {
            return Err(StorageError::AlreadyExists("Folder already exists".to_string()));
        }
        
        // Actually create the folder marker
        // This is a placeholder implementation - the actual implementation
        // would depend on the storage provider
        // ...
        
        Ok(())
    }
    
    pub async fn delete_folder(
        &self,
        user_id: &Uuid,
        folder_path: &str,
        recursive: bool,
    ) -> Result<u32, StorageError> {
        // Normalize path
        let normalized_path = self.normalize_path(folder_path);
        
        // Check if folder exists
        let files = self.file_repository
            .find_by_path_prefix(user_id, &normalized_path, recursive)
            .await?;
            
        if files.is_empty() {
            return Err(StorageError::NotFound("Folder not found".to_string()));
        }
        
        // If not recursive, check if folder is empty
        if !recursive {
            let subfolders = self.extract_immediate_subfolders(&files, &normalized_path);
            
            if !files.is_empty() || !subfolders.is_empty() {
                return Err(StorageError::NotEmpty("Folder is not empty".to_string()));
            }
        }
        
        // Delete all files
        let mut deleted_count = 0;
        
        for file in files {
            self.file_repository.delete(&file.id).await?;
            deleted_count += 1;
        }
        
        Ok(deleted_count)
    }
    
    // Helper methods
    
    fn normalize_path(&self, path: &str) -> String {
        let mut normalized = path.trim().to_string();
        
        // Ensure path starts with "/"
        if !normalized.starts_with('/') {
            normalized = format!("/{}", normalized);
        }
        
        // Ensure path ends with "/" if not empty
        if !normalized.ends_with('/') && normalized != "/" {
            normalized = format!("{}/", normalized);
        }
        
        normalized
    }
    
    fn extract_immediate_subfolders(&self, files: &[StoredFile], parent_path: &str) -> Vec<String> {
        let mut subfolders = HashSet::new();
        
        for file in files {
            // Skip the parent path prefix
            if let Some(relative_path) = file.key.strip_prefix(parent_path) {
                // Get the first segment of the path
                if let Some(index) = relative_path.find('/') {
                    let subfolder = &relative_path[0..index];
                    if !subfolder.is_empty() {
                        subfolders.insert(subfolder.to_string());
                    }
                }
            }
        }
        
        subfolders.into_iter().collect()
    }
}

pub struct FolderContents {
    pub path: String,
    pub files: Vec<StoredFile>,
    pub subfolders: Vec<String>,
}
```

### 8. Content Delivery Network Integration

```rust
pub struct CdnService {
    base_url: String,
    secret_key: String,
    ttl: Duration,
}

impl CdnService {
    pub fn new(base_url: String, secret_key: String, ttl: Duration) -> Self {
        Self {
            base_url,
            secret_key,
            ttl,
        }
    }
    
    pub fn generate_cdn_url(&self, file_path: &str, expires_in: Duration) -> String {
        let expires_at = Utc::now() + expires_in;
        let expires_timestamp = expires_at.timestamp();
        
        let signature_base = format!("{}{}", file_path, expires_timestamp);
        let signature = self.generate_signature(&signature_base);
        
        format!("{}/{}?expires={}&signature={}", 
            self.base_url.trim_end_matches('/'),
            file_path.trim_start_matches('/'),
            expires_timestamp,
            signature
        )
    }
    
    pub fn validate_cdn_url(&self, path: &str, expires: i64, signature: &str) -> bool {
        if Utc::now().timestamp() > expires {
            return false;
        }
        
        let signature_base = format!("{}{}", path, expires);
        let expected_signature = self.generate_signature(&signature_base);
        
        crypto::constant_time_eq(signature.as_bytes(), expected_signature.as_bytes())
    }
    
    fn generate_signature(&self, data: &str) -> String {
        let mut hmac = HmacSha256::new_from_slice(self.secret_key.as_bytes())
            .expect("HMAC can take key of any size");
            
        hmac.update(data.as_bytes());
        let result = hmac.finalize();
        
        hex::encode(result.into_bytes())
    }
}
```

## Implementation Steps

1. Set up project structure and database models
2. Implement S3 storage provider integration
3. Create core file operations (upload, download, delete)
4. Add file versioning system
5. Implement permission management
6. Create virtual folder functionality
7. Add quota management for users
8. Implement signed URL generation for direct access
9. Integrate with CDN for optimized content delivery
10. Add support for large file uploads (multipart)
11. Create health and monitoring endpoints
12. Implement bulk operations for efficiency
13. Add comprehensive tests for all functionality

## Technical Decisions

### Why Abstract Storage Providers?

The Storage Service uses a provider interface to abstract different storage backends because:
- Enables easy switching between storage providers (S3, Google Cloud Storage, etc.)
- Allows for local development without cloud dependencies
- Facilitates testing with mock implementations
- Supports potential future migration between providers
- Enables hybrid storage strategies based on file types or usage patterns

### Versioning Strategy

File versioning was implemented to:
- Maintain history of content changes
- Allow rollbacks to previous versions
- Support collaborative workflows where multiple users may modify files
- Protect against accidental data loss
- Enable audit trails of content modifications

### Virtual Folders vs. Real Folders

The service implements "virtual" folders (using path prefixes) rather than actual folders because:
- Most cloud object storage systems are flat namespaces with key prefixes
- Provides more flexibility in organizing and reorganizing content
- Reduces overhead of managing folder entities separately
- Simplifies operations like moves and renames
- Maintains compatibility with object storage paradigms

## Success Criteria

The Storage Service will be considered successfully implemented when:

1. Files can be uploaded, downloaded, and managed through the API
2. Access controls properly restrict or grant access to resources
3. Versioning system maintains history and allows retrieval of previous versions
4. Virtual folders provide intuitive organization of files
5. User quotas are properly enforced
6. CDN integration optimizes content delivery performance
7. Large file operations work reliably (both upload and download)
8. System is resilient to failures and properly handles edge cases
9. Performance meets requirements (upload/download speeds, response times)
10. All access patterns are properly secured against unauthorized access 