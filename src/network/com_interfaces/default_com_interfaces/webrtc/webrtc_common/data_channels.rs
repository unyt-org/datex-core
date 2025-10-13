use std::{
    cell::RefCell, collections::HashMap, future::Future, pin::Pin, rc::Rc,
};

use crate::network::com_interfaces::com_interface_socket::ComInterfaceSocketUUID;

type OnMessageCallback = dyn Fn(Vec<u8>);

pub struct DataChannel<T> {
    pub label: String,
    pub data_channel: T,
    pub on_message: RefCell<Option<Box<OnMessageCallback>>>,
    pub open_channel: RefCell<Option<Box<dyn Fn()>>>,
    pub on_close: Option<Box<dyn Fn()>>,
    pub socket_uuid: RefCell<Option<ComInterfaceSocketUUID>>,
}
impl<T> DataChannel<T> {
    pub fn new(label: String, data_channel: T) -> Self {
        DataChannel {
            label,
            data_channel,
            on_message: RefCell::new(None),
            open_channel: RefCell::new(None),
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

type OnDataChannelAddedCallback<T> =
    dyn Fn(
        Rc<RefCell<DataChannel<T>>>,
    ) -> Pin<Box<dyn Future<Output = ()> + 'static>>;

pub struct DataChannels<T> {
    pub data_channels: HashMap<String, Rc<RefCell<DataChannel<T>>>>,
    pub on_add: Option<Box<OnDataChannelAddedCallback<T>>>,
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
    ) -> Option<Rc<RefCell<DataChannel<T>>>> {
        self.data_channels.get(label).cloned()
    }
    pub fn add_data_channel(
        &mut self,
        data_channel: Rc<RefCell<DataChannel<T>>>,
    ) {
        let label = data_channel.borrow().label.clone();
        self.data_channels.insert(label, data_channel);
    }
    pub async fn create_data_channel(&mut self, label: String, channel: T) {
        let data_channel =
            Rc::new(RefCell::new(DataChannel::new(label.clone(), channel)));
        self.data_channels
            .insert(label.clone(), data_channel.clone());
        if let Some(fut) = self.on_add.take() {
            fut(data_channel).await;
        }
    }
}
