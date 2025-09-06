use std::{cell::RefCell, collections::HashMap, pin::Pin, rc::Rc};

#[derive(Clone, Debug)]
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

pub struct MediaTracks<T> {
    pub tracks: HashMap<String, Rc<RefCell<MediaTrack<T>>>>,
    pub on_add: Option<
        Box<
            dyn Fn(
                Rc<RefCell<MediaTrack<T>>>,
            ) -> Pin<Box<dyn Future<Output = ()> + 'static>>,
        >,
    >,
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
