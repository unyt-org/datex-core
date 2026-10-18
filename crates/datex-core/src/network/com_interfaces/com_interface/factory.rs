pub use crate::network::com_hub::managers::com_interface_manager::ComInterfaceAsyncFactoryResult;
use crate::{
    channel::mpsc::{UnboundedReceiver, create_unbounded_channel},
    global::dxb_block::DXBBlock,
    network::{
        com_hub::errors::ComInterfaceCreateError,
        com_interfaces::com_interface::{
            ComInterfaceUUID,
            properties::{ComInterfaceProperties, InterfaceDirection},
            socket::ComInterfaceSocketUUID,
        },
    },
    prelude::*,
    std_sync::Mutex,
    traits::convert_value::ConvertValue,
    utils::async_callback::AsyncCallback,
    values::{core_values::endpoint::Endpoint, value::Value},
};
use core::{async_iter::AsyncIterator, fmt::Debug, pin::Pin};
use futures::channel::oneshot::Sender;
use futures_core::future::LocalBoxFuture;
use futures_util::TryStreamExt;
use crate::traits::convert_parts::FromParts;

pub type NewSocketsIterator = Pin<
    Box<dyn AsyncIterator<Item = Result<SocketConfiguration, ()>> + 'static>,
>;

#[derive(Debug, Clone)]

pub struct SocketProperties {
    pub direction: InterfaceDirection,
    pub channel_factor: u32,
    pub direct_endpoint: Option<Endpoint>,
    pub connection_timestamp: u64,
    uuid: ComInterfaceSocketUUID,
}

impl SocketProperties {
    pub fn new(direction: InterfaceDirection, channel_factor: u32) -> Self {
        SocketProperties {
            direction,
            channel_factor,
            direct_endpoint: None,
            connection_timestamp: crate::time::now_ms(),
            uuid: ComInterfaceSocketUUID::new(),
        }
    }
    pub fn new_with_direct_endpoint(
        direction: InterfaceDirection,
        channel_factor: u32,
        endpoint: Endpoint,
    ) -> Self {
        SocketProperties {
            direction,
            channel_factor,
            direct_endpoint: Some(endpoint),
            connection_timestamp: crate::time::now_ms(),
            uuid: ComInterfaceSocketUUID::new(),
        }
    }

    pub fn new_with_maybe_direct_endpoint(
        direction: InterfaceDirection,
        channel_factor: u32,
        maybe_endpoint: Option<Endpoint>,
    ) -> Self {
        SocketProperties {
            direction,
            channel_factor,
            direct_endpoint: maybe_endpoint,
            connection_timestamp: crate::time::now_ms(),
            uuid: ComInterfaceSocketUUID::new(),
        }
    }

    pub fn uuid(&self) -> ComInterfaceSocketUUID {
        self.uuid.clone()
    }
}

pub type SocketDataIterator =
    Pin<Box<dyn AsyncIterator<Item = Result<Vec<u8>, ()>>>>;

pub struct SocketConfiguration {
    pub properties: SocketProperties,

    /// An asynchronous iterator that yields incoming data from the socket as Vec<u8>
    /// It is driven by the com hub to receive data from the socket
    pub iterator: Option<SocketDataIterator>,

    /// A callback that is called by the com hub to send data through the socket
    /// This can be either a synchronous or asynchronous callback depending on the interface implementation
    pub send_callback: Option<SendCallback>,
    /// An optional asynchronous callback that is called by the com hub when the socket is closed
    pub close_async_callback: Option<CloseAsyncCallback>,
}

impl Debug for SocketConfiguration {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SocketConfiguration")
            .field("properties", &self.properties)
            .finish()
    }
}

impl SocketConfiguration {
    /// Creates a SocketDataIterator for a given socket with the provided parameters.
    /// This is the most general constructor for SocketConfiguration, allowing for optional incoming data iterator, send callback, and close callback.
    pub fn new<I, F, Fut>(
        socket_configuration: SocketProperties,
        maybe_iter: Option<I>,
        send_callback: Option<SendCallback>,
        close_async_callback: Option<F>,
    ) -> Self
    where
        I: AsyncIterator<Item = Result<Vec<u8>, ()>> + 'static,
        F: FnOnce() -> Fut + 'static,
        Fut: Future<Output = ()> + 'static,
    {
        SocketConfiguration {
            properties: socket_configuration,
            iterator: maybe_iter
                .map(|iter| Box::pin(iter) as SocketDataIterator),
            send_callback,
            close_async_callback: close_async_callback.map(|cb| {
                Box::new(move || {
                    Box::pin(cb()) as Pin<Box<dyn Future<Output = ()>>>
                }) as CloseAsyncCallback
            }),
        }
    }

