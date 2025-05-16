use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, Hash)]
pub struct RTCIceServer {
    pub urls: Vec<String>,
    pub username: Option<String>,
    pub credential: Option<String>,
}
impl RTCIceServer {
    pub fn new(urls: Vec<String>) -> Self {
        Self {
            urls,
            username: None,
            credential: None,
        }
    }
}
impl RTCIceServer {
    pub fn with_username(mut self, username: String) -> Self {
        self.username = Some(username);
        self
    }
    pub fn with_credential(mut self, credential: String) -> Self {
        self.credential = Some(credential);
        self
    }
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RTCIceCandidateInitDX {
    pub candidate: String,
    pub sdp_mid: Option<String>,
    #[serde(rename = "sdpMLineIndex")]
    pub sdp_mline_index: Option<u16>,
    pub username_fragment: Option<String>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub enum RTCSdpTypeDX {
    #[default]
    Unspecified,
    #[serde(rename = "answer")]
    Answer,
    #[serde(rename = "offer")]
    Offer,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RTCSessionDescriptionDX {
    #[serde(rename = "type")]
    pub sdp_type: RTCSdpTypeDX,
    pub sdp: String,
}
