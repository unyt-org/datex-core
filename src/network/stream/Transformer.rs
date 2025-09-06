use tungstenite::buffer;

use crate::{
    global::dxb_block::DXBBlock,
    network::stream::Stream::{QueuingStream, Stream},
};

#[derive(Debug, Clone)]
pub enum StreamKind {
    DATEXBlock,
    MediaSample,
    RTPPacket,
    Binary,
}

#[derive(Debug, Clone, Copy)]
pub enum Reliability {
    BestEffort,
    RequireAll,
}

#[derive(Debug, Clone)]
pub struct TransformerConfig {
    pub input: StreamKind,
    pub output: StreamKind,
    pub slice_size: usize,
}

pub struct StreamTransformer {
    config: TransformerConfig,
    parse_buffer: Vec<u8>,
}

// pub trait Transform<I, O, InStream, OutStream>
// where
//     InStream: Stream<I>,
//     OutStream: Stream<O>,
// {
//     fn ingest(&mut self, input: I);

//     fn process(&mut self, input: &mut InStream, output: &mut OutStream) {
//         while let Some(item) = input.next() {
//             self.ingest(item);
//         }

//         if input.is_ended() {
//             output.end();
//         }
//     }
// }

struct IOHolder<'a, I: 'static, O: 'static> {
    inputs: Vec<&'a mut dyn Stream<I>>,
    outputs: Vec<&'a mut dyn Stream<O>>,
}

impl<'a, I: 'static, O: 'static> IOHolder<'a, I, O> {
    pub fn new() -> Self {
        Self {
            inputs: Vec::new(),
            outputs: Vec::new(),
        }
    }
}

pub trait Transform<'a, I: 'static, O: 'static>
where
    Self: Sized,
{
    fn process<InStream>(&'a mut self, input: &mut InStream)
    where
        InStream: Stream<I>,
    {
        while let Some(item) = input.next() {
            self.ingest(item);
        }

        if input.is_ended() {
            self.close();
        }
    }
    fn holder(&'a mut self) -> &mut IOHolder<'a, I, O>;

    // fn holder<'a>(&'a mut self) -> &'a mut IOHolder<'a, I, O>;
    fn add_input(&'a mut self, input: &'a mut dyn Stream<I>) {
        self.holder().inputs.push(input);
    }

    // add_output ohne Ownership zu übernehmen
    fn add_output(&'a mut self, output: &'a mut dyn Stream<O>) {
        self.holder().outputs.push(output);
    }

    fn inputs(&'a mut self) -> &'a mut Vec<&'a mut dyn Stream<I>> {
        &mut self.holder().inputs
    }

    fn outputs(&'a mut self) -> &'a mut Vec<&'a mut dyn Stream<O>> {
        &mut self.holder().outputs
    }
    // fn holder(&mut self) -> &mut IOHolder<I, O>;

    // fn add_output<S: Stream<O> + 'static>(&mut self, output: S) {
    //     self.holder().outputs.push(Box::new(output));
    // }
    // fn add_input<S: Stream<I> + 'static>(&mut self, input: S) {
    //     self.holder().inputs.push(Box::new(input));
    // }

    // fn outputs(&mut self) -> &mut Vec<Box<dyn Stream<O>>> {
    //     &mut self.holder().outputs
    // }

    // fn inputs(&mut self) -> &mut Vec<Box<dyn Stream<I>>> {
    //     &mut self.holder().inputs
    // }

    fn emit(&'a mut self, item: O)
    where
        O: Clone,
    {
        for out in self.outputs().iter_mut() {
            out.push(item.clone());
        }
    }

    fn emit_owned(&'a mut self, item: O)
    where
        O: Clone,
    {
        let mut first = true;
        for out in self.outputs().iter_mut() {
            if first {
                out.push(item.clone());
                first = false;
            } else {
                out.push(item.clone());
            }
        }
    }

    fn end_all(&'a mut self) {
        for out in self.outputs().iter_mut() {
            out.end();
        }
    }

    fn ingest(&'a mut self, input: I);

    fn close(&'a mut self);
}

pub struct BinaryToDATEXBlockTransformer<'a> {
    buffer: Vec<u8>,
    slice_size: usize,
    holder: IOHolder<'a, u8, DXBBlock>,
}
impl<'a> BinaryToDATEXBlockTransformer<'a> {
    pub fn new(slice_size: usize) -> Self {
        Self {
            buffer: Vec::new(),
            slice_size,
            holder: IOHolder::new(),
        }
    }
    fn collect(&'a mut self) {
        let mut blocks = vec![];
        {
            let size = self.slice_size;
            let buffer = &mut self.buffer;
            while buffer.len() >= size {
                let data: Vec<u8> = buffer.drain(..size).collect();
                let mut block = DXBBlock {
                    body: data,
                    ..Default::default()
                };
                block.recalculate_struct();
                blocks.push(block);
            }
        }
        // drop(buffer);
        for block in blocks.drain(..) {
            // moves blocks out
            // self.emit(block);
        }
    }
}

impl<'a> Transform<'a, u8, DXBBlock> for BinaryToDATEXBlockTransformer<'a> {
    fn ingest(&'a mut self, byte: u8) {
        self.buffer.push(byte);
        self.collect();
    }

    fn close(&'a mut self) {
        if !self.buffer.is_empty() {
            self.collect();
        }
        // self.end_all();
    }

    fn holder(&'a mut self) -> &mut IOHolder<'a, u8, DXBBlock> {
        &mut self.holder
    }
}
