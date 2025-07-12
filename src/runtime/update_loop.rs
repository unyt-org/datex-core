use std::rc::Rc;
use std::time::Duration;
use futures::channel::oneshot;
use log::info;
use crate::runtime::Runtime;
use crate::task::{sleep, spawn_with_panic_notify};

impl Runtime {
    /// Starts the 
    pub fn start_update_loop(self_rc: Rc<Runtime>) {
        info!("starting runtime update loop...");

        // if already running, do nothing
        if *self_rc.update_loop_running.borrow() {
            return;
        }

        // set update loop running flag
        *self_rc.update_loop_running.borrow_mut() = true;

        spawn_with_panic_notify(async move {
            while *self_rc.update_loop_running.borrow() {
                self_rc.update();
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
    pub async fn stop_update_loop(&self) {
        info!("Stopping Runtime update loop for {}", self.endpoint);
        *self.update_loop_running.borrow_mut() = false;

        let (sender, receiver) = oneshot::channel::<()>();

        self.update_loop_stop_sender.borrow_mut().replace(sender);

        receiver.await.unwrap();
    }
    
    fn update(&self) {
        // Update logic goes here
        
    }
}