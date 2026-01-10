use crate::network::com_hub::ComHub;
use crate::network::com_interfaces::com_interface::ComInterface;
use crate::network::com_interfaces::com_interface::error::ComInterfaceError;
use crate::network::com_interfaces::com_interface::properties::InterfaceProperties;
use crate::network::com_interfaces::com_interface::socket::ComInterfaceSocketUUID;
use crate::serde::Deserialize;
use crate::serde::deserializer::from_value_container;
use crate::stdlib::any::Any;
use crate::stdlib::cell::RefCell;
use crate::stdlib::rc::Rc;
use crate::values::value_container::ValueContainer;
use core::pin::Pin;
use log::error;

pub trait ComInterfaceImplementation {
    fn send_block<'a>(
        &'a self,
        block: &'a [u8],
        _: ComInterfaceSocketUUID,
    ) -> Pin<Box<dyn Future<Output = bool> + 'a>>;

    fn get_properties(&self) -> InterfaceProperties;
    fn handle_close<'a>(&'a self) -> Pin<Box<dyn Future<Output = bool> + 'a>>;
    fn handle_open<'a>(&'a self) -> Pin<Box<dyn Future<Output = bool> + 'a>>;
}

/// A specific implementation of a communication interface for a channel
pub trait ComInterfaceImpl: ComInterfaceImplementation + Any {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T> ComInterfaceImpl for T
where
    T: ComInterfaceImplementation + Any,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// This trait can be implemented by any ComInterfaceImplementation impl that wants to
/// support a factory method for creating instances of the interface.
/// Example:
/// ```
/// # use core::cell::RefCell;
/// # use datex_core::stdlib::rc::Rc;
/// # use datex_core::network::com_interfaces::com_interface::{ComInterfaceError};
/// # use datex_core::network::com_interfaces::com_interface_implementation::ComInterfaceFactory;
/// # use datex_core::network::com_interfaces::com_interface_properties::InterfaceProperties;
/// use serde::{Deserialize, Serialize};
/// use datex_core::network::com_interfaces::default_com_interfaces::base_interface::BaseInterface;
///
/// #[derive(Serialize, Deserialize)]
/// struct BaseInterfaceSetupData {
///    pub example_data: String,
/// }
///
/// impl ComInterfaceFactory<BaseInterfaceSetupData> for BaseInterface {
///     fn create(setup_data: BaseInterfaceSetupData) -> Result<BaseInterface, ComInterfaceError> {
///         // ...
///         Ok(BaseInterface::new_with_name("example"))
///     }
///     fn get_default_properties() -> InterfaceProperties {
///         InterfaceProperties {
///             interface_type: "example".to_string(),
///             ..Default::default()
///         }
///     }
/// }
pub trait ComInterfaceFactory
where
    Self: Sized + ComInterfaceImpl,
{
    type SetupData: Deserialize<'static> + 'static;

    /// The factory method that is called from the ComHub on a registered interface
    /// to create a new instance of the interface.
    /// The setup data is passed as a ValueContainer and has to be downcasted
    fn factory(
        setup_data: ValueContainer,
        com_interface: Rc<ComInterface>,
    ) -> Result<Box<dyn ComInterfaceImpl>, ComInterfaceError> {
        let data = from_value_container::<Self::SetupData>(setup_data);
        match data {
            Ok(init_data) => {
                let interface = Self::create(init_data, com_interface);
                match interface {
                    Ok(interface) => Ok(Box::new(interface)),
                    Err(e) => Err(e),
                }
            }
            Err(e) => {
                error!("Failed to deserialize setup data: {e}");
                core::panic!("Invalid setup data for com interface factory")
            }
        }
    }

    /// Register the interface on which the factory is implemented
    /// on the given ComHub.
    fn register_on_com_hub(com_hub: Rc<ComHub>) {
        let interface_type = Self::get_default_properties().interface_type;
        com_hub.register_interface_factory(interface_type, Self::factory);
    }

    /// Create a new instance of the interface with the given setup data.
    /// If no instance could be created with the given setup data,
    /// None is returned.
    fn create(
        setup_data: Self::SetupData,
        com_interface: Rc<ComInterface>,
    ) -> Result<Self, ComInterfaceError>;

    /// Get the default interface properties for the interface.
    fn get_default_properties() -> InterfaceProperties;
}
