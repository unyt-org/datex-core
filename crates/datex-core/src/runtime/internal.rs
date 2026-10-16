use crate::{
    channel::mpsc::{UnboundedReceiver, create_unbounded_channel},
    collections::HashMap,
    core_compiler::{
        instruction_input::InstructionInput, core_compilation_context::DXBWithSharedValues,
        core_compile,
    },
    dif::dif_interface::DIFInterface,
    disassembler::{
        options::DisassemblerOptions, print_disassembled_with_options,
    },
    global::{
        dxb_block::{
            DXBBlock, IncomingEndpointContextSectionId, IncomingSection,
            OutgoingContextId,
        },
        protocol_structures::{
            block_header::BlockHeader, encrypted_header::EncryptedHeader,
            routing_header::RoutingHeader,
        },
    },
    libs::core::CoreLibrary,
    network::{
        com_hub::{
            ComHub, InterfacePriority, network_response::ResponseOptions,
        },
        com_interfaces::local_loopback_interface::LocalLoopbackInterfaceSetupData,
    },
    prelude::*,
    random::RandomState,
    runtime::{
        Runtime, RuntimeConfig, RuntimeConfigInterface,
        cache::shared_references_cache::SharedReferencesCache,
        execution::{
            ExecutionError,
            context::{
                ExecutionContext, ExecutionMode, RemoteExecutionContext,
                ScriptExecutionError,
            },
            execution_input::ExecutionCallerMetadata,
        },
        pointer_address_provider::SelfOwnedPointerAddressProvider,
        pointer_availability_lookup::PointerAvailabilityLookup,
        remote_value_sync::{
            SubscriberError, synced_value_data::SyncedValueData,
        },
    },
    shared_values::{
        SharedContainer, base_shared_value_container::observers::TransceiverId,
    },
    time::Instant,
    utils::task_manager::TaskManager,
    values::{
        core_values::endpoint::Endpoint, value_container::ValueContainer,
    },
};
use alloc::rc::Rc;
use core::{
    cell::{Ref, RefCell, RefMut},
    pin::Pin,
};
use core::ops::DerefMut;
use indexmap::IndexMap;
use log::{debug, error, info};
use crate::datex_registry::{all_datex_types, get_all_modules};
use crate::types::r#type::Type;

#[derive(Debug)]
pub struct RuntimeInternal {
    version: String,
    endpoint: Endpoint,

    core_library: CoreLibrary,

    shared_references_cache: RefCell<SharedReferencesCache>,
    pointer_address_provider: Rc<RefCell<SelfOwnedPointerAddressProvider>>,
    com_hub: Rc<ComHub>,
    config: RuntimeConfig,

    /// Public endpoint interface properties
    endpoint_properties: RefCell<IndexMap<String, ValueContainer, RandomState>>,

    /// counter to keep track of transceiver ids
    transceiver_counter: RefCell<u8>,

    task_manager: TaskManager,

    // receiver for incoming sections from com hub
    incoming_sections_receiver: RefCell<UnboundedReceiver<IncomingSection>>,

    /// active execution contexts, stored by context_id
    execution_contexts:
        RefCell<HashMap<IncomingEndpointContextSectionId, ExecutionContext>>,

    pointer_availability_lookup: RefCell<PointerAvailabilityLookup>,

    synced_values: RefCell<SyncedValueData>,
}

macro_rules! get_execution_context {
    // take context and self_rc as parameters
    ($self_rc:expr, $execution_context:expr) => {
        match $execution_context {
            Some(context) => {
                // set current runtime in execution context if local execution context
                if let &mut ExecutionContext::Local(ref mut local_context) = context {
                    local_context.set_runtime_internal($self_rc.clone());
                }
                context
            },
            None => {
               &mut ExecutionContext::local(ExecutionMode::Static, $self_rc.clone(), ExecutionCallerMetadata::local_default())
            }
        }
    };
}

