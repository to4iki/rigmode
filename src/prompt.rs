/// What an agent hook handed us for one prompt. Agent-agnostic; adapters
/// decode their own payload into this.
#[derive(Debug)]
pub struct PromptMeta {
    pub prompt: String,
    pub session_id: Option<String>,
    pub transcript_path: Option<String>,
    pub cwd: Option<String>,
}
