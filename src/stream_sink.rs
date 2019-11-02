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
    pub fn new() -> StreamSink<A> {
        StreamSink {
            stream: Stream::new()
        }
    }

    pub fn to_stream(&self) -> Stream<A> {
        self.stream.clone()
    }

    pub fn send(&self, sodium_ctx: &SodiumCtx, a: A) {
        sodium_ctx.transaction(|| {
            sodium_ctx.add_dependents_to_changed_node(self.stream.node());
            self.stream.with_data(|data: &mut StreamData<A>| {
                data.firing_op = Some(a);
                data.node.with_data(|data: &mut NodeData| {
                    data.changed = true;
                });
            });
            {
                let stream = self.stream.clone();
                sodium_ctx.post(move || {
                    stream.with_data(|data: &mut StreamData<A>| {
                        data.firing_op = None;
                        data.node.with_data(|data: &mut NodeData| {
                            data.changed = false;
                        });
                    });
                });
            }
        });
    }
}
