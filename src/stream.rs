use std::sync::Arc;
use std::sync::Mutex;
use crate::node::Node;
use crate::node::NodeData;
use crate::listener::Listener;
use crate::sodium_ctx::SodiumCtx;

pub struct Stream<A> {
    pub data: Arc<Mutex<StreamData<A>>>
}

impl<A> Clone for Stream<A> {
    fn clone(&self) -> Self {
        Stream {
            data: self.data.clone()
        }
    }
}

pub struct StreamData<A> {
    pub firing_op: Option<A>,
    pub node: Node
}

impl<A:Send+'static> Stream<A> {
    pub fn new() -> Stream<A> {
        Stream {
            data: Arc::new(Mutex::new(StreamData {
                firing_op: None,
                node: Node::new(|_: &SodiumCtx| {}, Vec::new())
            }))
        }
    }

    pub fn with_firing_op<R,K:FnOnce(&mut Option<A>)->R>(&self, k: K) -> R {
        self.with_data(|data: &mut StreamData<A>| k(&mut data.firing_op))
    }

    pub fn node(&self) -> Node {
        self.with_data(|data: &mut StreamData<A>| data.node.clone())
    }

    pub fn map<B:Send+'static,FN:FnMut(&A)->B+Send+'static>(&self, mut f: FN) -> Stream<B> {
        let s = Stream {
            data: Arc::new(Mutex::new(StreamData {
                firing_op: self.with_firing_op(|firing_op: &mut Option<A>| {
                    if let Some(ref firing) = firing_op {
                        return Some(f(firing));
                    } else {
                        return None;
                    }
                }),
                node: Node::new(|_:&SodiumCtx| {}, vec![self.node()])
            }))
        };
        s.node().with_data(|data: &mut NodeData| {
            let _self = self.clone();
            let _s = s.clone();
            data.update = Box::new(move |sodium_ctx: &SodiumCtx| {
                _self.with_firing_op(|firing_op: &mut Option<A>| {
                    if let Some(ref firing) = firing_op {
                        _s._send(sodium_ctx, f(firing));
                    }
                })
            });
        });
        s
    }

    pub fn filter<PRED:FnMut(&A)->bool+Send+'static>(&self, mut pred: PRED) -> Stream<A> where A: Clone {
        let s = Stream {
            data: Arc::new(Mutex::new(StreamData {
                firing_op: self.with_firing_op(|firing_op: &mut Option<A>| firing_op.clone().filter(|firing| pred(firing))),
                node: Node::new(|_:&SodiumCtx| {}, vec![self.node()])
            }))
        };
        s.node().with_data(|data: &mut NodeData| {
            let _self = self.clone();
            let _s = s.clone();
            data.update = Box::new(move |sodium_ctx: &SodiumCtx| {
                _self.with_firing_op(|firing_op: &mut Option<A>| {
                    let firing_op2 = firing_op.clone().filter(|firing| pred(firing));
                    if let Some(firing) = firing_op2 {
                        _s._send(sodium_ctx, firing);
                    }
                })
            });
        });
        s
    }

    pub fn listen<K: FnMut(&A)+Send+'static>(&self, mut k: K) -> Listener {
        let self_ = self.clone();
        let node =
            Node::new(
                move |_: &SodiumCtx| {
                    self_.with_data(|data: &mut StreamData<A>| {
                        for firing in &data.firing_op {
                            k(firing)
                        }
                    });
                },
                vec![self.node()]
            );
        Listener::new(node)
    }

    pub fn with_data<R,K:FnOnce(&mut StreamData<A>)->R>(&self, k: K) -> R {
        let mut l = self.data.lock();
        let data: &mut StreamData<A> = l.as_mut().unwrap();
        k(data)
    }

    pub fn _send(&self, sodium_ctx: &SodiumCtx, a: A) {
        sodium_ctx.transaction(|| {
            let is_first = self.with_data(|data: &mut StreamData<A>| {
                let is_first = data.firing_op.is_none();
                data.firing_op = Some(a);
                data.node.with_data(|data: &mut NodeData| {
                    data.changed = true;
                });
                is_first
            });
            if is_first {
                let _self = self.clone();
                sodium_ctx.post(move || {
                    _self.with_data(|data: &mut StreamData<A>| {
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
