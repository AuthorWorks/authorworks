//! S3/MinIO client for the Media Worker

use anyhow::{Context, Result};
use aws_sdk_s3::{Client, Config, Credentials, Region};
use aws_sdk_s3::primitives::ByteStream;
use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncReadExt;

pub struct S3Client {
    client: Client,
    bucket: String,
}

impl S3Client {
    pub fn new(endpoint: &str, access_key: &str, secret_key: &str, bucket: &str) -> Self {
        let creds = Credentials::new(access_key, secret_key, None, None, "media-worker");
        
        let config = Config::builder()
            .endpoint_url(endpoint)
            .region(Region::new("us-east-1"))
            .credentials_provider(creds)
            .force_path_style(true)
            .build();

        let client = Client::from_conf(config);

        Self {
            client,
            bucket: bucket.to_string(),
        }
    }

    pub async fn download_file(&self, key: &str, local_path: &Path) -> Result<()> {
        let response = self.client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .context("Failed to get object from S3")?;

        let body = response.body.collect().await
            .context("Failed to read S3 response body")?;

        tokio::fs::write(local_path, body.into_bytes()).await
            .context("Failed to write file")?;

        Ok(())
    }

    pub async fn upload_file(&self, local_path: &Path, key: &str, content_type: &str) -> Result<()> {
        let mut file = File::open(local_path).await
            .context("Failed to open file for upload")?;

        let mut contents = Vec::new();
        file.read_to_end(&mut contents).await
            .context("Failed to read file")?;

        let body = ByteStream::from(contents);

        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(body)
            .content_type(content_type)
            .send()
            .await
            .context("Failed to upload to S3")?;

        Ok(())
    }

    pub async fn delete_file(&self, key: &str) -> Result<()> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .context("Failed to delete from S3")?;

        Ok(())
    }

    pub async fn file_exists(&self, key: &str) -> Result<bool> {
        match self.client
            .head_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
        {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}

