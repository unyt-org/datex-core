use std::rc::Rc;
use std::time::Duration;
use futures::channel::oneshot;
use log::info;
use crate::network::com_hub::ComHub;
use crate::runtime::execution_context::ExecutionContext;
use crate::runtime::{RuntimeInternal};
use crate::task::{sleep, spawn_with_panic_notify};

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
        RuntimeInternal::handle_incoming_sections(self_rc).await;
    }

    async fn handle_incoming_sections(self_rc: Rc<RuntimeInternal>) {
        // get incoming sections from ComHub
        let mut incoming_sections =
            self_rc.com_hub.block_handler.incoming_sections_queue.borrow_mut();
        // process each section
        for section in incoming_sections.drain(..) {
            // handle the section (this is a placeholder, actual handling logic goes here)
            info!("Handling incoming section: {section:?}");
            let mut context = ExecutionContext::local();
            let result = RuntimeInternal::execute_incoming_section(self_rc.clone(), section, &mut context).await;
        }
    }
}