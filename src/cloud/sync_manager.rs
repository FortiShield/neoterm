use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};
use reqwest::Client;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncableData {
    pub id: String,
    pub data_type: DataType,
    pub content: serde_json::Value,
    pub last_modified: DateTime<Utc>,
    pub version: u64,
    pub checksum: String,
    pub device_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataType {
    Settings,
    Themes,
    Workflows,
    CommandHistory,
    Plugins,
    Sessions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConflict {
    pub id: String,
    pub local_version: SyncableData,
    pub remote_version: SyncableData,
    pub conflict_type: ConflictType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConflictType {
    ModifiedBoth,
    DeletedLocal,
    DeletedRemote,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConflictResolution {
    UseLocal,
    UseRemote,
    Merge(serde_json::Value),
}

pub struct CloudSyncManager {
    client: Client,
    base_url: String,
    api_key: String,
    device_id: String,
    local_data: RwLock<HashMap<String, SyncableData>>,
    sync_queue: RwLock<Vec<String>>, // IDs to sync
    last_sync: RwLock<Option<DateTime<Utc>>>,
}

impl CloudSyncManager {
    pub fn new(base_url: String, api_key: String) -> Self {
        let device_id = Uuid::new_v4().to_string();
        
        Self {
            client: Client::new(),
            base_url,
            api_key,
            device_id,
            local_data: RwLock::new(HashMap::new()),
            sync_queue: RwLock::new(Vec::new()),
            last_sync: RwLock::new(None),
        }
    }

    pub async fn sync_all(&self) -> Result<Vec<SyncConflict>, Box<dyn std::error::Error>> {
        let mut conflicts = Vec::new();
        
        // Get remote changes since last sync
        let last_sync_time = *self.last_sync.read().await;
        let remote_changes = self.fetch_remote_changes(last_sync_time).await?;
        
        // Process remote changes and detect conflicts
        for remote_data in remote_changes {
            let local_data = self.local_data.read().await;
            
            if let Some(local_version) = local_data.get(&remote_data.id) {
                if local_version.version != remote_data.version && 
                   local_version.last_modified > remote_data.last_modified {
                    // Conflict detected
                    conflicts.push(SyncConflict {
                        id: remote_data.id.clone(),
                        local_version: local_version.clone(),
                        remote_version: remote_data,
                        conflict_type: ConflictType::ModifiedBoth,
                    });
                } else {
                    // Remote is newer, update local
                    drop(local_data);
                    self.update_local_data(remote_data).await?;
                }
            } else {
                // New remote data
                self.update_local_data(remote_data).await?;
            }
        }
        
        // Upload local changes
        self.upload_pending_changes().await?;
        
        // Update last sync time
        *self.last_sync.write().await = Some(Utc::now());
        
        Ok(conflicts)
    }

    pub async fn add_to_sync_queue(&self, data: SyncableData) -> Result<(), Box<dyn std::error::Error>> {
        let mut local_data = self.local_data.write().await;
        let mut sync_queue = self.sync_queue.write().await;
        
        local_data.insert(data.id.clone(), data.clone());
        
        if !sync_queue.contains(&data.id) {
            sync_queue.push(data.id);
        }
        
        Ok(())
    }

    pub async fn resolve_conflict(&self, conflict: SyncConflict, resolution: ConflictResolution) -> Result<(), Box<dyn std::error::Error>> {
        let resolved_data = match resolution {
            ConflictResolution::UseLocal => conflict.local_version,
            ConflictResolution::UseRemote => conflict.remote_version,
            ConflictResolution::Merge(merged_content) => {
                let mut merged_data = conflict.local_version;
                merged_data.content = merged_content;
                merged_data.last_modified = Utc::now();
                merged_data.version += 1;
                merged_data.checksum = self.calculate_checksum(&merged_data.content);
                merged_data
            }
        };
        
        // Update local data
        self.update_local_data(resolved_data.clone()).await?;
        
        // Upload resolution
        self.upload_data(&resolved_data).await?;
        
        Ok(())
    }

    async fn fetch_remote_changes(&self, since: Option<DateTime<Utc>>) -> Result<Vec<SyncableData>, Box<dyn std::error::Error>> {
        let mut url = format!("{}/api/sync/changes", self.base_url);
        
        if let Some(since_time) = since {
            url.push_str(&format!("?since={}", since_time.to_rfc3339()));
        }
        
        let response = self.client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("X-Device-ID", &self.device_id)
            .send()
            .await?;
        
        if response.status().is_success() {
            let changes: Vec<SyncableData> = response.json().await?;
            Ok(changes)
        } else {
            Err(format!("Failed to fetch remote changes: {}", response.status()).into())
        }
    }

    async fn upload_pending_changes(&self) -> Result<(), Box<dyn std::error::Error>> {
        let sync_queue = self.sync_queue.read().await;
        let local_data = self.local_data.read().await;
        
        for id in sync_queue.iter() {
            if let Some(data) = local_data.get(id) {
                self.upload_data(data).await?;
            }
        }
        
        drop(sync_queue);
        drop(local_data);
        
        // Clear sync queue
        self.sync_queue.write().await.clear();
        
        Ok(())
    }

    async fn upload_data(&self, data: &SyncableData) -> Result<(), Box<dyn std::error::Error>> {
        let url = format!("{}/api/sync/data", self.base_url);
        
        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("X-Device-ID", &self.device_id)
            .json(data)
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(format!("Failed to upload data: {}", response.status()).into());
        }
        
        Ok(())
    }

    async fn update_local_data(&self, data: SyncableData) -> Result<(), Box<dyn std::error::Error>> {
        let mut local_data = self.local_data.write().await;
        local_data.insert(data.id.clone(), data);
        Ok(())
    }

    fn calculate_checksum(&self, content: &serde_json::Value) -> String {
        use sha2::{Sha256, Digest};
        let content_str = serde_json::to_string(content).unwrap_or_default();
        let mut hasher = Sha256::new();
        hasher.update(content_str.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    pub async fn get_sync_status(&self) -> SyncStatus {
        let sync_queue = self.sync_queue.read().await;
        let last_sync = *self.last_sync.read().await;
        
        SyncStatus {
            pending_uploads: sync_queue.len(),
            last_sync_time: last_sync,
            is_online: self.check_connectivity().await,
        }
    }

    async fn check_connectivity(&self) -> bool {
        let url = format!("{}/api/health", self.base_url);
        
        match self.client
            .get(&url)
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await
        {
            Ok(response) => response.status().is_success(),
            Err(_) => false,
        }
    }

    pub async fn force_download(&self, data_type: DataType) -> Result<Vec<SyncableData>, Box<dyn std::error::Error>> {
        let url = format!("{}/api/sync/data?type={:?}", self.base_url, data_type);
        
        let response = self.client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("X-Device-ID", &self.device_id)
            .send()
            .await?;
        
        if response.status().is_success() {
            let data: Vec<SyncableData> = response.json().await?;
            
            // Update local data
            for item in &data {
                self.update_local_data(item.clone()).await?;
            }
            
            Ok(data)
        } else {
            Err(format!("Failed to download data: {}", response.status()).into())
        }
    }

    pub async fn backup_all_data(&self) -> Result<String, Box<dyn std::error::Error>> {
        let local_data = self.local_data.read().await;
        let backup_data: Vec<&SyncableData> = local_data.values().collect();
        
        let backup_json = serde_json::to_string_pretty(&backup_data)?;
        let backup_id = Uuid::new_v4().to_string();
        
        // Save backup locally
        let backup_path = format!("backups/backup_{}.json", backup_id);
        tokio::fs::create_dir_all("backups").await?;
        tokio::fs::write(&backup_path, &backup_json).await?;
        
        // Upload backup to cloud
        let url = format!("{}/api/backups", self.base_url);
        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("X-Device-ID", &self.device_id)
            .json(&serde_json::json!({
                "id": backup_id,
                "data": backup_json,
                "timestamp": Utc::now()
            }))
            .send()
            .await?;
        
        if response.status().is_success() {
            Ok(backup_id)
        } else {
            Err(format!("Failed to upload backup: {}", response.status()).into())
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStatus {
    pub pending_uploads: usize,
    pub last_sync_time: Option<DateTime<Utc>>,
    pub is_online: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sync_manager_creation() {
        let manager = CloudSyncManager::new(
            "https://api.example.com".to_string(),
            "test-key".to_string()
        );
        
        let status = manager.get_sync_status().await;
        assert_eq!(status.pending_uploads, 0);
        assert!(status.last_sync_time.is_none());
    }

    #[tokio::test]
    async fn test_add_to_sync_queue() {
        let manager = CloudSyncManager::new(
            "https://api.example.com".to_string(),
            "test-key".to_string()
        );
        
        let data = SyncableData {
            id: "test-1".to_string(),
            data_type: DataType::Settings,
            content: serde_json::json!({"theme": "dark"}),
            last_modified: Utc::now(),
            version: 1,
            checksum: "abc123".to_string(),
            device_id: "device-1".to_string(),
        };
        
        manager.add_to_sync_queue(data).await.unwrap();
        
        let status = manager.get_sync_status().await;
        assert_eq!(status.pending_uploads, 1);
    }
}
