use crate::impl_::stream_loop::StreamLoop;
use crate::impl_::cell::Cell;
use crate::impl_::lazy::Lazy;
use crate::impl_::sodium_ctx::SodiumCtx;

use std::cell::RefCell;
use std::rc::Rc;
use std::mem;
use bacon_rajan_cc::{Cc, Trace, Tracer};

pub struct CellLoop<A:'static> {
    pub data: Cc<RefCell<CellLoopData<A>>>
}

impl<A> Clone for CellLoop<A> {
    fn clone(&self) -> Self {
        CellLoop {
            data: self.data.clone()
        }
    }
}

pub struct CellLoopData<A:'static> {
    pub stream_loop: StreamLoop<A>,
    pub cell: Cell<A>,
    pub init_value_op: Rc<RefCell<Option<A>>>
}

impl<A> Trace for CellLoopData<A> {
    fn trace(&self, tracer: &mut Tracer) {
        self.stream_loop.trace(tracer);
        self.cell.trace(tracer);
    }
}

impl<A:Clone+'static> CellLoop<A> {

    pub fn new(sodium_ctx: &SodiumCtx) -> CellLoop<A> {
        let init_value_op: Rc<RefCell<Option<A>>> = Rc::new(RefCell::new(None));
        let init_value: Lazy<A>;
        {
            let init_value_op = init_value_op.clone();
            init_value = Lazy::new(move || {
                let mut l = init_value_op.borrow_mut();
                let init_value_op: &mut Option<A> = &mut l;
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
        CellLoop {
            data: Cc::new(RefCell::new(CellLoopData {
                stream_loop: stream_loop,
                cell: stream.hold_lazy(init_value),
                init_value_op
            }))
        }
    }

    pub fn cell(&self) -> Cell<A> {
        self.with_data(|data: &mut CellLoopData<A>| data.cell.clone())
    }

    pub fn loop_(&self, ca: &Cell<A>) {
        self.with_data(|data: &mut CellLoopData<A>| {
            data.stream_loop.loop_(&ca.updates());
            let mut l = data.init_value_op.borrow_mut();
            let init_value_op: &mut Option<A> = &mut l;
            *init_value_op = Some(ca.sample());
        });
    }

    pub fn with_data<R,K:FnOnce(&mut CellLoopData<A>)->R>(&self, k: K) -> R {
        let mut l = self.data.borrow_mut();
        let data: &mut CellLoopData<A> = &mut l;
        k(data)
    }
}
