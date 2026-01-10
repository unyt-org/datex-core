use crate::core_compiler::value_compiler::compile_value_container;
use crate::global::dxb_block::{DXBBlock, IncomingSection, OutgoingContextId};
use crate::global::protocol_structures::block_header::FlagsAndTimestamp;
use crate::global::protocol_structures::block_header::{
    BlockHeader, BlockType,
};
use crate::global::protocol_structures::encrypted_header::EncryptedHeader;
use crate::global::protocol_structures::routing_header::RoutingHeader;
use crate::runtime::{AsyncContext, RuntimeInternal};
use crate::runtime::execution::ExecutionError;
use crate::stdlib::borrow::ToOwned;
use crate::stdlib::rc::Rc;
use crate::stdlib::vec;
use crate::stdlib::vec::Vec;
use crate::task::{sleep, spawn_with_panic_notify};
use crate::values::core_values::endpoint::Endpoint;
use crate::values::value_container::ValueContainer;
use core::prelude::rust_2024::*;
use core::result::Result;
use core::time::Duration;
use futures::channel::oneshot;
use log::info;
use datex_core::task::UnboundedReceiver;

#[cfg_attr(feature = "embassy_runtime", embassy_executor::task)]
async fn handle_incoming_section_task(
    runtime_rc: Rc<RuntimeInternal>,
    section: IncomingSection,
) {
    let (result, endpoint, context_id) =
        RuntimeInternal::execute_incoming_section(runtime_rc.clone(), section)
            .await;
    info!(
        "Execution result (on {} from {}): {result:?}",
        runtime_rc.endpoint, endpoint
    );
    // send response back to the sender
    let res = RuntimeInternal::send_response_block(
        runtime_rc.clone(),
        result,
        endpoint,
        context_id,
    )
    .await;
    // TODO #231: handle errors in sending response
}

#[cfg_attr(feature = "embassy_runtime", embassy_executor::task)]
async fn handle_incoming_sections_task(
    runtime_rc: Rc<RuntimeInternal>,
) {
    let async_context_clone = runtime_rc.async_context.clone();
    let mut sections_receiver = runtime_rc.com_hub.incoming_sections_receiver.borrow_mut().consume();

    while let Some(section) = sections_receiver.next().await {
        let runtime_rc_clone = runtime_rc.clone();
        spawn_with_panic_notify(
            &async_context_clone,
            handle_incoming_section_task(runtime_rc_clone, section),
        );
    }
}

impl RuntimeInternal {
    /// Spawns a task that receives incoming sections from the ComHub and executes them in separate tasks
    pub(crate) fn handle_incoming_sections(self_rc: Rc<RuntimeInternal>) {
        spawn_with_panic_notify(
            &self_rc.async_context.clone(),
            handle_incoming_sections_task(self_rc),
        );
    }

    async fn send_response_block(
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
            block.set_receivers(core::slice::from_ref(&receiver_endpoint));

            self_rc.com_hub.send_own_block(block).await
        } else {
            core::todo!("#233 Handle returning error response block");
        }
    }
}
