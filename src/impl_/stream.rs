use crate::impl_::cell::Cell;
use crate::impl_::node::Node;
use crate::impl_::node::NodeData;
use crate::impl_::lazy::Lazy;
use crate::impl_::listener::Listener;
use crate::impl_::sodium_ctx::SodiumCtx;
use crate::impl_::lambda::IsLambda1;
use crate::impl_::lambda::IsLambda2;

use std::mem;
use std::sync::Arc;
use std::sync::Mutex;

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
    pub node: Node,
    pub sodium_ctx: SodiumCtx,
    pub coalescer_op: Option<Box<dyn FnMut(&A,&A)->A+Send>>
}

impl<A:Send+'static> Stream<A> {
    pub fn new(sodium_ctx: &SodiumCtx) -> Stream<A> {
        Stream {
            data: Arc::new(Mutex::new(StreamData {
                firing_op: None,
                node: sodium_ctx.null_node(),
                sodium_ctx: sodium_ctx.clone(),
                coalescer_op: None
            })),
        }
    }

    pub fn _new_with_coalescer<COALESCER:FnMut(&A,&A)->A+Send+'static>(sodium_ctx: &SodiumCtx, coalescer: COALESCER) -> Stream<A> {
        Stream {
            data: Arc::new(Mutex::new(StreamData {
                firing_op: None,
                node: sodium_ctx.null_node(),
                sodium_ctx: sodium_ctx.clone(),
                coalescer_op: Some(Box::new(coalescer))
            })),
        }
    }

    pub fn _new<MkNode:FnOnce(&Stream<A>)->Node>(sodium_ctx: &SodiumCtx, mk_node: MkNode) -> Stream<A> {
        let s = Stream {
            data: Arc::new(Mutex::new(StreamData {
                firing_op: None,
                node: sodium_ctx.null_node(),
                sodium_ctx: sodium_ctx.clone(),
                coalescer_op: None
            }))
        };
        let node = mk_node(&s);
        s.with_data(|data: &mut StreamData<A>| data.node = node.clone());
        let mut update: Box<dyn FnMut()+Send> = Box::new(|| {});
        node.with_data(|data: &mut NodeData| {
            mem::swap(&mut update, &mut data.update);
        });
        update();
        node.with_data(|data: &mut NodeData| {
            mem::swap(&mut update, &mut data.update);
        });
        let is_firing =
            s.with_data(|data: &mut StreamData<A>| {
                if data.firing_op.is_some() {
                    data.node.with_data(|data: &mut NodeData| data.changed = true);
                    true
                } else {
                    false
                }
            });
        if is_firing {
            let s = s.clone();
            sodium_ctx.post(move || {
                s.with_data(|data: &mut StreamData<A>| {
                    data.firing_op = None;
                    data.node.with_data(|data: &mut NodeData| data.changed = false)
                })
            });
        }
        s
    }

    pub fn with_firing_op<R,K:FnOnce(&mut Option<A>)->R>(&self, k: K) -> R {
        self.with_data(|data: &mut StreamData<A>| k(&mut data.firing_op))
    }

    pub fn node(&self) -> Node {
        self.with_data(|data: &mut StreamData<A>| data.node.clone())
    }

    pub fn sodium_ctx(&self) -> SodiumCtx {
        self.with_data(|data: &mut StreamData<A>| data.sodium_ctx.clone())
    }

    pub fn snapshot<B:Send+Clone+'static,C:Send+'static,FN:IsLambda2<A,B,C>+Send+'static>(&self, cb: &Cell<B>, mut f: FN) -> Stream<C> {
        let cb = cb.clone();
        self.map(move |a: &A| f.call(a, &cb.sample()))
    }

    pub fn snapshot1<B:Send+Clone+'static>(&self, cb: &Cell<B>) -> Stream<B> {
        self.snapshot(cb, |_a: &A, b: &B| b.clone())
    }

    pub fn map<B:Send+'static,FN:IsLambda1<A,B>+Send+'static>(&self, mut f: FN) -> Stream<B> {
        let _self = self.clone();
        let sodium_ctx = self.sodium_ctx().clone();
        Stream::_new(
            &sodium_ctx,
            |s: &Stream<B>| {
                let _s = s.clone();
                let sodium_ctx = sodium_ctx.clone();
                Node::new(
                    move || {
                        _self.with_firing_op(|firing_op: &mut Option<A>| {
                            if let Some(ref firing) = firing_op {
                                _s._send(&sodium_ctx, f.call(firing));
                            }
                        })
                    },
                    vec![self.node()]
                )
            }
        )
    }

    pub fn filter<PRED:IsLambda1<A,bool>+Send+'static>(&self, mut pred: PRED) -> Stream<A> where A: Clone {
        let _self = self.clone();
        let sodium_ctx = self.sodium_ctx().clone();
        Stream::_new(
            &sodium_ctx,
            |s: &Stream<A>| {
                let _s = s.clone();
                let sodium_ctx = sodium_ctx.clone();
                Node::new(
                    move || {
                        _self.with_firing_op(|firing_op: &mut Option<A>| {
                            let firing_op2 = firing_op.clone().filter(|firing| pred.call(firing));
                            if let Some(firing) = firing_op2 {
                                _s._send(&sodium_ctx, firing);
                            }
                        });
                    },
                    vec![self.node()]
                )
            }
        )
    }

    pub fn or_else(&self, s2: &Stream<A>) -> Stream<A> where A: Clone {
        self.merge(s2, |lhs:&A, _rhs:&A| lhs.clone())
    }

    pub fn merge<FN:IsLambda2<A,A,A>+Send+'static>(&self, s2: &Stream<A>, mut f: FN) -> Stream<A> where A: Clone {
        let _self = self.clone();
        let s2 = s2.clone();
        let sodium_ctx = self.sodium_ctx().clone();
        Stream::_new(
            &sodium_ctx,
            |s: &Stream<A>| {
                let _s = s.clone();
                let _s2 = s2.clone();
                let sodium_ctx = sodium_ctx.clone();
                Node::new(
                    move || {
                        _self.with_firing_op(|firing1_op: &mut Option<A>| {
                            _s2.with_firing_op(|firing2_op: &mut Option<A>| {
                                if let Some(ref firing1) = firing1_op {
                                    if let Some(ref firing2) = firing2_op {
                                        _s._send(&sodium_ctx, f.call(firing1, firing2));
                                    } else {
                                        _s._send(&sodium_ctx, firing1.clone());
                                    }
                                } else {
                                    if let Some(ref firing2) = firing2_op {
                                        _s._send(&sodium_ctx, firing2.clone());
                                    }
                                }
                            })
                        })
                    },
                    vec![self.node(), s2.node()]
                )
            }
        )
    }

    pub fn hold(&self, a: A) -> Cell<A> where A: Clone {
        let sodium_ctx = self.sodium_ctx();
        sodium_ctx.transaction(|| {
            Cell::_new(&sodium_ctx, self.clone(), Lazy::of_value(a))
        })
    }

    pub fn hold_lazy(&self, a: Lazy<A>) -> Cell<A> where A: Clone {
        let sodium_ctx = self.sodium_ctx();
        sodium_ctx.transaction(|| {
            Cell::_new(&sodium_ctx, self.clone(), a)
        })
    }

    pub fn _listen<K:IsLambda1<A,()>+Send+'static>(&self, mut k: K, weak: bool) -> Listener {
        let self_ = self.clone();
        let node =
            Node::new(
                move || {
                    self_.with_data(|data: &mut StreamData<A>| {
                        for firing in &data.firing_op {
                            k.call(firing)
                        }
                    });
                },
                vec![self.node()]
            );
        Listener::new(&self.sodium_ctx(), weak, node)
    }

    pub fn listen_weak<K:IsLambda1<A,()>+Send+'static>(&self, k: K) -> Listener {
        self._listen(k, true)
    }

    pub fn listen<K:IsLambda1<A,()>+Send+'static>(&self, k: K) -> Listener {
        self._listen(k, false)
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
                if let Some(ref mut coalescer) = data.coalescer_op {
                    if let Some(ref mut firing) = data.firing_op {
                        *firing = coalescer(firing, &a);
                    }
                    if is_first {
                        data.firing_op = Some(a);
                    }
                } else {
                    data.firing_op = Some(a);
                }
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