    /// Creates a SocketDataIterator for a given socket with the provided parameters.
    /// Expects both an iterator for incoming data and a send callback for outgoing data.
    pub fn new_in_out<I>(
        socket_configuration: SocketProperties,
        iter: I,
        send_callback: SendCallback,
    ) -> Self
    where
        I: AsyncIterator<Item = Result<Vec<u8>, ()>> + 'static,
    {
        SocketConfiguration {
            properties: socket_configuration,
            iterator: Some(Box::pin(iter)),
            send_callback: Some(send_callback),
            close_async_callback: None,
        }
    }

    /// Creates a SocketDataIterator for a given socket with the provided parameters.
    /// Only handles incoming data; no send callback is provided.
    pub fn new_in<I>(
        socket_configuration: SocketProperties,
        maybe_iter: I,
    ) -> Self
    where
        I: AsyncIterator<Item = Result<Vec<u8>, ()>> + 'static,
    {
        SocketConfiguration {
            properties: socket_configuration,
            iterator: Some(Box::pin(maybe_iter)),
            send_callback: None,
            close_async_callback: None,
        }
    }

    /// Creates a SocketDataIterator for a given socket with the provided parameters.
    /// Only handles outgoing data; no incoming data iterator is provided.
    pub fn new_out(
        socket_configuration: SocketProperties,
        send_callback: SendCallback,
    ) -> Self {
        SocketConfiguration {
            properties: socket_configuration,
            iterator: None,
            send_callback: Some(send_callback),
            close_async_callback: None,
        }
    }

    /// Creates a SocketDataIterator with a combined approach for handling both incoming and outgoing data in the same async generator
    pub fn new_combined<I>(
        socket_configuration: SocketProperties,
        generator_initializer: impl FnOnce(
            UnboundedReceiver<(DXBBlock, Sender<Result<(), SendFailure>>)>,
        ) -> I,
    ) -> Self
    where
        I: AsyncIterator<Item = Result<Vec<u8>, ()>> + 'static,
    {
        let (out_sender, out_receiver) = create_unbounded_channel::<(
            DXBBlock,
            Sender<Result<(), SendFailure>>,
        )>();
        let out_sender = Rc::new(Mutex::new(out_sender));

        SocketConfiguration::new_in_out(
            socket_configuration,
            generator_initializer(out_receiver),
            SendCallback::new_async(move |block: DXBBlock| {
                let out_sender = out_sender.clone();
                async move {
                    let (sender, receiver) = futures::channel::oneshot::channel::<
                        Result<(), SendFailure>,
                    >();
                    out_sender
                        .try_lock()
                        .unwrap()
                        .send((block, sender))
                        .await
                        .unwrap();
                    receiver.await.unwrap()
                }
            }),
        )
    }
}

/// A callback that is called by the com hub to send data through the interface.
#[derive(Clone)]
pub enum SendCallback {
    /// A synchronous send callback.
    /// The callback receives a DXBBlock and the UUID of the socket to send the data through.
    /// It returns a SendSuccess result which can contain already received data from the remote side.
    /// The failure case returns a SendFailure containing the original DXBBlock.
    Sync(Rc<dyn Fn(DXBBlock) -> Result<SendSuccess, SendFailure> + 'static>),
    SyncOnce(Rc<dyn Fn(DXBBlock) -> Result<SendSuccess, SendFailure>>),
    /// An asynchronous send callback.
    /// The callback receives a DXBBlock and the UUID of the socket to send the data through.
    /// It returns a future that resolves to a Result indicating success or failure.
    /// The success case does not return any data, as any received data should be handled
    /// through the receive iterator.
    /// The failure case returns a SendFailure containing the original DXBBlock.
    Async(AsyncCallback<DXBBlock, Result<(), SendFailure>>),
}

impl Debug for SendCallback {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            SendCallback::Sync(_) => write!(f, "SendCallback::Sync(...)"),
            SendCallback::SyncOnce(_) => {
                write!(f, "SendCallback::SyncOnce(...)")
            }
            SendCallback::Async(_) => write!(f, "SendCallback::Async(...)"),
        }
    }
}

#[derive(Default)]
pub enum SendSuccess {
    /// Indicates that the data was sent successfully without any immediate received data.
    #[default]
    Sent,
    /// Indicates that the data was sent successfully and includes data received
    /// from the remote side (possibly in response to the sent data).
    SentWithNewIncomingData(Vec<u8>),
}

