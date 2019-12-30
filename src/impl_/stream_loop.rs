use crate::impl_::node::NodeData;
use crate::impl_::sodium_ctx::SodiumCtx;
use crate::impl_::stream::Stream;

use std::sync::Arc;
use std::sync::Mutex;

pub struct StreamLoop<A> {
    pub data: Arc<Mutex<StreamLoopData<A>>>
}

pub struct StreamLoopData<A> {
    pub stream: Stream<A>,
    pub looped: bool
}

impl<A> Clone for StreamLoop<A> {
    fn clone(&self) -> Self {
        StreamLoop { data: self.data.clone() }
    }
}

impl<A:Clone+Send+'static> StreamLoop<A> {

    pub fn new(sodium_ctx: &SodiumCtx) -> StreamLoop<A> {
        StreamLoop {
            data: Arc::new(Mutex::new(StreamLoopData {
                stream: Stream::new(sodium_ctx),
                looped: false
            }))
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
            let s = Stream::downgrade(&s);
            node.add_update_dependencies(vec![node.clone()]);
            let s_out = data.stream.clone();
            node.with_data(|data: &mut NodeData| {
                data.update = Box::new(move || {
                    let s = s.upgrade().unwrap();
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
