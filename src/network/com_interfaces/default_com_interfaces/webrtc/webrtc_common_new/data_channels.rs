use std::{
    cell::RefCell,
    collections::HashMap,
    future::Future,
    pin::Pin,
    rc::Rc,
    sync::{Arc, Mutex},
};

use crate::network::com_interfaces::com_interface_socket::ComInterfaceSocketUUID;
pub type OpenChannelCallback<T> = Arc<
    dyn Fn(
            Arc<Mutex<DataChannel<T>>>,
        ) -> Pin<Box<dyn Future<Output = ()> + Send>>
        + Send
        + Sync,
>;
pub struct DataChannel<T> {
    pub label: String,
    pub data_channel: T,
    pub on_message: Option<Box<dyn Fn(Vec<u8>)>>,
    pub open_channel: Option<OpenChannelCallback<T>>,
    pub on_close: Option<Box<dyn Fn()>>,
    pub socket_uuid: RefCell<Option<ComInterfaceSocketUUID>>,
}
impl<T> DataChannel<T> {
    pub fn new(label: String, data_channel: T) -> Self {
        DataChannel {
            label,
            data_channel,
            on_message: None,
            open_channel: None,
            on_close: None,
            socket_uuid: RefCell::new(None),
        }
    }
    pub fn label(&self) -> String {
        self.label.clone()
    }
    pub fn set_socket_uuid(&self, socket_uuid: ComInterfaceSocketUUID) {
        self.socket_uuid.replace(Some(socket_uuid));
    }
    pub fn get_socket_uuid(&self) -> Option<ComInterfaceSocketUUID> {
        self.socket_uuid.borrow().clone()
    }
}

pub struct DataChannels<T> {
    pub data_channels: HashMap<String, Arc<Mutex<DataChannel<T>>>>,
    pub on_add: Option<
        Box<
            dyn Fn(
                Arc<Mutex<DataChannel<T>>>,
            ) -> Pin<Box<dyn Future<Output = ()> + 'static>>,
        >,
    >,
}
impl<T> Default for DataChannels<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> DataChannels<T> {
    pub fn new() -> Self {
        DataChannels {
            data_channels: HashMap::new(),
            on_add: None,
        }
    }
    pub fn reset(&mut self) {
        self.data_channels.clear();
        self.on_add = None;
    }
    pub fn get_data_channel(
        &self,
        label: &str,
    ) -> Option<Arc<Mutex<DataChannel<T>>>> {
        self.data_channels.get(label).cloned()
    }
    pub fn add_data_channel(
        &mut self,
        data_channel: Arc<Mutex<DataChannel<T>>>,
    ) {
        let label = data_channel.lock().unwrap().label.clone();
        self.data_channels.insert(label, data_channel);
    }
    pub async fn create_data_channel(&mut self, label: String, channel: T) {
        let data_channel =
            Arc::new(Mutex::new(DataChannel::new(label.clone(), channel)));
        self.data_channels
            .insert(label.clone(), data_channel.clone());
        if let Some(fut) = self.on_add.take() {
            fut(data_channel).await;
        }
    }
}
