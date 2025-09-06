pub mod Stream;
pub mod Transformer;

#[cfg(test)]
mod tests {
    use tokio::task::spawn_local;

    use crate::{
        global::dxb_block::DXBBlock,
        network::stream::{
            Stream::{QueuingStream, Stream},
            Transformer::{BinaryToDATEXBlockTransformer, Transform},
        },
    };

    #[tokio::test]
    async fn stream() {
        // binary input
        let mut input_stream = QueuingStream::<u8>::default();

        // dxb output
        let mut output_stream = QueuingStream::<DXBBlock>::default();

        // transform
        let mut transformer = BinaryToDATEXBlockTransformer::new(4);
        transformer.add_output(&mut output_stream);
        transformer.add_input(&mut input_stream);

        spawn_local(async move {
            input_stream.push(1);
            input_stream.push(2);
            input_stream.push(3);
        });
    }
}
