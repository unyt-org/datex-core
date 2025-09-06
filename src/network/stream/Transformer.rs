use std::{cell::RefCell, rc::Rc};

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

pub struct IOHolder<I, O> {
    inputs: Vec<Rc<RefCell<dyn Stream<I>>>>,
    outputs: Vec<Rc<RefCell<dyn Stream<O>>>>,
}

impl<I, O> IOHolder<I, O> {
    pub fn new() -> Self {
        Self {
            inputs: Vec::new(),
            outputs: Vec::new(),
        }
    }
}

pub trait Transform<I, O>
where
    Self: Sized,
{
    fn consume<InStream>(&mut self, input: &mut InStream)
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
    fn holder(&self) -> &IOHolder<I, O>;
    fn holder_mut(&mut self) -> &mut IOHolder<I, O>;

    /// Pull data from all input streams and process them
    fn next(&mut self) {
        let inputs = self.holder().inputs.clone();
        for input in &inputs {
            if let Some(next) = input.borrow_mut().next() {
                self.ingest(next);
            }
        }
    }

    fn add_input(&mut self, input: Rc<RefCell<dyn Stream<I>>>) {
        self.holder_mut().inputs.push(input);
    }
    fn add_input_owned<S>(&mut self, input: S)
    where
        S: Stream<I> + 'static,
    {
        self.holder_mut().inputs.push(Rc::new(RefCell::new(input)));
    }

    fn add_output(&mut self, output: Rc<RefCell<dyn Stream<O>>>) {
        self.holder_mut().outputs.push(output);
    }
    fn add_output_owned<S>(&mut self, output: S)
    where
        S: Stream<O> + 'static,
    {
        self.holder_mut()
            .outputs
            .push(Rc::new(RefCell::new(output)));
    }

    fn ingest(&mut self, input: I);
    fn close(&mut self);

    /// Check if all output streams are closed
    fn is_closed(&self) -> bool {
        self.holder().outputs.iter().all(|o| o.borrow().is_ended())
    }
}

trait TransformInternal<I, O>: Transform<I, O>
where
    Self: Sized,
{
    fn emit(&mut self, item: O)
    where
        O: Clone,
    {
        for out in self.holder_mut().outputs.iter_mut() {
            out.borrow_mut().push(item.clone());
        }
    }
    fn end_all(&self) {
        for out in self.holder().outputs.iter() {
            out.borrow_mut().end();
        }
    }
}

pub struct BinaryToDATEXBlockTransformer {
    buffer: Vec<u8>,
    slice_size: usize,
    holder: IOHolder<u8, DXBBlock>,
}
impl TransformInternal<u8, DXBBlock> for BinaryToDATEXBlockTransformer {}
impl BinaryToDATEXBlockTransformer {
    pub fn new(slice_size: usize) -> Self {
        Self {
            buffer: Vec::new(),
            slice_size,
            holder: IOHolder::new(),
        }
    }
    fn collect(&mut self) {
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
        for block in blocks.drain(..) {
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

    fn holder(&self) -> &IOHolder<u8, DXBBlock> {
        &self.holder
    }
    fn holder_mut(&mut self) -> &mut IOHolder<u8, DXBBlock> {
        &mut self.holder
    }
}
