use crate::impl_::node::NodeData;
use crate::impl_::sodium_ctx::SodiumCtx;
use crate::impl_::stream::Stream;

use std::cell::RefCell;
use bacon_rajan_cc::{Cc, Trace, Tracer};

pub struct StreamLoop<A:'static> {
    pub data: Cc<RefCell<StreamLoopData<A>>>
}

impl<A> Trace for StreamLoop<A> {
    fn trace(&mut self, tracer: &mut Tracer) {
        tracer(&self.data);
    }
}

pub struct StreamLoopData<A:'static> {
    pub stream: Stream<A>,
    pub looped: bool
}

impl<A> Trace for StreamLoopData<A> {
    fn trace(&mut self, tracer: &mut Tracer) {
        self.stream.trace(tracer);
    }
}

impl<A> Clone for StreamLoop<A> {
    fn clone(&self) -> Self {
        StreamLoop { data: self.data.clone() }
    }
}

impl<A:Clone+'static> StreamLoop<A> {

    pub fn new(sodium_ctx: &SodiumCtx) -> StreamLoop<A> {
        StreamLoop {
            data: Cc::new(RefCell::new(StreamLoopData {
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
        let mut l = self.data.borrow_mut();
        let data: &mut StreamLoopData<A> = &mut l;
        k(data)
    }
}
