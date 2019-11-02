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
            sodium_ctx.with_data(|data: &mut SodiumCtxData| {
                let dependents: Vec<Node> = self.stream.with_data(|data: &mut StreamData<A>| {
                    data.node.with_data(|data: &mut NodeData| {
                        data.dependents
                            .iter()
                            .flat_map(|node: &WeakNode| {
                                node.upgrade()
                            })
                            .collect()
                    })
                });
                let node: Node;
                {
                    let l = self.stream.data.lock();
                    let n: &StreamData<A> = l.as_ref().unwrap();
                    node = n.node.clone();
                    node.with_data(|data2: &mut NodeData| { data2.changed = true });
                }
                for dependent in dependents {
                    data.changed_nodes.push(dependent);
                }
            });
        });
    }
}
