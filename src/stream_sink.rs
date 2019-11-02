use crate::node::Node;
use crate::node::NodeData;
use crate::node::WeakNode;
use crate::stream::Stream;
use crate::stream::StreamData;
use crate::sodium_ctx::SodiumCtx;
use crate::sodium_ctx::SodiumCtxData;

#[derive(Clone)]
pub struct StreamSink<A> {
    stream: Stream<A>
}

impl<A:Send+'static> StreamSink<A> {
    pub fn new(sodium_ctx: &SodiumCtx) -> StreamSink<A> {
        StreamSink {
            stream: Stream::new(sodium_ctx)
        }
    }

    pub fn to_stream(&self) -> Stream<A> {
        self.stream.clone()
    }

    pub fn send(&self, sodium_ctx: &SodiumCtx, a: A) {
        sodium_ctx.transaction(|| {
            sodium_ctx.add_dependents_to_changed_nodes(self.stream.node());
            self.stream._send(sodium_ctx, a);
        });
    }
}
