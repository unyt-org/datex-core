use std::collections::VecDeque;

use log::error;

use crate::datex_values::Endpoint;

use super::{
    structures::{RTCIceCandidateInitDX, RTCIceServer},
    utils::serialize,
};

pub struct WebRTCCommon {
    pub endpoint: Endpoint,
    pub ice_servers: Vec<RTCIceServer>,
    pub candidates: VecDeque<Vec<u8>>,
    pub is_remote_description_set: bool,
    pub on_ice_candidate: Option<Box<dyn Fn(Vec<u8>)>>,
    pub on_connect: Option<Box<dyn Fn()>>,
}

impl WebRTCCommon {
    pub fn reset(&mut self) {
        self.is_remote_description_set = false;
        self.candidates.clear();
        self.on_ice_candidate = None;
    }
    pub fn new(endpoint: impl Into<Endpoint>) -> Self {
        WebRTCCommon {
            endpoint: endpoint.into(),
            candidates: VecDeque::new(),
            is_remote_description_set: false,
            on_ice_candidate: None,
            on_connect: None,
            ice_servers: vec![RTCIceServer {
                urls: vec!["stun:stun.l.google.com:19302".to_string()],
                username: None,
                credential: None,
            }],
        }
    }
    pub fn on_ice_candidate(&self, candidate: RTCIceCandidateInitDX) {
        if let Some(ref on_ice_candidate) = self.on_ice_candidate {
            if let Ok(candidate) = serialize(&candidate) {
                on_ice_candidate(candidate);
            } else {
                error!("Failed to serialize candidate");
            }
        } else {
            error!("No on_ice_candidate callback set");
        }
    }
}
