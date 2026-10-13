//! Typed convenience methods over the core command submission path.
use super::PiClient;
use crate::{Result, protocol::*};
use serde_json::Value;

impl PiClient {
    pub async fn prompt(&self, message: impl Into<String>) -> Result<PromptDisposition> {
        Ok(self
            .typed::<Disposition<PromptDisposition>>(Command::Prompt {
                message: message.into(),
                images: None,
                streaming_behavior: None,
            })
            .await?
            .disposition)
    }
    pub fn prompt_with(
        &self,
        message: impl Into<String>,
        images: Option<Vec<ImageContent>>,
        streaming_behavior: Option<StreamingBehavior>,
    ) -> impl std::future::Future<Output = Result<PromptDisposition>> + Send {
        let images = crate::owned::Owned::new(images);
        let message = message.into();
        let response = self.typed::<Disposition<PromptDisposition>>(Command::Prompt {
            message,
            images: images.take(),
            streaming_behavior,
        });
        async move { Ok(response.await?.disposition) }
    }
    pub fn steer(
        &self,
        message: impl Into<String>,
        images: Option<Vec<ImageContent>>,
    ) -> impl std::future::Future<Output = Result<QueuedInputDisposition>> + Send {
        let images = crate::owned::Owned::new(images);
        let message = message.into();
        let response = self.typed::<Disposition<QueuedInputDisposition>>(Command::Steer {
            message,
            images: images.take(),
        });
        async move { Ok(response.await?.disposition) }
    }
    pub fn follow_up(
        &self,
        message: impl Into<String>,
        images: Option<Vec<ImageContent>>,
    ) -> impl std::future::Future<Output = Result<QueuedInputDisposition>> + Send {
        let images = crate::owned::Owned::new(images);
        let message = message.into();
        let response = self.typed::<Disposition<QueuedInputDisposition>>(Command::FollowUp {
            message,
            images: images.take(),
        });
        async move { Ok(response.await?.disposition) }
    }
    pub async fn abort(&self) -> Result<()> {
        self.command(Command::Abort).await.map(|_| ())
    }
    pub async fn clear_queue(&self) -> Result<Queue> {
        self.typed(Command::ClearQueue).await
    }
    pub async fn get_state(&self) -> Result<SessionState> {
        self.typed(Command::GetState).await
    }
    pub async fn new_session(&self, parent_session: Option<String>) -> Result<SessionChange> {
        self.typed(Command::NewSession { parent_session }).await
    }
    pub async fn switch_session(&self, session_path: impl Into<String>) -> Result<SessionChange> {
        self.typed(Command::SwitchSession {
            session_path: session_path.into(),
        })
        .await
    }
    pub async fn fork(&self, entry_id: impl Into<String>) -> Result<ForkResult> {
        self.typed(Command::Fork {
            entry_id: entry_id.into(),
        })
        .await
    }
    pub async fn clone_session(&self) -> Result<SessionChange> {
        self.typed(Command::Clone).await
    }
    pub async fn get_messages(&self) -> Result<Messages> {
        self.typed(Command::GetMessages).await
    }
    pub async fn get_entries(&self, since: Option<String>) -> Result<Entries> {
        self.typed(Command::GetEntries { since }).await
    }
    pub async fn get_tree(&self) -> Result<Tree> {
        self.typed(Command::GetTree).await
    }
    pub async fn get_commands(&self) -> Result<Commands> {
        self.typed(Command::GetCommands).await
    }
    pub async fn get_session_stats(&self) -> Result<SessionStats> {
        self.typed(Command::GetSessionStats).await
    }
    pub async fn bash(
        &self,
        command: impl Into<String>,
        exclude_from_context: Option<bool>,
    ) -> Result<BashResult> {
        self.typed(Command::Bash {
            command: command.into(),
            exclude_from_context,
        })
        .await
    }
    pub async fn compact(&self, custom_instructions: Option<String>) -> Result<CompactionResult> {
        self.typed(Command::Compact {
            custom_instructions,
        })
        .await
    }
    pub async fn set_model(
        &self,
        provider: impl Into<String>,
        model_id: impl Into<String>,
    ) -> Result<Value> {
        self.typed(Command::SetModel {
            provider: provider.into(),
            model_id: model_id.into(),
        })
        .await
    }
    pub async fn cycle_model(&self) -> Result<Option<ModelCycle>> {
        self.typed(Command::CycleModel).await
    }
    pub async fn get_available_models(&self) -> Result<Models> {
        self.typed(Command::GetAvailableModels).await
    }
    pub async fn set_thinking_level(&self, level: ThinkingLevel) -> Result<()> {
        self.command(Command::SetThinkingLevel { level })
            .await
            .map(|_| ())
    }
    pub async fn cycle_thinking_level(&self) -> Result<Option<Level>> {
        self.typed(Command::CycleThinkingLevel).await
    }
    pub async fn get_available_thinking_levels(&self) -> Result<Levels> {
        self.typed(Command::GetAvailableThinkingLevels).await
    }
    pub async fn set_steering_mode(&self, mode: QueueMode) -> Result<()> {
        self.command(Command::SetSteeringMode { mode })
            .await
            .map(|_| ())
    }
    pub async fn set_follow_up_mode(&self, mode: QueueMode) -> Result<()> {
        self.command(Command::SetFollowUpMode { mode })
            .await
            .map(|_| ())
    }
    pub async fn set_auto_compaction(&self, enabled: bool) -> Result<()> {
        self.command(Command::SetAutoCompaction { enabled })
            .await
            .map(|_| ())
    }
    pub async fn set_auto_retry(&self, enabled: bool) -> Result<()> {
        self.command(Command::SetAutoRetry { enabled })
            .await
            .map(|_| ())
    }
    pub async fn abort_retry(&self) -> Result<()> {
        self.command(Command::AbortRetry).await.map(|_| ())
    }
    pub async fn abort_bash(&self) -> Result<()> {
        self.command(Command::AbortBash).await.map(|_| ())
    }
    pub async fn export_html(&self, output_path: Option<String>) -> Result<ExportPath> {
        self.typed(Command::ExportHtml { output_path }).await
    }
    pub async fn get_fork_messages(&self) -> Result<ForkMessages> {
        self.typed(Command::GetForkMessages).await
    }
    pub async fn get_last_assistant_text(&self) -> Result<LastAssistantText> {
        self.typed(Command::GetLastAssistantText).await
    }
    pub async fn set_session_name(&self, name: impl Into<String>) -> Result<()> {
        self.command(Command::SetSessionName { name: name.into() })
            .await
            .map(|_| ())
    }
}
