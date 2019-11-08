use crate::impl_::stream::Stream;
use crate::impl_::sodium_ctx::SodiumCtx;

#[derive(Clone)]
pub struct StreamSink<A> {
    stream: Stream<A>,
    sodium_ctx: SodiumCtx
}

impl<A:Send+'static> StreamSink<A> {
    pub fn new(sodium_ctx: &SodiumCtx) -> StreamSink<A> {
        StreamSink {
            stream: Stream::new(sodium_ctx),
            sodium_ctx: sodium_ctx.clone()
        }
    }

    pub fn to_stream(&self) -> Stream<A> {
        self.stream.clone()
    }

    pub fn send(&self, a: A) {
        self.sodium_ctx.transaction(|| {
            self.sodium_ctx.add_dependents_to_changed_nodes(self.stream.node());
            self.stream._send(&self.sodium_ctx, a);
        });
    }
}