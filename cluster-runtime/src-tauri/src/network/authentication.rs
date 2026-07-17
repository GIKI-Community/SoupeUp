//! Authentication module for secure peer authentication
//! 
//! Implements public key exchange and challenge-response authentication

use sha2::Sha256;
use rand::Rng;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Authentication manager for handling peer authentication
pub struct Authenticator {
    /// Known public keys by node ID
    known_keys: Arc<RwLock<HashMap<String, String>>>,
    /// Pending challenges by node ID
    challenges: Arc<RwLock<HashMap<String, (String, i64)>>>,
}

impl Authenticator {
    pub fn new() -> Self {
        Self {
            known_keys: Arc::new(RwLock::new(HashMap::new())),
            challenges: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a known node's public key
    pub async fn register_known_node(&self, node_id: String, public_key: String) {
        let mut keys = self.known_keys.write().await;
        keys.insert(node_id, public_key);
    }

    /// Generate a challenge for a node
    pub async fn generate_challenge(&self, node_id: &str) -> String {
        let challenge: String = rand::thread_rng()
            .sample_iter(&rand::distributions::Alphanumeric)
            .take(32)
            .map(char::from)
            .collect();
        
        let timestamp = chrono::Utc::now().timestamp();
        let mut challenges = self.challenges.write().await;
        challenges.insert(node_id.to_string(), (challenge.clone(), timestamp));
        
        challenge
    }

    /// Verify a challenge response
    pub async fn verify_challenge(
        &self,
        node_id: &str,
        signature: &str,
        challenge: &str,
    ) -> bool {
        // In a real implementation, this would verify the signature
        // using the node's public key and the challenge
        
        let mut challenges = self.challenges.write().await;
        if let Some((stored_challenge, timestamp)) = challenges.remove(node_id) {
            // Check if challenge is still valid (within 60 seconds)
            let now = chrono::Utc::now().timestamp();
            if now - timestamp > 60 {
                return false;
            }
            
            // Verify signature (simplified - in production use proper crypto)
            // For now, we just check that the signature is not empty
            !signature.is_empty() && stored_challenge == challenge
        } else {
            false
        }
    }

    /// Generate a signature for a challenge
    pub fn sign_challenge(challenge: &str, private_key: &str) -> String {
        // In production, use proper cryptographic signing
        // For now, use a simple HMAC-SHA256
        use hmac::{Hmac, Mac};
        type HmacSha256 = Hmac<Sha256>;
        
        let mut mac = HmacSha256::new_from_slice(private_key.as_bytes())
            .expect("Invalid key length");
        mac.update(challenge.as_bytes());
        let result = mac.finalize();
        hex::encode(result.into_bytes())
    }

    /// Generate a key pair (for demonstration)
    pub fn generate_key_pair() -> (String, String) {
        // In production, use proper key generation
        // For now, generate random strings
        let private: String = rand::thread_rng()
            .sample_iter(&rand::distributions::Alphanumeric)
            .take(64)
            .map(char::from)
            .collect();
        
        let public: String = rand::thread_rng()
            .sample_iter(&rand::distributions::Alphanumeric)
            .take(64)
            .map(char::from)
            .collect();
        
        (public, private)
    }

    /// Validate a peer's identity
    pub async fn validate_peer(&self, node_id: &str, public_key: &str) -> bool {
        let keys = self.known_keys.read().await;
        if let Some(stored_key) = keys.get(node_id) {
            stored_key == public_key
        } else {
            // If unknown, accept for now (in production, would require manual trust)
            true
        }
    }

    /// Clear expired challenges
    pub async fn clear_expired(&self) {
        let now = chrono::Utc::now().timestamp();
        let mut challenges = self.challenges.write().await;
        challenges.retain(|_, (_, timestamp)| now - *timestamp <= 60);
    }
}

impl Default for Authenticator {
    fn default() -> Self {
        Self::new()
    }
}

/// Session management for authenticated connections
#[derive(Debug, Clone)]
pub struct Session {
    pub node_id: String,
    pub session_id: String,
    pub established_at: i64,
    pub last_activity: i64,
}

impl Session {
    pub fn new(node_id: String) -> Self {
        let session_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().timestamp();
        Self {
            node_id,
            session_id,
            established_at: now,
            last_activity: now,
        }
    }

    pub fn update_activity(&mut self) {
        self.last_activity = chrono::Utc::now().timestamp();
    }

    pub fn is_valid(&self) -> bool {
        let now = chrono::Utc::now().timestamp();
        // Session expires after 24 hours of inactivity
        now - self.last_activity < 86400
    }
}

/// Session manager
pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<String, Session>>>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn create_session(&self, node_id: String) -> Session {
        let session = Session::new(node_id.clone());
        let mut sessions = self.sessions.write().await;
        sessions.insert(node_id, session.clone());
        session
    }

    pub async fn get_session(&self, node_id: &str) -> Option<Session> {
        let sessions = self.sessions.read().await;
        sessions.get(node_id).cloned()
    }

    pub async fn update_activity(&self, node_id: &str) {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(node_id) {
            session.update_activity();
        }
    }

    pub async fn remove_session(&self, node_id: &str) {
        let mut sessions = self.sessions.write().await;
        sessions.remove(node_id);
    }

    pub async fn clear_expired(&self) {
        let mut sessions = self.sessions.write().await;
        sessions.retain(|_, session| session.is_valid());
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}