#[derive(Debug, Clone)]
pub struct SendFailure(pub Box<DXBBlock>);

impl SendCallback {
    pub fn new_sync(
        f: impl Fn(DXBBlock) -> Result<SendSuccess, SendFailure> + 'static,
    ) -> Self {
        SendCallback::Sync(Rc::new(f))
    }

    // Sync send callback that can only be called once - after that, it returns SendFailure
    pub fn new_sync_once(
        f: impl FnOnce(DXBBlock) -> Result<SendSuccess, SendFailure>
        + 'static
        + Send
        + Sync,
    ) -> Self {
        let once_fn = Box::new(Mutex::new(Some(f)));
        let wrapper = move |block: DXBBlock| {
            let mut lock = once_fn.try_lock().unwrap();
            if let Some(func) = lock.take() {
                func(block)
            } else {
                Err(SendFailure(Box::new(block)))
            }
        };
        SendCallback::SyncOnce(Rc::new(wrapper))
    }

    pub fn new_async<F, Fut>(f: F) -> Self
    where
        F: Fn(DXBBlock) -> Fut + 'static,
        Fut: core::future::Future<Output = Result<(), SendFailure>> + 'static,
    {
        SendCallback::Async(AsyncCallback::new(f))
    }
}

pub struct ComInterfaceConfiguration {
    // should not be provided from JS side
    uuid: ComInterfaceUUID,
    /// The properties of the interface instance
    pub properties: Rc<ComInterfaceProperties>,
    /// Indicates that this interface only establishes a single socket connection
    /// And stops the sockets iterator after yielding the first socket configuration.
    /// When set to true, the first socket connection is awaited on interface creation.
    pub has_single_socket: bool,
    // TODO #725: docs
    pub new_sockets_iterator: NewSocketsIterator,
    /// An optional asynchronous callback that is called by the com hub when the interface is closed
    pub close_async_callback: Option<CloseAsyncCallback>,
}

impl Debug for ComInterfaceConfiguration {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ComInterfaceConfiguration")
            .field("uuid", &self.uuid)
            .field("properties", &self.properties)
            .finish()
    }
}

impl ComInterfaceConfiguration {
    /// Creates a new ComInterfaceConfiguration with the given properties and socket iterator.
    pub fn new_multi_socket<I>(
        properties: ComInterfaceProperties,
        new_sockets_iterator: I,
    ) -> Self
    where
        I: AsyncIterator<Item = Result<SocketConfiguration, ()>> + 'static,
    {
        ComInterfaceConfiguration {
            uuid: ComInterfaceUUID::new(),
            properties: Rc::new(properties),
            has_single_socket: false,
            new_sockets_iterator: Box::pin(new_sockets_iterator),
            close_async_callback: None,
        }
    }

    /// Creates a new ComInterfaceConfiguration with a single socket configuration.
    pub fn new_single_socket(
        properties: ComInterfaceProperties,
        socket_configuration: SocketConfiguration,
    ) -> Self {
        ComInterfaceConfiguration {
            uuid: ComInterfaceUUID::new(),
            properties: Rc::new(properties),
            has_single_socket: true,
            new_sockets_iterator: Box::pin(async gen move {
                yield Ok(socket_configuration)
            }),
            close_async_callback: None,
        }
    }

    /// Creates a new ComInterfaceConfiguration
    pub fn new<I, F, Fut>(
        properties: ComInterfaceProperties,
        has_single_socket: bool,
        new_sockets_iterator: I,
        close_async_callback: Option<F>,
    ) -> Self
    where
        I: AsyncIterator<Item = Result<SocketConfiguration, ()>> + 'static,
        F: FnOnce() -> Fut + 'static,
        Fut: Future<Output = ()> + 'static,
    {
        ComInterfaceConfiguration {
            uuid: ComInterfaceUUID::new(),
            properties: Rc::new(properties),
            has_single_socket,
            new_sockets_iterator: Box::pin(new_sockets_iterator),
            close_async_callback: close_async_callback.map(|cb| {
                Box::new(move || {
                    Box::pin(cb()) as Pin<Box<dyn Future<Output = ()>>>
                }) as CloseAsyncCallback
            }),
        }
    }

    pub fn uuid(&self) -> ComInterfaceUUID {
        self.uuid.clone()
    }
}

pub type CloseAsyncCallback = Box<dyn FnOnce() -> LocalBoxFuture<'static, ()>>;

