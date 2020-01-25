use crate::Cell;
use crate::CellSink;
use crate::CellLoop;
use crate::StreamSink;
use crate::StreamLoop;
use crate::impl_::sodium_ctx::SodiumCtx as SodiumCtxImpl;

pub struct SodiumCtx {
    pub impl_: SodiumCtxImpl
}

impl SodiumCtx {
    pub fn new() -> SodiumCtx {
        SodiumCtx { impl_: SodiumCtxImpl::new() }
    }

    pub fn new_cell<A:Clone+'static>(&self, a: A) -> Cell<A> {
        Cell::new(self, a)
    }

    pub fn new_cell_sink<A:Clone+'static>(&self, a: A) -> CellSink<A> {
        CellSink::new(self, a)
    }

    pub fn new_stream_sink<A:Clone+'static>(&self) -> StreamSink<A> {
        StreamSink::new(self)
    }

    pub fn new_cell_loop<A:Clone+'static>(&self) -> CellLoop<A> {
        CellLoop::new(self)
    }

    pub fn new_stream_loop<A:Clone+'static>(&self) -> StreamLoop<A> {
        StreamLoop::new(self)
    }

    pub fn new_stream_sink_with_coalescer<A:Clone+'static,COALESCER:FnMut(&A,&A)->A+'static>(&self, coalescer: COALESCER) -> StreamSink<A> {
        StreamSink::new_with_coalescer(self, coalescer)
    }

    pub fn transaction<R,K:FnOnce()->R>(&self, k: K) -> R {
        self.impl_.transaction(k)
    }
}
