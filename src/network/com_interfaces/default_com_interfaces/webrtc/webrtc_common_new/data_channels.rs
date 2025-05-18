use std::{
    cell::RefCell,
    collections::HashMap,
    future::Future,
    pin::Pin,
    rc::Rc,
    sync::{Arc, Mutex},
};

use crate::network::com_interfaces::com_interface_socket::ComInterfaceSocketUUID;

pub struct DataChannel<'a, T> {
    pub label: String,
    pub data_channel: &'a T,
    pub on_message: RefCell<Option<Box<dyn Fn(Vec<u8>)>>>,
    pub open_channel: RefCell<Option<Box<dyn Fn() + Send + Sync>>>,
    pub on_close: Option<Box<dyn Fn()>>,
    pub socket_uuid: RefCell<Option<ComInterfaceSocketUUID>>,
}
impl<'a, T> DataChannel<'a, T> {
    pub fn new(label: String, data_channel: &'a T) -> Self {
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

pub struct DataChannels<'a, T> {
    pub data_channels: HashMap<String, Rc<RefCell<DataChannel<'a, T>>>>,
    pub on_add: Option<
        Box<
            dyn Fn(
                Rc<RefCell<DataChannel<T>>>,
            ) -> Pin<Box<dyn Future<Output = ()> + 'static>>,
        >,
    >,
}
impl<'a, T> Default for DataChannels<'a, T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, T> DataChannels<'a, T> {
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
        &'a self,
        label: &str,
    ) -> Option<Rc<RefCell<DataChannel<'a, T>>>> {
        self.data_channels.get(label).cloned()
    }
    pub fn add_data_channel(
        &'a mut self,
        data_channel: Rc<RefCell<DataChannel<'a, T>>>,
    ) {
        let label = data_channel.borrow().label.clone();
        self.data_channels.insert(label, data_channel);
    }
    pub async fn create_data_channel(&mut self, label: String, channel: &'a T) {
        let data_channel =
            Rc::new(RefCell::new(DataChannel::new(label.clone(), channel)));
        self.data_channels
            .insert(label.clone(), data_channel.clone());
        if let Some(fut) = self.on_add.take() {
            fut(data_channel).await;
        }
    }
}
