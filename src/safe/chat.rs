//! Chat-template wrapper: safe rendering of role/content turns via llama.cpp
//! (`llama_model_chat_template` + `llama_chat_apply_template` through `llama-cpp-2`).
//!
//! All orchestration is safe Rust; FFI stays inside the backend crate.

use crate::error::{Error, Result};
use crate::safe::Model;
use llama_cpp_2::model::{LlamaChatMessage, LlamaChatTemplate};
use llama_cpp_2::ChatTemplateError;

/// One conversation turn: role (`system` / `user` / `assistant` / `tool`) + content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatMessage {
    role: String,
    content: String,
}

impl ChatMessage {
    /// Create a message. Null bytes in role or content are rejected early (unrepresentable in FFI).
    pub fn new(role: impl Into<String>, content: impl Into<String>) -> Result<Self> {
        let role = role.into();
        let content = content.into();
        if role.contains('\0') || content.contains('\0') {
            return Err(Error::ChatTemplate(
                "role or content contains a null byte".into(),
            ));
        }
        Ok(Self { role, content })
    }

    /// Message role.
    #[must_use]
    pub fn role(&self) -> &str {
        &self.role
    }

    /// Message content.
    #[must_use]
    pub fn content(&self) -> &str {
        &self.content
    }

    fn to_inner(&self) -> std::result::Result<LlamaChatMessage, Error> {
        LlamaChatMessage::new(self.role.clone(), self.content.clone())
            .map_err(|e| Error::ChatTemplate(e.to_string()))
    }
}

fn template_error(e: impl std::fmt::Display) -> Error {
    Error::ChatTemplate(e.to_string())
}

impl Model {
    /// Chat template baked into the GGUF metadata, `Ok(None)` when the model ships none.
    pub fn chat_template(&self) -> Result<Option<String>> {
        match self.inner.chat_template(None) {
            Ok(t) => t.to_string().map(Some).map_err(template_error),
            Err(ChatTemplateError::MissingTemplate) => Ok(None),
            Err(e) => Err(template_error(e)),
        }
    }

    /// Render messages with the model's built-in template; `add_ass` appends the assistant opening tag.
    ///
    /// # Errors
    /// [`Error::ChatTemplate`] when the model has no template or rendering fails.
    pub fn apply_chat_template(&self, messages: &[ChatMessage], add_ass: bool) -> Result<String> {
        let tmpl = self.inner.chat_template(None).map_err(template_error)?;
        self.render_with(&tmpl, messages, add_ass)
    }

    /// Render messages with an explicit template: a llama.cpp built-in name (`"chatml"`, `"llama3"`)
    /// or a raw Jinja string (e.g. the output of [`Model::chat_template`]).
    pub fn apply_named_chat_template(
        &self,
        template: &str,
        messages: &[ChatMessage],
        add_ass: bool,
    ) -> Result<String> {
        let tmpl = LlamaChatTemplate::new(template).map_err(template_error)?;
        self.render_with(&tmpl, messages, add_ass)
    }

    fn render_with(
        &self,
        tmpl: &LlamaChatTemplate,
        messages: &[ChatMessage],
        add_ass: bool,
    ) -> Result<String> {
        let inner: Vec<LlamaChatMessage> = messages
            .iter()
            .map(ChatMessage::to_inner)
            .collect::<Result<_>>()?;
        self.inner
            .apply_chat_template(tmpl, &inner, add_ass)
            .map_err(template_error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_message_accessors_roundtrip() {
        let m = ChatMessage::new("user", "Hello").expect("valid");
        assert_eq!(m.role(), "user");
        assert_eq!(m.content(), "Hello");
    }

    #[test]
    fn chat_message_rejects_null_in_role() {
        let e = ChatMessage::new("sy\0stem", "x").expect_err("null role must fail");
        assert!(e.to_string().contains("null byte"));
    }

    #[test]
    fn chat_message_rejects_null_in_content() {
        let e = ChatMessage::new("user", "a\0b").expect_err("null content must fail");
        assert!(matches!(e, Error::ChatTemplate(_)));
    }

    #[test]
    fn chat_message_to_inner_ok() {
        let m = ChatMessage::new("assistant", "ok").expect("valid");
        assert!(m.to_inner().is_ok());
    }
}
