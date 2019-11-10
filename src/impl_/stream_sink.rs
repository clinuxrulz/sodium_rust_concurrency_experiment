use crate::impl_::stream::Stream;
use crate::impl_::sodium_ctx::SodiumCtx;

pub struct StreamSink<A> {
    stream: Stream<A>,
    sodium_ctx: SodiumCtx
}

impl<A> Clone for StreamSink<A> {
    fn clone(&self) -> Self {
        StreamSink {
            stream: self.stream.clone(),
            sodium_ctx: self.sodium_ctx.clone()
        }
    }
}

impl<A:Send+'static> StreamSink<A> {
    pub fn new(sodium_ctx: &SodiumCtx) -> StreamSink<A> {
        StreamSink {
            stream: Stream::new(sodium_ctx),
            sodium_ctx: sodium_ctx.clone()
        }
    }

    pub fn new_with_coalescer<COALESCER:FnMut(&A,&A)->A+Send+'static>(sodium_ctx: &SodiumCtx, coalescer: COALESCER) -> StreamSink<A> {
        StreamSink {
            stream: Stream::_new_with_coalescer(sodium_ctx, coalescer),
            sodium_ctx: sodium_ctx.clone()
        }
    }

    pub fn stream(&self) -> Stream<A> {
        self.stream.clone()
    }

    pub fn send(&self, a: A) {
        self.sodium_ctx.transaction(|| {
            self.sodium_ctx.add_dependents_to_changed_nodes(self.stream.node());
            self.stream._send(a);
        });
    }
}
