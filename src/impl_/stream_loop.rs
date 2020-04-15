use crate::impl_::gc_node::{GcNode, Tracer};
use crate::impl_::node::NodeData;
use crate::impl_::sodium_ctx::SodiumCtx;
use crate::impl_::stream::Stream;

use std::sync::Arc;
use std::sync::Mutex;

pub struct StreamLoop<A> {
    pub data: Arc<Mutex<StreamLoopData<A>>>,
    pub gc_node: GcNode
}

pub struct StreamLoopData<A> {
    pub stream: Stream<A>,
    pub looped: bool
}

impl<A> Clone for StreamLoop<A> {
    fn clone(&self) -> Self {
        self.gc_node.inc_ref();
        StreamLoop {
            data: self.data.clone(),
            gc_node: self.gc_node.clone()
        }
    }
}

impl<A> Drop for StreamLoop<A> {
    fn drop(&mut self) {
        self.gc_node.dec_ref();
    }
}

impl<A:Clone+Send+'static> StreamLoop<A> {

    pub fn new(sodium_ctx: &SodiumCtx) -> StreamLoop<A> {
        let stream_loop_data = Arc::new(Mutex::new(StreamLoopData {
            stream: Stream::new(sodium_ctx),
            looped: false
        }));
        let gc_node_trace;
        {
            let stream_loop_data = stream_loop_data.clone();
            gc_node_trace = move |tracer: &mut Tracer| {
                let l = stream_loop_data.lock();
                let stream_loop_data = l.as_ref().unwrap();
                tracer(&stream_loop_data.stream.gc_node);
            };
        }
        StreamLoop {
            data: stream_loop_data,
            gc_node: GcNode::new(&sodium_ctx.gc_ctx(), "StreamLoop::new", || {}, gc_node_trace)
        }
    }

    pub fn stream(&self) -> Stream<A> {
        self.with_data(|data: &mut StreamLoopData<A>| data.stream.clone())
    }

    pub fn loop_(&self, s: &Stream<A>) {
        self.with_data(|data: &mut StreamLoopData<A>| {
            if data.looped {
                panic!("StreamLoop already looped.");
            }
            data.looped = true;
            let node = data.stream.node();
            node.add_dependency(s.node());
            let s = s.clone();
            let s_out = data.stream.clone();
            node.add_update_dependencies(vec![s.gc_node.clone(), s_out.gc_node.clone()]);
            node.with_data(|data: &mut NodeData| {
                data.update = Box::new(move || {
                    s.with_firing_op(|firing_op: &mut Option<A>| {
                        if let Some(ref firing) = firing_op {
                            s_out._send(firing.clone());
                        }
                    });
                });
            });
        })
    }

    pub fn with_data<R,K:FnOnce(&mut StreamLoopData<A>)->R>(&self, k: K) -> R {
        let mut l = self.data.lock();
        let data: &mut StreamLoopData<A> = l.as_mut().unwrap();
        k(data)
    }
}
