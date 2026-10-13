//! Extension UI requests, answer validation and wire replies.
use super::Extra;
use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UiRequest {
    pub id: String,
    #[serde(flatten)]
    pub method: UiMethod,
    #[serde(flatten)]
    pub extra: Extra,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "method")]
pub enum UiMethod {
    #[serde(rename = "select")]
    Select {
        title: String,
        options: Vec<String>,
        timeout: Option<u64>,
    },
    #[serde(rename = "confirm")]
    Confirm {
        title: String,
        message: String,
        timeout: Option<u64>,
    },
    #[serde(rename = "input")]
    Input {
        title: String,
        placeholder: Option<String>,
        timeout: Option<u64>,
    },
    #[serde(rename = "editor")]
    Editor {
        title: String,
        prefill: Option<String>,
    },
    #[serde(rename = "notify", rename_all = "camelCase")]
    Notify {
        message: String,
        notify_type: Option<String>,
    },
    #[serde(rename = "setStatus", rename_all = "camelCase")]
    SetStatus {
        status_key: String,
        status_text: Option<String>,
    },
    #[serde(rename = "setWidget", rename_all = "camelCase")]
    SetWidget {
        widget_key: String,
        widget_lines: Option<Vec<String>>,
        widget_placement: Option<String>,
    },
    #[serde(rename = "setTitle")]
    SetTitle { title: String },
    #[serde(rename = "set_editor_text")]
    SetEditorText { text: String },
}
impl UiMethod {
    pub fn is_dialog(&self) -> bool {
        matches!(
            self,
            Self::Select { .. } | Self::Confirm { .. } | Self::Input { .. } | Self::Editor { .. }
        )
    }
}
#[derive(Debug, Clone, PartialEq)]
pub enum UiAnswer {
    Value(String),
    Confirmed(bool),
    Cancel,
}
impl UiAnswer {
    fn validate(&self, request: &UiRequest) -> Result<()> {
        let valid = match (&request.method, self) {
            (m, Self::Cancel) => m.is_dialog(),
            (UiMethod::Confirm { .. }, Self::Confirmed(_)) => true,
            (UiMethod::Select { options, .. }, Self::Value(v)) => options.contains(v),
            (UiMethod::Input { .. } | UiMethod::Editor { .. }, Self::Value(_)) => true,
            _ => false,
        };
        if !valid {
            return Err(Error::Protocol("UI response does not match dialog".into()));
        }
        Ok(())
    }
    /// Borrow the text and ID so bounded output does not need an owned JSON copy.
    pub(crate) fn wire_record<'a>(&'a self, request: &'a UiRequest) -> Result<impl Serialize + 'a> {
        self.validate(request)?;
        #[derive(Serialize)]
        struct Reply<'a> {
            r#type: &'static str,
            id: &'a str,
            #[serde(skip_serializing_if = "Option::is_none")]
            value: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            confirmed: Option<bool>,
            #[serde(skip_serializing_if = "Option::is_none")]
            cancelled: Option<bool>,
        }
        Ok(Reply {
            r#type: "extension_ui_response",
            id: &request.id,
            value: match self {
                Self::Value(text) => Some(text),
                _ => None,
            },
            confirmed: match self {
                Self::Confirmed(value) => Some(*value),
                _ => None,
            },
            cancelled: matches!(self, Self::Cancel).then_some(true),
        })
    }
    pub fn record(&self, request: &UiRequest) -> Result<Value> {
        self.validate(request)?;
        let mut v = serde_json::json!({"type":"extension_ui_response","id":request.id});
        match self {
            Self::Value(s) => v["value"] = s.clone().into(),
            Self::Confirmed(b) => v["confirmed"] = (*b).into(),
            Self::Cancel => v["cancelled"] = true.into(),
        }
        Ok(v)
    }
}
