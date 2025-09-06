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

struct IOHolder<I, O> {
    inputs: Vec<Box<dyn Stream<I>>>,
    outputs: Vec<Box<dyn Stream<O>>>,
}

pub trait Transform<I, O>
where
    Self: Sized,
{
    fn process<InStream>(&mut self, input: &mut InStream)
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

    fn outputs(&mut self) -> &mut Vec<Box<dyn Stream<O>>>;

    fn add_output<S: Stream<O> + 'static>(&mut self, output: S) {
        self.outputs().push(Box::new(output));
    }

    fn emit(&mut self, item: O)
    where
        O: Clone,
    {
        for out in self.outputs().iter_mut() {
            out.push(item.clone());
        }
    }

    fn emit_owned(&mut self, item: O)
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

    fn end_all(&mut self) {
        for out in self.outputs().iter_mut() {
            out.end();
        }
    }

    fn ingest(&mut self, input: I);

    fn close(&mut self);
}

// pub struct BaseTransformer<O> {
//     outputs: Vec<Box<dyn Stream<O>>>,
// }

// impl<O> BaseTransformer<O>
// where
//     O: Clone,
// {
//     pub fn new() -> Self {
//         Self {
//             outputs: Vec::new(),
//         }
//     }

//     fn outputs(&mut self) -> &mut Vec<Box<dyn Stream<O>>>;

//     /// Add an output stream
//     fn add_output<S: Stream<O> + 'static>(&mut self, output: S) {
//         self.outputs().push(Box::new(output));
//     }

//     pub fn emit(&mut self, item: O)
//     where
//         O: Clone,
//     {
//         for out in self.outputs.iter_mut() {
//             out.push(item.clone());
//         }
//     }

//     pub fn emit_owned(&mut self, item: O) {
//         let mut first = true;
//         for out in self.outputs.iter_mut() {
//             if first {
//                 out.push(item.clone());
//                 first = false;
//             } else {
//                 out.push(item.clone());
//             }
//         }
//     }
// }

pub struct BinaryToDATEXBlockTransformer {
    buffer: Vec<u8>,
    slice_size: usize,
    outputs: Vec<Box<dyn Stream<DXBBlock>>>,
}
impl BinaryToDATEXBlockTransformer {
    pub fn new(slice_size: usize) -> Self {
        Self {
            buffer: Vec::new(),
            slice_size,
            outputs: Vec::new(),
        }
    }
    fn collect(&mut self) {
        while self.buffer.len() >= self.slice_size {
            let data: Vec<u8> = self.buffer.drain(..self.slice_size).collect();
            let mut block = DXBBlock {
                body: data,
                ..Default::default()
            };
            block.recalculate_struct();
            self.emit(block);
        }
    }
}

impl Transform<u8, DXBBlock> for BinaryToDATEXBlockTransformer {
    fn ingest(&mut self, byte: u8) {
        self.buffer.push(byte);
        self.collect();
    }

    fn close(&mut self) {
        if !self.buffer.is_empty() {
            self.collect();
        }
        self.end_all();
    }

    fn outputs(&mut self) -> &mut Vec<Box<dyn Stream<DXBBlock>>> {
        &mut self.outputs
    }
}
