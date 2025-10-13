use std::{cell::RefCell, collections::HashMap, pin::Pin, rc::Rc};

use serde::{Deserialize, Serialize};
// use webrtc::{
//     api::media_engine::{MIME_TYPE_OPUS, MIME_TYPE_VP8},
//     rtp_transceiver::{
//         RTCPFeedback,
//         rtp_codec::{RTCRtpCodecCapability, RTCRtpCodecParameters},
//     },
// };

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum MediaKind {
    Audio,
    Video,
}

pub struct MediaTrack<T> {
    pub id: String,
    pub kind: MediaKind,
    pub track: T,

    pub on_mute: RefCell<Option<Box<dyn Fn()>>>,
    pub on_unmute: RefCell<Option<Box<dyn Fn()>>>,
    pub on_ended: RefCell<Option<Box<dyn Fn()>>>,
}

impl<T> MediaTrack<T> {
    pub fn new(id: String, kind: MediaKind, track: T) -> Self {
        MediaTrack {
            id,
            kind,
            track,
            on_mute: RefCell::new(None),
            on_unmute: RefCell::new(None),
            on_ended: RefCell::new(None),
        }
    }

    pub fn id(&self) -> String {
        self.id.clone()
    }

    pub fn kind(&self) -> MediaKind {
        self.kind.clone()
    }

    pub fn set_on_mute(&self, cb: Box<dyn Fn()>) {
        self.on_mute.replace(Some(cb));
    }

    pub fn set_on_unmute(&self, cb: Box<dyn Fn()>) {
        self.on_unmute.replace(Some(cb));
    }

    pub fn set_on_ended(&self, cb: Box<dyn Fn()>) {
        self.on_ended.replace(Some(cb));
    }

    // These would be called internally by the backend implementations
    pub fn trigger_mute(&self) {
        if let Some(cb) = self.on_mute.borrow().as_ref() {
            cb();
        }
    }
    pub fn trigger_unmute(&self) {
        if let Some(cb) = self.on_unmute.borrow().as_ref() {
            cb();
        }
    }
    pub fn trigger_ended(&self) {
        if let Some(cb) = self.on_ended.borrow().as_ref() {
            cb();
        }
    }
}

type OnMediaTrackAddedCallback<T> =
    dyn Fn(
        Rc<RefCell<MediaTrack<T>>>,
    ) -> Pin<Box<dyn Future<Output = ()> + 'static>>;

pub struct MediaTracks<T> {
    pub tracks: HashMap<String, Rc<RefCell<MediaTrack<T>>>>,
    pub on_add: Option<Box<OnMediaTrackAddedCallback<T>>>,
}

impl<T> Default for MediaTracks<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> MediaTracks<T> {
    pub fn new() -> Self {
        MediaTracks {
            tracks: HashMap::new(),
            on_add: None,
        }
    }

    pub fn reset(&mut self) {
        self.tracks.clear();
        self.on_add = None;
    }

    pub fn get_track(&self, id: &str) -> Option<Rc<RefCell<MediaTrack<T>>>> {
        self.tracks.get(id).cloned()
    }

    pub fn add_track(&mut self, track: Rc<RefCell<MediaTrack<T>>>) {
        let id = track.borrow().id.clone();
        self.tracks.insert(id, track);
    }

    pub async fn create_track(
        &mut self,
        id: String,
        kind: MediaKind,
        track: T,
    ) {
        let media_track =
            Rc::new(RefCell::new(MediaTrack::new(id.clone(), kind, track)));
        self.tracks.insert(id.clone(), media_track.clone());

        if let Some(fut) = self.on_add.take() {
            fut(media_track).await;
        }
    }
}

// FIXME #381: Add a subset allowed list of RTCRtpCodecParameters
// #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
// pub struct MediaTrackConfig {
//     pub kind: MediaKind,
//     pub clock_rate: u32,
//     pub channels: u16,
// }
// pub struct MimeType {
//     pub kind: String,
// }

// impl MimeType {
//     pub fn new(kind: &str) -> Self {
//         MimeType {
//             kind: kind.to_string(),
//         }
//     }
// }
pub enum MediaTrackCodec {
    Opus, // Audio
    VP8,  // Video
    H264, // Video
}
impl MediaTrackCodec {
    pub fn to_mime_type(&self) -> String {
        match self {
            MediaTrackCodec::Opus => "audio/opus".to_string(),
            MediaTrackCodec::VP8 => "video/VP8".to_string(),
            MediaTrackCodec::H264 => "video/H264".to_string(),
        }
    }
}
// const FEEDBACK: Vec<RTCPFeedback> = vec![
//     RTCPFeedback {
//         typ: "goog-remb".to_owned(),
//         parameter: "".to_owned(),
//     },
//     RTCPFeedback {
//         typ: "ccm".to_owned(),
//         parameter: "fir".to_owned(),
//     },
//     RTCPFeedback {
//         typ: "nack".to_owned(),
//         parameter: "".to_owned(),
//     },
//     RTCPFeedback {
//         typ: "nack".to_owned(),
//         parameter: "pli".to_owned(),
//     },
// ];
// impl Into<RTCRtpCodecParameters> for MediaTrackCodec {
//     fn into(self) -> RTCRtpCodecParameters {
//         match self {
//             MediaTrackCodec::Opus => RTCRtpCodecParameters {
//                 capability: RTCRtpCodecCapability {
//                     mime_type: MIME_TYPE_OPUS.to_owned(),
//                     clock_rate: 48000,
//                     channels: 2,
//                     sdp_fmtp_line: "minptime=10;useinbandfec=1".to_owned(),
//                     rtcp_feedback: vec![],
//                 },
//                 payload_type: 111,
//                 ..Default::default()
//             },
//             MediaTrackCodec::VP8 => RTCRtpCodecParameters {
//                 capability: RTCRtpCodecCapability {
//                     mime_type: MIME_TYPE_VP8.to_owned(),
//                     clock_rate: 90000,
//                     channels: 0,
//                     sdp_fmtp_line: "".to_owned(),
//                     rtcp_feedback: video_rtcp_feedback.clone(),
//                 },
//                 payload_type: 96,
//                 ..Default::default()
//             },
//             MediaTrackCodec::H264 => RTCRtpCodecParameters::new("video/H264"),
//         }
//     }
// }
