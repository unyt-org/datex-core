pub mod synced_value_data;

use crate::{
    core_compiler::{
        core_compilation_context::CompileInput,
        update_compiler::compile_updates,
    },
    prelude::*,
    runtime::{
        RuntimeInternal,
        execution::context::{ExecutionMode, RemoteExecutionContext},
    },
    shared_values::{
        PointerAddress, ReferenceMutability, ReferencedSharedContainer,
        SharedContainer, Subscribers,
        base_shared_value_container::observers::{
            ObserveOptions, Observer, ObserverError, TransceiverId,
        },
        traits::SharedContainerCommon,
    },
    value_updates::update_data::Update,
    values::core_values::endpoint::Endpoint,
};
use alloc::rc::Rc;
use core::{assert_matches, fmt::Display, ops::Deref};

#[derive(Debug, PartialEq, Eq)]
pub enum SubscriberError {
    NotASelfOwnedContainer,
    Observer(ObserverError),
}

impl From<ObserverError> for SubscriberError {
    fn from(err: ObserverError) -> Self {
        SubscriberError::Observer(err)
    }
}

impl Display for SubscriberError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            SubscriberError::NotASelfOwnedContainer => {
                write!(f, "The shared container is not self-owned.")
            }
            SubscriberError::Observer(err) => {
                write!(f, "Observer error: {}", err)
            }
        }
    }
}

impl RuntimeInternal {
    /// Sends local value updates back to the owner of a (remote) shared container.
    /// If the container is not mutable, this function does nothing.
    pub fn sync_value_with_owner(
        self: Rc<Self>,
        shared_container: &SharedContainer,
    ) -> Result<(), SubscriberError> {
        if shared_container.can_mutate() {
            // access as weak container so that the Rc can still be garbage collected
            let weak_container = shared_container
                .derive_reference_with_max_mutability()
                .downgrade();
            let self_clone = self.clone();

            let observer_id = shared_container.observe(Observer {
                transceiver_id: TransceiverId::Remote(
                    shared_container.pointer_address().endpoint(),
                ),
                callback: Rc::new(move |update| {
                    if let Some(container) = weak_container.upgrade() {
                        self_clone.task_manager().register_task(
                            self_clone.clone().send_update_to_owner(
                                container,
                                update.clone(),
                            ),
                        );
                    }
                }),
                options: ObserveOptions::default(),
            })?;

            // store observer
            let mut synced_values = self.synced_values_mut();
            synced_values
                .set_owner_observer(shared_container.clone(), observer_id);
        }
        Ok(())
    }

    /// Subscribes an endpoint to a self-owned shared container with the specified access rights.
    ///
    /// # Safety
    /// The caller must ensure that the shared container is self-owned.
    pub unsafe fn subscribe_endpoint_to_owned_value(
        self: Rc<Self>,
        shared_container: &SharedContainer,
        endpoint: &Endpoint,
        access_rights: ReferenceMutability,
    ) -> Result<(), ObserverError> {
        let mut synced_values = self.synced_values_mut();
        let subscribers = if let Some(subscribers) =
            synced_values.get_subscribers_mut(shared_container)
        {
            subscribers
                .remote_execution_context_mut()
                .add_endpoint(endpoint.clone());
            // TODO: when new subscriber is added, send initial block for context
            subscribers
        } else {
            let observer_id =
                if shared_container.base_shared_container().is_mutable() {
                    let container = shared_container.clone();
                    let self_clone = self.clone();
                    Some(shared_container.observe(Observer {
                        transceiver_id: TransceiverId::Remote(Endpoint::ANY),
                        callback: Rc::new(move |data| {
                            self_clone
                                .send_update_to_subscribers(&container, data)
                        }),
                        options: ObserveOptions::default(),
                    })?)
                } else {
                    None
                };

            let context = RemoteExecutionContext::new(
                vec![endpoint.clone()],
                ExecutionMode::Static,
                self.clone().into(),
            );

            unsafe {
                synced_values.set_subscribers(
                    shared_container,
                    Subscribers::new(observer_id, context),
                )
            }
        };
        subscribers.add_subscriber(endpoint, access_rights);

        Ok(())
    }