/// This trait can be implemented to provide a factory with a synchronous setup process
/// for a ComInterface implementation that can be registered on a ComHub.
/// The trait should be implemented for the setup data type of the interface.
/// Example:
/// ```
/// #![feature(gen_blocks)]
///
/// use datex_macros_internal::Datex;
/// use datex_core::network::com_hub::errors::ComInterfaceCreateError;
/// use datex_core::network::com_interfaces::com_interface::factory::{ComInterfaceSyncFactory,ComInterfaceConfiguration};
/// use datex_core::network::com_interfaces::com_interface::properties::ComInterfaceProperties;
///
/// #[derive(Datex)]
/// # #[datex(_force_datex_core_namespace)]
/// struct ExampleInterfaceSetupData {
///    pub example_data: String,
/// }
///
/// impl ComInterfaceSyncFactory for ExampleInterfaceSetupData {
///     fn create_interface(
///         self,
///     ) -> Result<ComInterfaceConfiguration, ComInterfaceCreateError> {
///         todo!("#726 Initialize the interface here")
///     }
///
///     fn get_default_properties() -> ComInterfaceProperties {
///         ComInterfaceProperties {
///             interface_type: "example".to_string(),
///             ..Default::default()
///         }
///     }
/// }
pub trait ComInterfaceSyncFactory
where
    Self: ConvertValue + FromParts + Sized,
{
    /// The factory method that is called from the ComHub on a registered interface
    /// to create a new instance of the interface.
    /// The setup data is passed as a ValueContainer and has to be downcasted
    fn factory(
        setup_data: Value,
    ) -> Result<ComInterfaceConfiguration, ComInterfaceCreateError> {
        let setup_data = setup_data
            .try_convert_into_value()
            .map_err(|_| ComInterfaceCreateError::SetupDataParseError)?;
        Self::create_interface(setup_data)
    }

    /// Create a new instance of the interface with the given setup data.
    /// If no instance could be created with the given setup data,
    /// None is returned.
    fn create_interface(
        self,
    ) -> Result<ComInterfaceConfiguration, ComInterfaceCreateError>;

    /// Get the default interface properties for the interface.
    fn get_default_properties() -> ComInterfaceProperties;
}

/// This trait can be implemented to provide a factory with an asynchronous setup process
/// for a ComInterface implementation that can be registered on a ComHub.
/// The trait should be implemented for the setup data type of the interface.
/// Example:
/// ```
/// #![feature(gen_blocks)]
///
/// use datex_macros_internal::Datex;
/// use datex_core::network::com_hub::errors::ComInterfaceCreateError;
/// use datex_core::network::com_interfaces::com_interface::factory::ComInterfaceAsyncFactory;
/// use datex_core::network::com_interfaces::com_interface::properties::ComInterfaceProperties;
/// use datex_core::network::com_hub::managers::com_interface_manager::ComInterfaceAsyncFactoryResult;///
///
/// #[derive(Datex)]
/// # #[datex(_force_datex_core_namespace)]
/// struct ExampleInterfaceSetupData {
///    pub example_data: String,
/// }
///
/// impl ComInterfaceAsyncFactory for ExampleInterfaceSetupData {
///     fn create_interface(
///         self
///     ) -> ComInterfaceAsyncFactoryResult {
///         Box::pin(async move {
///             // Initialize the interface here asynchronously
///             todo!("#727 Undescribed by author.")
///         })
///     }
///     fn get_default_properties() -> ComInterfaceProperties {
///         ComInterfaceProperties {
///             interface_type: "example".to_string(),
///             ..Default::default()
///         }
///     }
/// }
pub trait ComInterfaceAsyncFactory
where
    Self: ConvertValue + FromParts + Sized,
{
    /// The factory method that is called from the ComHub on a registered interface
    /// to create a new instance of the interface.
    /// The setup data is passed as a ValueContainer and has to be downcasted
    fn factory(setup_data: Value) -> ComInterfaceAsyncFactoryResult {
        Box::pin(async move {
            let setup_data = setup_data
                .try_convert_into_value()
                .map_err(|_| ComInterfaceCreateError::SetupDataParseError)?;
            Self::create_interface(setup_data).await
        })
    }

    /// Create a new instance of the interface with the given setup data.
    /// If no instance could be created with the given setup data,
    /// None is returned.
    fn create_interface(self) -> ComInterfaceAsyncFactoryResult;

    /// Get the default interface properties for the interface.
    fn get_default_properties() -> ComInterfaceProperties;
}