impl From<Rc<RuntimeInternal>> for Runtime {
    fn from(value: Rc<RuntimeInternal>) -> Self {
        Runtime { internal: value }
    }
}

impl RuntimeInternal {
    pub(crate) fn new(
        endpoint: Endpoint,
        shared_references_cache: RefCell<SharedReferencesCache>,
        pointer_address_provider: Rc<RefCell<SelfOwnedPointerAddressProvider>>,
        config: RuntimeConfig,
        com_hub: Rc<ComHub>,
        task_manager: TaskManager,
        incoming_sections_receiver: UnboundedReceiver<IncomingSection>,
    ) -> RuntimeInternal {
        let pointer_availability_lookup =
            PointerAvailabilityLookup::new(endpoint.clone());
        let internal = RuntimeInternal {
            version: env!("CARGO_PKG_VERSION").to_string(),
            endpoint,
            shared_references_cache,
            pointer_address_provider,
            config,
            com_hub,
            task_manager,
            endpoint_properties: RefCell::new(IndexMap::default()),
            core_library: CoreLibrary::default(),
            incoming_sections_receiver: RefCell::new(
                incoming_sections_receiver,
            ),
            synced_values: RefCell::new(SyncedValueData::default()),
            execution_contexts: RefCell::new(HashMap::new()),
            transceiver_counter: RefCell::new(0),
            pointer_availability_lookup: RefCell::new(
                pointer_availability_lookup,
            ),
        };

        internal.register_static_data();

        internal
    }

    /// Registers global modules and types in the cache
    fn register_static_data(&self) {
        let mut cache = self.shared_references_cache_mut();
        let cache = cache.deref_mut();

        // register all registered rust modules as datex endpoint properties
        let modules = get_all_modules(cache);
        for (name, value) in modules {
            self.register_endpoint_property(&name, value.into());
        }

        // register all types in cache
        let types = all_datex_types(cache);
        for ty in types {
            if let Type::Entity(ty) = ty && let SharedContainer::Owned(container) = ty.shared_container() {
                cache.register_owned_shared_container(&container.derive_with_max_mutability())
            }
        }

    }

