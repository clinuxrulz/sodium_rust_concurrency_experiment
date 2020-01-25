use crate::impl_::stream_sink::StreamSink as StreamSinkImpl;
use crate::sodium_ctx::SodiumCtx;
use crate::stream::Stream;

pub struct StreamSink<A:'static> {
    pub impl_: StreamSinkImpl<A>
}

impl<A> Clone for StreamSink<A> {
    fn clone(&self) -> Self {
        StreamSink {
            impl_: self.impl_.clone()
        }
    }
}

impl<A:Clone+'static> StreamSink<A> {
    pub fn new(sodium_ctx: &SodiumCtx) -> StreamSink<A> {
        StreamSink { impl_: StreamSinkImpl::new(&sodium_ctx.impl_) }
    }

    pub fn new_with_coalescer<COALESCER:FnMut(&A,&A)->A+'static>(sodium_ctx: &SodiumCtx, coalescer: COALESCER) -> StreamSink<A> {
        StreamSink { impl_: StreamSinkImpl::new_with_coalescer(&sodium_ctx.impl_, coalescer) }
    }

    pub fn stream(&self) -> Stream<A> {
        Stream { impl_: self.impl_.stream() }
    }

    pub fn send(&self, a: A) {
        self.impl_.send(a);
    }
}
