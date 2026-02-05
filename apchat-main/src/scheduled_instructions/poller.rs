// Background poller for scheduled instructions
//
// This module provides a background task that periodically checks for scheduled
// instructions that are due to be injected into the input channel.

use anyhow::Result;
use apchat_mspc::{MspcChannel, MspcMessage};
use std::sync::Arc;
use tokio::sync::Mutex;
use chrono::Utc;
use apchat_tools::memory::{
    connect_pool, init_db, get_memory_db_path,
    get_due_scheduled_instructions, mark_scheduled_instruction_as_processed,
};

/// Background poller for scheduled instructions
/// 
/// This struct holds the state needed for the background poller task.
/// It should be spawned as a tokio task and stopped when the REPL mode exits.
pub struct ScheduledInstructionPoller {
    /// MSPC channel for injecting instructions
    mspc_channel: Arc<MspcChannel>,
    /// Path to the memory database
    db_path: std::path::PathBuf,
}

impl ScheduledInstructionPoller {
    /// Create a new scheduled instruction poller
    /// 
    /// # Arguments
    /// * `db_path` - Path to the memory database
    pub fn new(db_path: std::path::PathBuf) -> Self {
        Self {
            mspc_channel: Arc::new(MspcChannel::new(100)),
            db_path,
        }
    }

    /// Set the MSPC channel for injecting instructions
    pub fn set_channel(&mut self, channel: Arc<MspcChannel>) {
        self.mspc_channel = channel;
    }

    /// Start the background poller task
    /// 
    /// This spawns a tokio task that periodically checks for scheduled instructions
    /// that are due and injects them into the MSPC channel.
    pub fn start(&mut self) -> tokio::task::JoinHandle<()> {
        let pool_path = self.db_path.clone();
        let mspc_channel = self.mspc_channel.clone();
        
        let handle = tokio::spawn(async move {
            // Initialize database
            let pool = match connect_pool(&pool_path).await {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("Failed to connect to memory database: {}", e);
                    return;
                }
            };
            
            if let Err(e) = init_db(&pool).await {
                eprintln!("Failed to initialize memory database: {}", e);
                return;
            }
            
            let pool = Arc::new(Mutex::new(pool));
            
            let poll_interval = std::time::Duration::from_secs(30);
            
            loop {
                tokio::time::sleep(poll_interval).await;
                
                if let Err(e) = poll_and_inject(&pool, &mspc_channel).await {
                    eprintln!("Scheduled instruction poller error: {}", e);
                }
            }
        });
        
        handle
    }
}

impl Default for ScheduledInstructionPoller {
    fn default() -> Self {
        Self::new(get_memory_db_path())
    }
}

/// Poll for due scheduled instructions and inject them into the MSPC channel
async fn poll_and_inject(
    pool: &Arc<Mutex<sqlx::SqlitePool>>,
    mspc_channel: &Arc<MspcChannel>,
) -> Result<()> {
    let now = Utc::now().timestamp();
    
    // Get due instructions with a reasonable limit
    let pool_guard = pool.lock().await;
    let instructions = get_due_scheduled_instructions(&pool_guard, now, Some(100)).await?;
    
    if instructions.is_empty() {
        return Ok(());
    }
    
    for instruction in &instructions {
        // Inject the instruction as a regular user input
        let message = MspcMessage::UserInput(instruction.content.clone(), Some("scheduled".to_string()));
        
        if let Err(e) = mspc_channel.send(message).await {
            eprintln!("Failed to inject scheduled instruction '{}': {}", instruction.id, e);
            continue;
        }
        
        // Mark the instruction as processed
        if let Err(e) = mark_scheduled_instruction_as_processed(&pool_guard, &instruction.id, now).await {
            eprintln!("Failed to mark instruction '{}' as processed: {}", instruction.id, e);
            // Don't return here - continue processing other instructions
        }
    }
    
    Ok(())
}