    pub fn version(&self) -> &str {
        &self.version
    }
    pub fn config(&self) -> &RuntimeConfig {
        &self.config
    }
    pub fn com_hub(&self) -> Rc<ComHub> {
        self.com_hub.clone()
    }
    pub fn endpoint(&self) -> &Endpoint {
        &self.endpoint
    }
    pub fn endpoint_properties(
        &self,
    ) -> Ref<'_, IndexMap<String, ValueContainer, RandomState>> {
        self.endpoint_properties.borrow()
    }
    pub fn endpoint_properties_mut(
        &self,
    ) -> RefMut<'_, IndexMap<String, ValueContainer, RandomState>> {
        self.endpoint_properties.borrow_mut()
    }
    pub fn get_endpoint_property_by_name(
        &'_ self,
        key: &str,
    ) -> Option<Ref<'_, ValueContainer>> {
        Ref::filter_map(self.endpoint_properties.borrow(), |props| {
            props.get(key)
        })
        .ok()
    }
    pub fn get_endpoint_property_by_name_mut(
        &'_ self,
        key: &str,
    ) -> Option<RefMut<'_, ValueContainer>> {
        RefMut::filter_map(self.endpoint_properties.borrow_mut(), |props| {
            props.get_mut(key)
        })
        .ok()
    }

    pub fn register_endpoint_property(&self, name: &str, value: ValueContainer) {
        self
            .endpoint_properties_mut()
            .insert(name.to_string(), value);
    }

    pub fn pointer_availability_lookup(
        &self,
    ) -> Ref<'_, PointerAvailabilityLookup> {
        self.pointer_availability_lookup.borrow()
    }
    pub fn pointer_availability_lookup_mut(
        &self,
    ) -> RefMut<'_, PointerAvailabilityLookup> {
        self.pointer_availability_lookup.borrow_mut()
    }
    pub fn synced_values(&self) -> Ref<'_, SyncedValueData> {
        self.synced_values.borrow()
    }
    pub fn synced_values_mut(&self) -> RefMut<'_, SyncedValueData> {
        self.synced_values.borrow_mut()
    }

    pub fn shared_references_cache_refcell(
        &self,
    ) -> &RefCell<SharedReferencesCache> {
        &self.shared_references_cache
    }

    pub fn shared_references_cache(&self) -> Ref<'_, SharedReferencesCache> {
        self.shared_references_cache.borrow()
    }

    pub fn shared_references_cache_mut(
        &self,
    ) -> RefMut<'_, SharedReferencesCache> {
        self.shared_references_cache.borrow_mut()
    }

    pub fn core_library(&self) -> &CoreLibrary {
        &self.core_library
    }

    pub fn pointer_address_provider_mut(
        &self,
    ) -> RefMut<'_, SelfOwnedPointerAddressProvider> {
        self.pointer_address_provider.borrow_mut()
    }
    pub fn incoming_sections_receiver_mut(
        &self,
    ) -> RefMut<'_, UnboundedReceiver<IncomingSection>> {
        self.incoming_sections_receiver.borrow_mut()
    }
    pub fn incoming_sections_receiver(
        &self,
    ) -> Ref<'_, UnboundedReceiver<IncomingSection>> {
        self.incoming_sections_receiver.borrow()
    }
    pub fn task_manager(&self) -> &TaskManager {
        &self.task_manager
    }

    pub fn stub() -> RuntimeInternal {
        let (sender, receiver) = create_unbounded_channel();
        RuntimeInternal::new(
            Endpoint::default(),
            RefCell::new(SharedReferencesCache::default()),
            Rc::new(RefCell::new(SelfOwnedPointerAddressProvider::new(
                Endpoint::default(),
            ))),
            RuntimeConfig::default(),
            ComHub::create(Endpoint::default(), sender).0,
            TaskManager::create().0,
            receiver,
        )
    }

    /// Creates all interfaces configured in the runtime config
    async fn create_configured_interfaces(&self) {
        if let Some(interfaces) = &self.config.interfaces {
            for RuntimeConfigInterface {
                interface_type,
                config,
                priority,
            } in interfaces.iter()
            {
                let create_future = self
                    .com_hub
                    .clone()
                    .create_interface(interface_type, config.clone(), *priority)
                    .await;
                match create_future {
                    Err(err) => {
                        error!(
                            "Failed to create interface \"{interface_type}\": {err}"
                        )
                    }
                    Ok((_, ready_receiver)) => {
                        if let Some(ready_receiver) = ready_receiver {
                            let _ = ready_receiver.await;
                        }
                    }
                }
            }
        }
    }

    async fn init_local_loopback_interface(&self) {
        // add default local loopback interface
        let local_interface_setup_data =
            LocalLoopbackInterfaceSetupData.create_interface().unwrap();

        let ready_signal = self
            .com_hub
            .clone()
            .add_interface_from_configuration(
                local_interface_setup_data,
                InterfacePriority::None,
            )
            .expect("Failed to add local loopback interface");
        // local loopback interface is single socket interface and should always return a ready signal
        // which should always resolve to Ok
        ready_signal.unwrap().await.unwrap()
    }

    /// Performs asynchronous initialization of the runtime
    pub(crate) async fn init_async(&self) {
        // create local loopback interface and other configured interfaces
        self.init_local_loopback_interface().await;
        self.create_configured_interfaces().await;
    }

    #[cfg(feature = "compiler")]
    pub async fn execute(
        self: Rc<RuntimeInternal>,
        script: &str,
        inserted_values: &[ValueContainer],
        execution_context: Option<&mut ExecutionContext>,
    ) -> Result<Option<ValueContainer>, ScriptExecutionError> {
        let execution_context = get_execution_context!(
            Runtime::from(self.clone()),
            execution_context
        );
        let compile_start = Instant::now();
        let dxb = execution_context.compile(
            script,
            inserted_values,
            execution_context.receivers(),
        )?;
        debug!(
            "[Compilation took {} ms]",
            compile_start.elapsed().as_millis()
        );
        let execute_start = Instant::now();
        let result = RuntimeInternal::execute_dxb(
            self,
            dxb,
            None,
            Some(execution_context),
            true,
        )
        .await
        .map_err(ScriptExecutionError::from);
        debug!(
            "[Execution took {} ms]",
            execute_start.elapsed().as_millis()
        );
        result
    }

    #[cfg(feature = "compiler")]
    pub fn execute_sync(
        self: Rc<RuntimeInternal>,
        script: &str,
        inserted_values: &[ValueContainer],
        execution_context: Option<&mut ExecutionContext>,
    ) -> Result<Option<ValueContainer>, ScriptExecutionError> {
        let execution_context = get_execution_context!(
            Runtime::from(self.clone()),
            execution_context
        );
        let compile_start = Instant::now();
        let dxb = execution_context.compile(
            script,
            inserted_values,
            execution_context.receivers(),
        )?;
        debug!(
            "[Compilation took {} ms]",
            compile_start.elapsed().as_millis()
        );
        let execute_start = Instant::now();
        let result = RuntimeInternal::execute_dxb_sync(
            self,
            dxb,
            None,
            Some(execution_context),
            true,
        )
        .map_err(ScriptExecutionError::from);
        debug!(
            "[Execution took {} ms]",
            execute_start.elapsed().as_millis()
        );
        result
    }

    pub fn execute_dxb<'a>(
        self: Rc<RuntimeInternal>,
        input: DXBWithSharedValues,
        initial_stack_values: Option<Vec<ValueContainer>>,
        execution_context: Option<&'a mut ExecutionContext>,
        _end_execution: bool,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<Option<ValueContainer>, ExecutionError>>
                + 'a,
        >,
    > {
        Box::pin(async move {
            let execution_context = get_execution_context!(
                Runtime::from(self.clone()),
                execution_context
            );
            match execution_context {
                ExecutionContext::Remote(context) => {
                    // initial stack values are not (yet) supported for remote execution
                    if initial_stack_values.is_some() {
                        return Err(ExecutionError::InvalidExecutionState);
                    }
                    RuntimeInternal::execute_remote(self, context, input).await
                }
                ExecutionContext::Local(_) => {
                    execution_context
                        .execute_dxb(input, initial_stack_values)
                        .await
                }
            }
        })
    }

    pub fn execute_dxb_sync(
        self: Rc<RuntimeInternal>,
        dxb: DXBWithSharedValues,
        initial_stack_values: Option<Vec<ValueContainer>>,
        execution_context: Option<&mut ExecutionContext>,
        _end_execution: bool,
    ) -> Result<Option<ValueContainer>, ExecutionError> {
        let execution_context =
            get_execution_context!(Runtime::from(self), execution_context);
        match execution_context {
            ExecutionContext::Remote(_) => {
                Err(ExecutionError::RequiresAsyncExecution)
            }
            ExecutionContext::Local(_) => {
                execution_context.execute_dxb_sync(dxb, initial_stack_values)
            }
        }
    }

    /// Returns the existing execution context for the given context_id,
    /// or creates a new one if it doesn't exist.
    /// To reuse the context later, the caller must store it back in the map after use.
    fn take_execution_context(
        self: Rc<RuntimeInternal>,
        context_id: &IncomingEndpointContextSectionId,
        incoming_section: &IncomingSection,
    ) -> ExecutionContext {
        let runtime = Runtime::from(self.clone());
        let mut execution_contexts = self.execution_contexts.borrow_mut();
        // get execution context by context_id or create a new one if it doesn't exist
        let execution_context = execution_contexts.remove(context_id);
        if let Some(context) = execution_context {
            context
        } else {
            let caller_metadata = ExecutionCallerMetadata {
                endpoint: incoming_section.get_sender(),
            };
            ExecutionContext::local(
                ExecutionMode::unbounded(),
                runtime,
                caller_metadata,
            )
        }
    }

    pub async fn execute_remote(
        self: Rc<RuntimeInternal>,
        remote_execution_context: &RemoteExecutionContext,
        input: DXBWithSharedValues,
    ) -> Result<Option<ValueContainer>, ExecutionError> {
        let routing_header: RoutingHeader = RoutingHeader::default()
            .with_sender(self.endpoint.clone())
            .to_owned();

        // get existing context_id for context, or create a new one
        let context_id = {
            let mut context_ref =
                remote_execution_context.context_id.borrow_mut();
            context_ref.unwrap_or_else(|| {
                let id = self.com_hub.block_handler.get_new_context_id();
                // if the context_id is not set, we create a new one
                *context_ref = Some(id);
                id
            })
        };
        let block_header = BlockHeader {
            context_id,
            ..BlockHeader::default()
        };
        let encrypted_header = EncryptedHeader::default();

        let mut block = DXBBlock::new(
            routing_header,
            block_header,
            encrypted_header,
            input.dxb,
        );

        block.set_receivers(remote_execution_context.endpoints());
        // TODO: ensure in remote_execution_context that endpoints are never empty
        unsafe {
            self.clone().register_shared_containers_for_endpoints(
                &(remote_execution_context
                    .endpoints()
                    .iter()
                    .collect::<Vec<_>>()),
                input.shared_values,
            )?;
        }

        let response = self
            .com_hub
            .send_own_block_await_response(block, ResponseOptions::default())
            .await
            .remove(0)?;
        let incoming_section = response.take_incoming_section();

        // TODO: do we need to pass input.shared_values here to execution?
        RuntimeInternal::execute_incoming_section(self, incoming_section, None)
            .await
            .0
    }

    pub async fn execute_instructions_remote(
        self: Rc<RuntimeInternal>,
        endpoints: Vec<Endpoint>,
        instructions_input: Vec<InstructionInput>,
    ) -> Result<Option<ValueContainer>, ExecutionError> {
        let dxb = core_compile(
            &self.pointer_availability_lookup(),
            &endpoints,
            instructions_input,
        );

        let remote_execution_context = RemoteExecutionContext::new(
            endpoints,
            ExecutionMode::Static,
            Runtime::from(self.clone()),
        );
        self.execute_remote(&remote_execution_context, dxb).await
    }

    pub(crate) async fn execute_incoming_section(
        self: Rc<RuntimeInternal>,
        mut incoming_section: IncomingSection,
        shared_values: Option<Vec<SharedContainer>>,
    ) -> (
        Result<Option<ValueContainer>, ExecutionError>,
        Endpoint,
        OutgoingContextId,
    ) {
        let section_context_id =
            incoming_section.get_section_context_id().clone();
        let mut context = Self::take_execution_context(
            self.clone(),
            &section_context_id,
            &incoming_section,
        );
        info!(
            "Executing incoming section with index: {}",
            incoming_section.get_section_index()
        );

        let mut result = None;
        let mut last_block = None;

        // iterate over the blocks in the incoming section
        loop {
            let block = incoming_section.next().await;
            if let Some(block) = block {
                let res = RuntimeInternal::execute_dxb_block_local(
                    self.clone(),
                    block.clone(),
                    Some(&mut context),
                    // NOTE: this assumes that all shared values are only needed as references, not as owned values
                    shared_values.clone(),
                )
                .await;
                if let Err(err) = res {
                    return (
                        Err(err),
                        block.sender().clone(),
                        block.block_header.context_id,
                    );
                }
                result = res.unwrap();
                last_block = Some(block);
            } else {
                break;
            }
        }

        if last_block.is_none() {
            unreachable!("Incoming section must contain at least one block");
        }
        let last_block = last_block.unwrap();
        let sender_endpoint = last_block.sender().clone();
        let context_id = last_block.block_header.context_id;

        // insert the context back into the map for future use
        // TODO #638: is this needed or can we drop the context after execution here?
        self.execution_contexts
            .borrow_mut()
            .insert(section_context_id, context);

        (Ok(result), sender_endpoint, context_id)
    }

    async fn execute_dxb_block_local(
        self: Rc<RuntimeInternal>,
        block: DXBBlock,
        execution_context: Option<&mut ExecutionContext>,
        shared_values: Option<Vec<SharedContainer>>,
    ) -> Result<Option<ValueContainer>, ExecutionError> {
        let execution_context = get_execution_context!(
            Runtime::from(self.clone()),
            execution_context
        );
        // assert that the execution context is local
        if !core::matches!(execution_context, ExecutionContext::Local(_)) {
            unreachable!(
                "Execution context must be local for executing a DXB block"
            );
        }
        let dxb = block.body;
        info!("executing on {}:\n", self.endpoint,);

        print_disassembled_with_options(&dxb, DisassemblerOptions::default());

        let end_execution =
            block.block_header.flags_and_timestamp.is_end_of_section();

        RuntimeInternal::execute_dxb(
            self,
            DXBWithSharedValues::new(dxb, shared_values.unwrap_or_default()),
            None,
            Some(execution_context),
            end_execution,
        )
        .await
    }

    /// Registers a list of shared containers for a single endpoint.
    pub fn register_shared_containers_for_single_endpoint(
        self: Rc<Self>,
        endpoint: &Endpoint,
        shared_containers: Vec<SharedContainer>,
    ) {
        unsafe {
            self.register_shared_containers_for_endpoints(
                &[endpoint],
                shared_containers,
            )
            .unwrap()
        }
    }

    /// Registers a list of shared containers for a list of endpoints to subscribe.
    /// Note: only self owned pointers are registered, others are skipped
    ///
    /// # Safety
    /// The caller must ensure that endpoints is not empty.
    pub unsafe fn register_shared_containers_for_endpoints(
        self: Rc<Self>,
        endpoints: &[&Endpoint],
        shared_containers: Vec<SharedContainer>,
    ) -> Result<(), SubscriberError> {
        if endpoints.is_empty() {
            panic!("endpoints must not be empty");
        }

        for shared_container in shared_containers {
            // subscribe (remote) endpoints to own container
            if shared_container.is_self_owned() {
                for endpoint in endpoints {
                    // SAFETY: We checked that the shared container is self-owned
                    unsafe {
                        self.clone()
                            .subscribe_endpoint_to_owned_value(
                                &shared_container,
                                endpoint,
                                shared_container
                                    .derive_reference_with_max_mutability()
                                    .reference_mutability(),
                            )
                            .map_err(SubscriberError::Observer)?
                    }
                    // also store the subscribed pointer in cache so that we can handle incoming
                    // pointer updates from the subscribers
                    self.shared_references_cache
                        .borrow_mut()
                        .register_owned_shared_container(
                            &shared_container
                                .derive_reference_with_max_mutability(),
                        );
                }
            }
        }

        Ok(())
    }

    pub fn get_env(&self) -> IndexMap<String, String, RandomState> {
        self.config.env.clone().unwrap_or_default()
    }

    /// Creates a new [DIFInterface] with a unique transceiver id and the runtime's pointer address provider
    pub fn create_dif_interface(&self) -> DIFInterface {
        let count = *self.transceiver_counter.borrow();
        let id = TransceiverId::Dif(count);
        self.transceiver_counter.replace(count + 1);
        DIFInterface::new(id, self.pointer_address_provider.clone())
    }
}
