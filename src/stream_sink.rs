use std::sync::Arc;
use std::sync::Mutex;
use crate::node::Node;
use crate::node::NodeData;
use crate::stream::Stream;
use crate::stream::StreamData;
use crate::sodium_ctx::SodiumCtx;
use crate::sodium_ctx::SodiumCtxData;

#[derive(Clone)]
pub struct StreamSink<A> {
    stream: Stream<A>
}

impl<A> StreamSink<A> {
    pub fn new() -> StreamSink<A> {
        StreamSink {
            stream: Stream::new()
        }
    }

    pub fn send(&self, sodium_ctx: &SodiumCtx, a: A) {
        sodium_ctx.transaction(|| {
            sodium_ctx.with_data(|data: &mut SodiumCtxData| {
                let node: Node;
                {
                    let l = self.stream.data.lock();
                    let n: &StreamData<A> = l.as_ref().unwrap();
                    node = n.node.clone();
                    node.with_data(|data2: &mut NodeData| { data2.changed = true });
                }
                data.changed_nodes.push(node);
            });
        });
    }
}
