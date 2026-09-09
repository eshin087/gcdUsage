use super::*;
impl Store {
    pub fn prompt_detail(&self, id: &str) -> Result<crate::navigation::PromptDetail, String> {
        if id.len() > 520 {
            return Err("Invalid prompt identifier".into());
        }
        if let Some(key) = id.strip_prefix("browser:") {
            let p: crate::browser_history::BrowserPrompt = self
                .read_one("SELECT data FROM browser_prompts WHERE id=?1", key)?
                .ok_or("Prompt no longer available")?;
            return Ok(crate::navigation::PromptDetail{id:id.into(),provider:p.provider.clone(),project:None,chat_title:p.chat_title,conversation_id:p.conversation_id.clone(),turn_id:p.message_id,preview:crate::presentation::clean_preview(&p.preview),timestamp:p.timestamp,tokens:TokenUsage::default(),models:p.models,request_count:0,original_url:crate::navigation::browser_url(&p.provider,&p.conversation_id),limitation:"Opens the conversation, not an exact message. Sign in to its original account. Browser exports do not include measured token usage.".into()});
        }
        let key = id.strip_prefix("local:").unwrap_or(id);
        let p = self.prompt(key)?.ok_or("Prompt no longer available")?;
        let rows: Vec<RequestUsage> = self.query_json(
            "SELECT data FROM requests WHERE prompt_id=?1 AND provider=?2 AND account_id=?3",
            params![p.id, p.provider.key(), p.account_id],
        )?;
        let mut tokens = TokenUsage::default();
        let mut models = std::collections::BTreeSet::new();
        for r in &rows {
            tokens.add(&r.tokens);
            models.insert(format!(
                "{} · {}",
                crate::presentation::plain(&r.model, 100),
                r.effort.as_deref().unwrap_or("unknown")
            ));
        }
        Ok(crate::navigation::PromptDetail{id:id.into(),provider:p.provider.key().into(),project:p.project,chat_title:p.chat_title,conversation_id:p.session_id,turn_id:p.turn_id,preview:crate::presentation::clean_preview(&p.preview),timestamp:Some(p.timestamp),tokens,models:models.into_iter().take(32).collect(),request_count:rows.len() as u64,original_url:None,limitation:"This coding-tool log has no verified exact-message link. Use the conversation and turn identifiers in your coding tool. Only the captured preview is retained.".into()})
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn remote_identity_collision_cannot_replace_an_existing_prompt() {
        let mut store = Store::open(Path::new(":memory:")).unwrap();
        let p = super::super::tests::prompt();
        store.save_prompt(&p).unwrap();
        let mut forged = p.clone();
        forged.account_id = "another-account".into();
        forged.preview = "overwritten".into();
        assert!(store
            .apply_event(&SyncEvent::Prompt(forged), false)
            .is_err());
        assert_eq!(store.prompt(&p.id).unwrap().unwrap().preview, p.preview);
    }
    #[test]
    fn detail_lookup_is_scoped_and_keeps_missing_link_unknown() {
        let mut store = Store::open(Path::new(":memory:")).unwrap();
        let p = super::super::tests::prompt();
        store.save_prompt(&p).unwrap();
        let d = store.prompt_detail(&format!("local:{}", p.id)).unwrap();
        assert!(d.original_url.is_none());
        assert_eq!(d.conversation_id, p.session_id);
        assert_eq!(d.tokens, TokenUsage::default());
        assert!(store.prompt_detail("local:' OR 1=1 --").is_err());
    }
}
