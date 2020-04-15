use crate::impl_::cell::Cell;
use crate::impl_::gc_node::{GcNode, Tracer};
use crate::impl_::lazy::Lazy;
use crate::impl_::sodium_ctx::SodiumCtx;
use crate::impl_::stream_loop::StreamLoop;

use std::mem;
use std::sync::Arc;
use std::sync::Mutex;

pub struct CellLoop<A> {
    pub data: Arc<Mutex<CellLoopData<A>>>,
    pub gc_node: GcNode
}

impl<A> Clone for CellLoop<A> {
    fn clone(&self) -> Self {
        self.gc_node.inc_ref();
        CellLoop {
            data: self.data.clone(),
            gc_node: self.gc_node.clone()
        }
    }
}

impl<A> Drop for CellLoop<A> {
    fn drop(&mut self) {
        self.gc_node.dec_ref();
    }
}

pub struct CellLoopData<A> {
    pub stream_loop: StreamLoop<A>,
    pub cell: Cell<A>,
    pub init_value_op: Arc<Mutex<Option<A>>>
}

impl<A:Send+Clone+'static> CellLoop<A> {

    pub fn new(sodium_ctx: &SodiumCtx) -> CellLoop<A> {
        let init_value_op: Arc<Mutex<Option<A>>> = Arc::new(Mutex::new(None));
        let init_value: Lazy<A>;
        {
            let init_value_op = init_value_op.clone();
            init_value = Lazy::new(move || {
                let mut l = init_value_op.lock();
                let init_value_op: &mut Option<A> = l.as_mut().unwrap();
                let mut result_op: Option<A> = None;
                mem::swap(&mut result_op, init_value_op);
                if let Some(init_value) = result_op {
                    return init_value.clone();
                }
                panic!("CellLoop sampled before looped.");
            });
        }
        let stream_loop = StreamLoop::new(sodium_ctx);
        let stream = stream_loop.stream();
        let cell_loop_data =
            Arc::new(Mutex::new(CellLoopData {
                stream_loop: stream_loop,
                cell: stream.hold_lazy(init_value),
                init_value_op
            }));
        let gc_node_trace;
        {
            let cell_loop_data = cell_loop_data.clone();
            gc_node_trace = move |tracer: &mut Tracer| {
                let l = cell_loop_data.lock();
                let cell_loop_data = l.as_ref().unwrap();
                tracer(&cell_loop_data.stream_loop.gc_node);
                tracer(&cell_loop_data.cell.gc_node);
            };
        }
        CellLoop {
            data: cell_loop_data,
            gc_node: GcNode::new(&sodium_ctx.gc_ctx(), "CellLoop::new", || {}, gc_node_trace)
        }
    }

    pub fn cell(&self) -> Cell<A> {
        self.with_data(|data: &mut CellLoopData<A>| data.cell.clone())
    }

    pub fn loop_(&self, ca: &Cell<A>) {
        self.with_data(|data: &mut CellLoopData<A>| {
            data.stream_loop.loop_(&ca.updates());
            let mut l = data.init_value_op.lock();
            let init_value_op: &mut Option<A> = l.as_mut().unwrap();
            *init_value_op = Some(ca.sample());
        });
    }

    pub fn with_data<R,K:FnOnce(&mut CellLoopData<A>)->R>(&self, k: K) -> R {
        let mut l = self.data.lock();
        let data: &mut CellLoopData<A> = l.as_mut().unwrap();
        k(data)
    }
}
