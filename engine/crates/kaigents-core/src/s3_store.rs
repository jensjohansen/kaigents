//! File: engine/crates/kaigents-core/src/s3_store.rs
//! Purpose: Cloud-agnostic S3-compatible artifact storage implementation.
//! Product/business importance: enables production-grade, durable artifact storage across AWS, MinIO, and Ceph.
//!
//! Copyright (c) 2026 John K Johansen
//! License: MIT (see LICENSE)

use crate::artifacts::{Artifact, ArtifactMetadata, ArtifactStorageRef, ArtifactStore};
use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::config::Credentials;
use aws_sdk_s3::presigning::PresigningConfig;
use aws_sdk_s3::Client;
use std::time::Duration;

/// S3Config holds connection details for S3-compatible storage.
#[derive(Debug, Clone)]
pub struct S3Config {
    pub endpoint_url: String,
    pub region: String,
    pub bucket: String,
    pub access_key: String,
    pub secret_key: String,
    pub force_path_style: bool,
}

impl S3Config {
    pub fn from_env() -> Self {
        Self {
            endpoint_url: std::env::var("KAIGENTS_S3_ENDPOINT").unwrap_or_default(),
            region: std::env::var("KAIGENTS_S3_REGION").unwrap_or_else(|_| "us-east-1".to_string()),
            bucket: std::env::var("KAIGENTS_S3_BUCKET").unwrap_or_else(|_| "kaigents-artifacts".to_string()),
            access_key: std::env::var("KAIGENTS_S3_ACCESS_KEY").unwrap_or_default(),
            secret_key: std::env::var("KAIGENTS_S3_SECRET_KEY").unwrap_or_default(),
            force_path_style: std::env::var("KAIGENTS_S3_FORCE_PATH_STYLE").map(|v| v == "true").unwrap_or(true),
        }
    }
}

/// S3ArtifactStore implements ArtifactStore for S3-compatible backends.
pub struct S3ArtifactStore {
    client: Client,
    bucket: String,
}

impl S3ArtifactStore {
    pub async fn new(cfg: &S3Config) -> Self {
        let region_provider = RegionProviderChain::first_try(aws_sdk_s3::config::Region::new(cfg.region.clone()));
        
        let mut loader = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(region_provider);

        if !cfg.endpoint_url.is_empty() {
            loader = loader.endpoint_url(cfg.endpoint_url.clone());
        }

        if !cfg.access_key.is_empty() {
            let creds = Credentials::new(
                cfg.access_key.clone(),
                cfg.secret_key.clone(),
                None,
                None,
                "kaigents",
            );
            loader = loader.credentials_provider(creds);
        }

        let aws_cfg = loader.load().await;
        let s3_cfg = aws_sdk_s3::config::Builder::from(&aws_cfg)
            .force_path_style(cfg.force_path_style)
            .build();
        
        let client = Client::from_conf(s3_cfg);

        Self {
            client,
            bucket: cfg.bucket.clone(),
        }
    }
}

#[async_trait::async_trait]
impl ArtifactStore for S3ArtifactStore {
    async fn store(&self, artifact: &Artifact, data: Vec<u8>) -> Result<ArtifactStorageRef, String> {
        let key = format!("{}/{}", artifact.run_id.as_uuid(), artifact.name);
        
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&key)
            .body(data.into())
            .content_type(&artifact.mime_type)
            .send()
            .await
            .map_err(|e| format!("S3 put_object failed: {}", e))?;

        Ok(ArtifactStorageRef::ObjectStore {
            bucket: self.bucket.clone(),
            key,
        })
    }

    async fn retrieve(&self, storage_ref: &ArtifactStorageRef) -> Result<Vec<u8>, String> {
        if let ArtifactStorageRef::ObjectStore { bucket, key } = storage_ref {
            let output = self.client
                .get_object()
                .bucket(bucket)
                .key(key)
                .send()
                .await
                .map_err(|e| format!("S3 get_object failed: {}", e))?;

            let data = output.body.collect().await
                .map_err(|e| format!("S3 read body failed: {}", e))?;
            
            Ok(data.into_bytes().to_vec())
        } else {
            Err("Unsupported storage reference for S3 store".to_string())
        }
    }

    async fn generate_url(
        &self,
        storage_ref: &ArtifactStorageRef,
        expires_in: Duration,
    ) -> Result<String, String> {
        if let ArtifactStorageRef::ObjectStore { bucket, key } = storage_ref {
            let presigning_config = PresigningConfig::expires_in(expires_in)
                .map_err(|e| format!("Invalid expiration: {}", e))?;

            let presigned = self.client
                .get_object()
                .bucket(bucket)
                .key(key)
                .presigned(presigning_config)
                .await
                .map_err(|e| format!("S3 presign failed: {}", e))?;

            Ok(presigned.uri().to_string())
        } else {
            Err("Unsupported storage reference for S3 presigning".to_string())
        }
    }

    async fn metadata(&self, storage_ref: &ArtifactStorageRef) -> Result<ArtifactMetadata, String> {
        if let ArtifactStorageRef::ObjectStore { bucket, key } = storage_ref {
            let output = self.client
                .head_object()
                .bucket(bucket)
                .key(key)
                .send()
                .await
                .map_err(|e| format!("S3 head_object failed: {}", e))?;

            Ok(ArtifactMetadata {
                size_bytes: output.content_length.unwrap_or(0) as u64,
                content_type: output.content_type.unwrap_or_else(|| "application/octet-stream".to_string()),
                etag: output.e_tag,
                last_modified: output.last_modified.map(|lm| lm.secs() as u64),
            })
        } else {
            Err("Unsupported storage reference for S3 metadata".to_string())
        }
    }
}