    /// Sends an update to the owner of the shared container.
    async fn send_update_to_owner(
        self: Rc<Self>,
        container: ReferencedSharedContainer,
        update: Update,
    ) {
        // a= [0, 43, 42]
        // a= [0, 43, 42]
        // @a -> a += 42 // + timestamp
        // ...
        // if ($a == 4) (
        //
        // )
        let owner = container.pointer_address().endpoint();

        let update_dxb = {
            let lookup = self.pointer_availability_lookup();
            let endpoints = [owner.clone()];
            let input = CompileInput::new(lookup.deref(), &endpoints);

            compile_updates(
                &SharedContainer::Referenced(container),
                &[update.data()],
                input,
            )
        };

        let self_clone = self.clone();

        let context = RemoteExecutionContext::new(
            vec![owner],
            ExecutionMode::Static,
            self.into(),
        );

        self_clone
            .execute_remote(&context, update_dxb)
            .await
            .expect("Failed to execute remote update block");
    }

    /// Handles an update to a shared container by notifying all subscribed endpoints.
    fn send_update_to_subscribers(
        self: &Rc<Self>,
        container: &SharedContainer,
        update: &Update,
    ) {
        let subscriber = self.synced_values();
        let subscribers = subscriber.get_subscribers(container);
        if let Some(subscribers) = subscribers {
            // TODO: don't send full update to source endpoint, but send update info with hash of the value
            let source_endpoint = Endpoint::from(update.source_id().clone())
                .as_local_if_endpoint(self.endpoint());

            let endpoints = subscribers
                .endpoints()
                .filter_map(|endpoint| {
                    let normalized_endpoint =
                        endpoint.as_local_if_endpoint(self.endpoint());
                    // filter out the source endpoint
                    if normalized_endpoint == source_endpoint {
                        None
                    } else {
                        Some(normalized_endpoint)
                    }
                })
                .collect::<Vec<_>>();

            if !endpoints.is_empty() {
                self.task_manager().register_task(
                    self.clone().send_update_block_to_subscribers(
                        // TODO: no clone?
                        container.clone(),
                        update.clone(),
                        endpoints,
                    ),
                );
            }
        }
    }

    /// Compiles a DXB block for the given update and sends it to the specified subscriber endpoints.
    /// Note: this function asserts that the shared container is still owned and that the remote execution
    /// context still exists. It also asserts that the receiver endpoints are not empty.
    async fn send_update_block_to_subscribers(
        self: Rc<RuntimeInternal>,
        container: SharedContainer,
        update: Update,
        receiver_endpoints: Vec<Endpoint>,
    ) {
        assert_matches!(
            container.pointer_address(),
            PointerAddress::SelfOwned(_)
        );

        let update_dxb = {
            let lookup = self.pointer_availability_lookup();
            let input = CompileInput::new(lookup.deref(), &receiver_endpoints);

            compile_updates(&container, &[update.data()], input)
        };

        let self_clone = self.clone();

        // FIXME: refcell hold across await point for context
        let mut subscriptions = self.synced_values_mut();

        let context = subscriptions
            .remote_execution_context_mut(&container)
            .unwrap();

        // Update the context's endpoints to the receiver endpoints for this update block.
        // This ensures that the endpoint that triggered this update does not get the update sent again
        // TODO: make sure that all endpoints that are skipped here get the number of skipped
        // blocks in the next block header so that they dont wait for this block (set skip_after_block)
        // SAFETY: send_update_block_to_subscribers is only called when receiver_endpoints is not empty
        unsafe {
            context.set_endpoints(receiver_endpoints);
        }

        self_clone
            .execute_remote(context, update_dxb)
            .await
            .expect("Failed to execute remote update block");
    }
}
