use crate::{
    network::{
        com_hub::{
            ComHub, InterfacePriority, managers::socket_manager::SocketManager,
        },
        com_interfaces::com_interface_old::{
            ComInterfaceOld, ComInterfaceSocketEvent,
        },
    },
    stdlib::{cell::RefCell, rc::Rc},
    task::{UnboundedReceiver, spawn_with_panic_notify},
};
use crate::network::com_interfaces::com_interface::ComInterface;

impl ComHub {
    pub(crate) fn handle_interface_socket_events(
        &self,
        interface: Rc<RefCell<ComInterface>>,
    ) {
        let mut interface_borrow = interface.borrow_mut();
        let socket_event_receiver =
            interface_borrow.take_socket_event_receiver();
        let interface_uuid = interface_borrow.uuid();
        let priority = self
            .interface_manager
            .borrow()
            .interface_priority(interface_uuid)
            .unwrap_or(InterfacePriority::None);
        spawn_with_panic_notify(
            &self.async_context,
            handle_interface_socket_events(
                socket_event_receiver,
                self.socket_manager.clone(),
                priority,
            ),
        );
    }
}

#[cfg_attr(feature = "embassy_runtime", embassy_executor::task)]
async fn handle_interface_socket_events(
    mut receiver_queue: UnboundedReceiver<ComInterfaceSocketEvent>,
    socket_manager: Rc<RefCell<SocketManager>>,
    priority: InterfacePriority,
) {
    while let Some(event) = receiver_queue.next().await {
        socket_manager
            .borrow_mut()
            .handle_socket_event(event, priority)
    }
}
