use crate::impl_::stream_sink::StreamSink as StreamSinkImpl;
use crate::sodium_ctx::SodiumCtx;
use crate::stream::Stream;

pub struct StreamSink<A> {
    pub impl_: StreamSinkImpl<A>
}

impl<A:Clone+Send+'static> StreamSink<A> {
    pub fn new(sodium_ctx: &SodiumCtx) -> StreamSink<A> {
        StreamSink { impl_: StreamSinkImpl::new(&sodium_ctx.impl_) }
    }

    pub fn to_stream(&self) -> Stream<A> {
        Stream { impl_: self.impl_.to_stream() }
    }

    pub fn send(&self, a: A) {
        self.impl_.send(a);
    }
}
