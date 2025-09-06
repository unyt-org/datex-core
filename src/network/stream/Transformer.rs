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

struct IOHolder<'a, I, O> {
    inputs: Vec<Box<dyn Stream<I> + 'a>>,
    outputs: Vec<Box<dyn Stream<O> + 'a>>,
}
impl<'a, I, O> IOHolder<'a, I, O> {
    pub fn new() -> Self {
        Self {
            inputs: Vec::new(),
            outputs: Vec::new(),
        }
    }
}

pub trait Transform<'a, I: 'a, O: 'a>
where
    Self: Sized,
{
    fn process<InStream>(&'static mut self, input: &mut InStream)
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

    fn get_holder(&'a mut self) -> &mut IOHolder<'a, I, O>;

    fn add_output<S: Stream<O> + 'a>(&'a mut self, output: S) {
        self.get_holder().outputs.push(Box::new(output));
    }

    fn outputs(&'a mut self) -> &mut Vec<Box<dyn Stream<O> + 'a>> {
        &mut self.get_holder().outputs
    }
    fn inputs(&'a mut self) -> &mut Vec<Box<dyn Stream<I> + 'a>> {
        &mut self.get_holder().inputs
    }

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
            self.emit(block);
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
        self.end_all();
    }

    fn get_holder(&'a mut self) -> &mut IOHolder<u8, DXBBlock> {
        &mut self.holder
    }
}
