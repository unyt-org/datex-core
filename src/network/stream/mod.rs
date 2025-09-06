pub mod Stream;
pub mod Stream2;
pub mod Transformer;
#[cfg(test)]
mod tests {
    use ntest_timeout::timeout;
    use std::time::Duration;
    use tokio::task::spawn_local;

    use crate::{
        global::dxb_block::DXBBlock,
        network::stream::{
            Stream::{QueuingStream, Stream},
            Transformer::{BinaryToDATEXBlockTransformer, Transform},
        },
        run_async,
        task::sleep,
    };

    #[tokio::test]
    #[timeout(2000)]
    async fn stream() {
        run_async! {
            // binary input
            let input_stream = QueuingStream::<u8>::default().to_ref_cell();

            // dxb output
            let output_stream =
                QueuingStream::<DXBBlock>::default().to_ref_cell();

            // transform
            let mut transformer = BinaryToDATEXBlockTransformer::new(4);
            transformer.add_output(output_stream.clone());
            transformer.add_input(input_stream.clone());

            spawn_local(async move {
                loop {
                    input_stream.borrow_mut().push(1);
                    sleep(Duration::from_millis(1)).await;
                }
            });
            spawn_local(async move {
                loop {
                    transformer.next();
                    if let Some(block) = output_stream.borrow_mut().next() {
                        println!("Received block: {:?}", block.body);
                    }
                    if output_stream.borrow().is_ended() {
                        break;
                    }
                    sleep(Duration::from_millis(10)).await;
                }
            });
            sleep(Duration::from_millis(500)).await;

        }
    }
}
