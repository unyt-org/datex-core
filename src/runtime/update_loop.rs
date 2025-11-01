use crate::global::dxb_block::{DXBBlock, OutgoingContextId};
use crate::global::protocol_structures::block_header::FlagsAndTimestamp;
use crate::global::protocol_structures::block_header::{
    BlockHeader, BlockType,
};
use crate::global::protocol_structures::encrypted_header::EncryptedHeader;
use crate::global::protocol_structures::routing_header::RoutingHeader;
use crate::runtime::RuntimeInternal;
use crate::runtime::execution::ExecutionError;
use crate::task::{sleep, spawn_with_panic_notify};
use crate::values::core_values::endpoint::Endpoint;
use crate::values::value_container::ValueContainer;
use futures::channel::oneshot;
use log::info;
use datex_core::core_compiler::value_compiler::compile_value_container;
use crate::stdlib::rc::Rc;
use crate::stdlib::time::Duration;

impl RuntimeInternal {
    /// Starts the
    pub fn start_update_loop(self_rc: Rc<RuntimeInternal>) {
        info!("starting runtime update loop...");

        // if already running, do nothing
        if *self_rc.update_loop_running.borrow() {
            return;
        }

        // set update loop running flag
        *self_rc.update_loop_running.borrow_mut() = true;

        spawn_with_panic_notify(async move {
            while *self_rc.update_loop_running.borrow() {
                RuntimeInternal::update(self_rc.clone()).await;
                sleep(Duration::from_millis(1)).await;
            }
            if let Some(sender) =
                self_rc.update_loop_stop_sender.borrow_mut().take()
            {
                sender.send(()).expect("Failed to send stop signal");
            }
        });
    }

    /// Stops the update loop for the Runtime, if it is running.
    pub async fn stop_update_loop(self_rc: Rc<RuntimeInternal>) {
        info!("Stopping Runtime update loop for {}", self_rc.endpoint);
        *self_rc.update_loop_running.borrow_mut() = false;

        let (sender, receiver) = oneshot::channel::<()>();

        self_rc.update_loop_stop_sender.borrow_mut().replace(sender);

        receiver.await.unwrap();
    }

    /// main update loop
    async fn update(self_rc: Rc<RuntimeInternal>) {
        // update the ComHub
        self_rc.com_hub.update();
        // handle incoming sections
        RuntimeInternal::handle_incoming_sections(self_rc);
    }

    /// pops incoming sections from the ComHub and executes them in separate tasks
    fn handle_incoming_sections(self_rc: Rc<RuntimeInternal>) {
        let mut sections = self_rc
            .com_hub
            .block_handler
            .incoming_sections_queue
            .borrow_mut();
        // get incoming sections from ComHub
        for section in sections.drain(..) {
            // execute the section in a separate task
            let self_rc = self_rc.clone();
            spawn_with_panic_notify(async move {
                let (result, endpoint, context_id) =
                    RuntimeInternal::execute_incoming_section(
                        self_rc.clone(),
                        section,
                    )
                    .await;
                info!(
                    "Execution result (on {} from {}): {result:?}",
                    self_rc.endpoint, endpoint
                );
                // send response back to the sender
                let res = RuntimeInternal::send_response_block(
                    self_rc.clone(),
                    result,
                    endpoint,
                    context_id,
                );
                // TODO #231: handle errors in sending response
            });
        }
    }

    fn send_response_block(
        self_rc: Rc<RuntimeInternal>,
        result: Result<Option<ValueContainer>, ExecutionError>,
        receiver_endpoint: Endpoint,
        context_id: OutgoingContextId,
    ) -> Result<(), Vec<Endpoint>> {
        let routing_header: RoutingHeader = RoutingHeader::default()
            .with_sender(self_rc.endpoint.clone())
            .to_owned();

        let block_header = BlockHeader {
            context_id,
            flags_and_timestamp: FlagsAndTimestamp::new()
                .with_block_type(BlockType::Response)
                .with_is_end_of_section(true)
                .with_is_end_of_context(true),
            ..BlockHeader::default()
        };
        let encrypted_header = EncryptedHeader::default();

        info!(
            "send response, context_id: {context_id:?}, receiver: {receiver_endpoint}"
        );

        if let Ok(value) = result {
            let dxb = if let Some(value) = &value {
                compile_value_container(value)
            } else {
                vec![]
            };

            let mut block = DXBBlock::new(
                routing_header,
                block_header,
                encrypted_header,
                dxb,
            );
            block.set_receivers(std::slice::from_ref(&receiver_endpoint));

            self_rc.com_hub.send_own_block(block)
        } else {
            core::todo!("#233 Handle returning error response block");
        }
    }
}
