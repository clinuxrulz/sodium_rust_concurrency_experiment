use crate::impl_::dep::Dep;
use crate::impl_::gc::{Finalize, Gc, GcCell, Trace, Tracer};
use crate::impl_::node::NodeData;
use crate::impl_::sodium_ctx::SodiumCtx;
use crate::impl_::stream::Stream;

pub struct StreamLoop<A:'static> {
    pub data: Gc<GcCell<StreamLoopData<A>>>
}

impl<A> Trace for StreamLoop<A> {
    fn trace(&self, tracer: &mut Tracer) {
        self.data.trace(tracer);
    }
}

impl<A> Finalize for StreamLoop<A> {
    fn finalize(&mut self) {}
}

pub struct StreamLoopData<A:'static> {
    pub stream: Stream<A>,
    pub looped: bool
}

impl<A> Trace for StreamLoopData<A> {
    fn trace(&self, tracer: &mut Tracer) {
        self.stream.trace(tracer);
    }
}

impl<A> Finalize for StreamLoopData<A> {
    fn finalize(&mut self) {}
}

impl<A> Clone for StreamLoop<A> {
    fn clone(&self) -> Self {
        StreamLoop { data: self.data.clone() }
    }
}

impl<A:Clone+'static> StreamLoop<A> {

    pub fn new(sodium_ctx: &SodiumCtx) -> StreamLoop<A> {
        StreamLoop {
            data: sodium_ctx.gc_ctx().new_gc(GcCell::new(StreamLoopData {
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
            let s_ = Stream::downgrade(&s);
            let s_out = Stream::downgrade(&data.stream);
            node.add_keep_alive(Dep::new(s.clone()));
            node.add_keep_alive(Dep::new(data.stream.clone()));
            node.with_data(|data: &mut NodeData| {
                data.update = Box::new(move || {
                    let s_ = s_.upgrade().unwrap();
                    let s_out = s_out.upgrade().unwrap();
                    s_.with_firing_op(|firing_op: &mut Option<A>| {
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
