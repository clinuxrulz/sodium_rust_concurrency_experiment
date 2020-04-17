use crate::impl_::cell::Cell;
use crate::impl_::gc_node::{GcNode, Tracer};
use crate::impl_::node::{Node, NodeData};
use crate::impl_::lazy::Lazy;
use crate::impl_::listener::Listener;
use crate::impl_::sodium_ctx::SodiumCtx;
use crate::impl_::stream_loop::StreamLoop;
use crate::impl_::stream_sink::StreamSink;
use crate::impl_::lambda::IsLambda1;
use crate::impl_::lambda::IsLambda2;
use crate::impl_::lambda::{lambda1, lambda1_deps, lambda2_deps};

use std::mem;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::Weak;

pub struct Stream<A> {
    pub data: Arc<Mutex<StreamData<A>>>,
    pub gc_node: GcNode
}

pub struct WeakStream<A> {
    pub data: Weak<Mutex<StreamData<A>>>,
    pub gc_node: GcNode
}

impl<A> Clone for Stream<A> {
    fn clone(&self) -> Self {
        self.gc_node.inc_ref();
        Stream {
            data: self.data.clone(),
            gc_node: self.gc_node.clone()
        }
    }
}

impl<A> Drop for Stream<A> {
    fn drop(&mut self) {
        self.gc_node.dec_ref();
    }
}

impl<A> Clone for WeakStream<A> {
    fn clone(&self) -> Self {
        WeakStream {
            data: self.data.clone(),
            gc_node: self.gc_node.clone()
        }
    }
}

pub struct StreamData<A> {
    pub node: Node,
    pub firing_op: Option<A>,
    pub sodium_ctx: SodiumCtx,
    pub coalescer_op: Option<Box<dyn FnMut(&A,&A)->A+Send>>
}

impl<A> Stream<A> {
    pub fn with_data<R,K:FnOnce(&mut StreamData<A>)->R>(&self, k: K) -> R {
        let mut l = self.data.lock();
        let data: &mut StreamData<A> = l.as_mut().unwrap();
        k(data)
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

}

impl<A:Send+'static> Stream<A> {
    pub fn new(sodium_ctx: &SodiumCtx) -> Stream<A> {
        Stream::_new(
            sodium_ctx,
            "Stream::new",
            |_s: &Stream<A>| {
                Node::new(
                    sodium_ctx,
                    "Stream::new node",
                    || {},
                    Vec::new()
                )
            }
        )
    }

    // for purpose of capturing stream in lambda
    pub fn nop(&self) {}

    pub fn _new_with_coalescer<COALESCER:FnMut(&A,&A)->A+Send+'static>(sodium_ctx: &SodiumCtx, coalescer: COALESCER) -> Stream<A> {
        let result_forward_ref: Arc<Mutex<Option<Arc<Mutex<StreamData<A>>>>>> = Arc::new(Mutex::new(None));
        let result;
        {
            let result_forward_ref = result_forward_ref.clone();
            result = Stream {
                data: Arc::new(Mutex::new(StreamData {
                    node: sodium_ctx.null_node(),
                    firing_op: None,
                    sodium_ctx: sodium_ctx.clone(),
                    coalescer_op: Some(Box::new(coalescer))
                })),
                gc_node: GcNode::new(
                    &sodium_ctx.gc_ctx(),
                    "Stream::_new_with_coalescer",
                    || {},
                    move |tracer: &mut Tracer| {
                        let l = result_forward_ref.lock();
                        let s = l.as_ref().unwrap();
                        let s: &Option<Arc<Mutex<StreamData<A>>>> = &s;
                        let s: Arc<Mutex<StreamData<A>>> = s.clone().unwrap();
                        let l = s.lock();
                        let s = l.as_ref().unwrap();
                        tracer(&s.node.gc_node);
                    }
                )
            };
        }
        {
            let mut l = result_forward_ref.lock();
            let mut result_forward_ref = l.as_mut().unwrap();
            let result_forward_ref: &mut Option<Arc<Mutex<StreamData<A>>>> = &mut result_forward_ref;
            *result_forward_ref = Some(result.data.clone());
        }
        result
    }

    pub fn _new<NAME:ToString,MkNode:FnOnce(&Stream<A>)->Node>(sodium_ctx: &SodiumCtx, name: NAME, mk_node: MkNode) -> Stream<A> {
        let result_forward_ref: Arc<Mutex<Option<Arc<Mutex<StreamData<A>>>>>> = Arc::new(Mutex::new(None));
        let s;
        {
            let result_forward_ref = result_forward_ref.clone();
            s = Stream {
                data: Arc::new(Mutex::new(StreamData {
                    node: sodium_ctx.null_node(),
                    firing_op: None,
                    sodium_ctx: sodium_ctx.clone(),
                    coalescer_op: None
                })),
                gc_node: GcNode::new(
                    &sodium_ctx.gc_ctx(),
                    name.to_string(),
                    || {},
                    move |tracer: &mut Tracer| {
                        let gc_node;
                        {
                            let l = result_forward_ref.lock();
                            let s = l.as_ref().unwrap();
                            let s: &Option<Arc<Mutex<StreamData<A>>>> = &s;
                            let s: Arc<Mutex<StreamData<A>>> = s.clone().unwrap();
                            let l = s.lock();
                            let s = l.as_ref().unwrap();
                            gc_node = s.node.gc_node.clone();
                        }
                        tracer(&gc_node);
                    }
                )
            };
        }
        let node = mk_node(&s);
        node.add_keep_alive(&s.gc_node);
        s.with_data(|data: &mut StreamData<A>| data.node = node.clone());
        {
            let mut l = result_forward_ref.lock();
            let mut result_forward_ref = l.as_mut().unwrap();
            let result_forward_ref: &mut Option<Arc<Mutex<StreamData<A>>>> = &mut result_forward_ref;
            *result_forward_ref = Some(s.data.clone());
        }
        {
            let mut update = node.data.update.write().unwrap();
            let update: &mut Box<_> = &mut update;
            update();
        }
        let is_firing =
            s.with_data(|data: &mut StreamData<A>| data.firing_op.is_some());
        if is_firing {
            {
                let s_node = s.node();
                let mut changed = s_node.data.changed.write().unwrap();
                *changed = true;
            }
            let s = s.clone();
            sodium_ctx.pre_post(move || {
                s.with_data(|data: &mut StreamData<A>| {
                    data.firing_op = None;
                    let s_node = s.node();
                    let mut changed = s_node.data.changed.write().unwrap();
                    *changed = true;
                });
            });
        }
        s
    }

    pub fn snapshot<B:Send+Clone+'static,C:Send+'static,FN:IsLambda2<A,B,C>+Send+Sync+'static>(&self, cb: &Cell<B>, mut f: FN) -> Stream<C> {
        let cb = cb.clone();
        let cb_node = cb.node();
        let mut f_deps = lambda2_deps(&f);
        f_deps.push(cb_node.gc_node.clone());
        self.map(lambda1(move |a: &A| f.call(a, &cb.sample()), f_deps))
    }

    pub fn snapshot1<B:Send+Clone+'static>(&self, cb: &Cell<B>) -> Stream<B> {
        self.snapshot(cb, |_a: &A, b: &B| b.clone())
    }

    pub fn map<B:Send+'static,FN:IsLambda1<A,B>+Send+Sync+'static>(&self, mut f: FN) -> Stream<B> {
        let self_ = self.clone();
        let sodium_ctx = self.sodium_ctx().clone();
        Stream::_new(
            &sodium_ctx,
            "Stream::map",
            |s: &Stream<B>| {
                let _s = s.clone();
                let f_deps = lambda1_deps(&f);
                let node = Node::new(
                    &sodium_ctx,
                    "Stream::map node",
                    move || {
                        self_.with_firing_op(|firing_op: &mut Option<A>| {
                            if let Some(ref firing) = firing_op {
                                _s._send(f.call(firing));
                            }
                        })
                    },
                    vec![self.node()]
                );
                node.add_update_dependencies(f_deps);
                node.add_update_dependencies(vec![self.gc_node.clone(), s.gc_node.clone()]);
                node
            }
        )
    }

    pub fn filter<PRED:IsLambda1<A,bool>+Send+Sync+'static>(&self, mut pred: PRED) -> Stream<A> where A: Clone {
        let sodium_ctx = self.sodium_ctx().clone();
        Stream::_new(
            &sodium_ctx,
            "Stream::filter",
            |s: &Stream<A>| {
                let self_ = self.clone();
                let self_gc_node = self.gc_node.clone();
                let _s = s.clone();
                let pred_deps = lambda1_deps(&pred);
                let node = Node::new(
                    &sodium_ctx,
                    "Stream::filter node",
                    move || {
                        self_.with_firing_op(|firing_op: &mut Option<A>| {
                            let firing_op2 = firing_op.clone().filter(|firing| pred.call(firing));
                            if let Some(firing) = firing_op2 {
                                _s._send(firing);
                            }
                        });
                    },
                    vec![self.node()]
                );
                node.add_update_dependencies(pred_deps);
                node.add_update_dependencies(vec![self_gc_node, s.gc_node.clone()]);
                node
            }
        )
    }

    pub fn or_else(&self, s2: &Stream<A>) -> Stream<A> where A: Clone {
        self.merge(s2, |lhs:&A, _rhs:&A| lhs.clone())
    }

    pub fn merge<FN:IsLambda2<A,A,A>+Send+Sync+'static>(&self, s2: &Stream<A>, mut f: FN) -> Stream<A> where A: Clone {
        let _self = self.clone();
        let s2 = s2.clone();
        let sodium_ctx = self.sodium_ctx().clone();
        Stream::_new(
            &sodium_ctx,
            "Stream::merge",
            |s: &Stream<A>| {
                let s_ = s.clone();
                let s2_ = s2.clone();
                let self_ = self.clone();
                let self_gc_node = self.gc_node.clone();
                let s2_gc_node = s2.gc_node.clone();
                let f_deps = lambda2_deps(&f);
                let node = Node::new(
                    &sodium_ctx,
                    "Stream::merge node",
                    move || {
                        self_.with_firing_op(|firing1_op: &mut Option<A>| {
                            s2_.with_firing_op(|firing2_op: &mut Option<A>| {
                                if let Some(ref firing1) = firing1_op {
                                    if let Some(ref firing2) = firing2_op {
                                        s_._send(f.call(firing1, firing2));
                                    } else {
                                        s_._send(firing1.clone());
                                    }
                                } else {
                                    if let Some(ref firing2) = firing2_op {
                                        s_._send(firing2.clone());
                                    }
                                }
                            })
                        })
                    },
                    vec![self.node(), s2.node()]
                );
                node.add_update_dependencies(f_deps);
                node.add_update_dependencies(vec![self_gc_node, s2_gc_node, s.gc_node.clone()]);
                node
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

    pub fn collect_lazy<B,S,F>(&self, init_state: Lazy<S>, f: F) -> Stream<B>
        where B: Send + Clone + 'static,
              S: Send + Clone + 'static,
              F: IsLambda2<A,S,(B,S)> + Send + Sync + 'static
    {
        let sodium_ctx = self.sodium_ctx();
        sodium_ctx.transaction(|| {
            let ea = self.clone();
            let es = StreamLoop::new(&sodium_ctx);
            let s = es.stream().hold_lazy(init_state);
            let ebs = ea.snapshot(&s, f);
            let eb = ebs.map(|(ref a,ref _b):&(B,S)| a.clone());
            let es_out = ebs.map(|(ref _a,ref b):&(B,S)| b.clone());
            es.loop_(&es_out);
            eb
        })
    }

    pub fn accum_lazy<S,F>(&self, init_state: Lazy<S>, f: F) -> Cell<S>
        where S: Send + Clone + 'static,
              F: IsLambda2<A,S,S> + Send + Sync + 'static
    {
        let sodium_ctx = self.sodium_ctx();
        let sodium_ctx = &sodium_ctx;
        sodium_ctx.transaction(|| {
            let es: StreamLoop<S> = StreamLoop::new(sodium_ctx);
            let s = es.stream().hold_lazy(init_state);
            let es_out = self.snapshot(&s, f);
            es.loop_(&es_out);
            s
        })
    }

    pub fn defer(&self) -> Stream<A> where A: Clone {
        let sodium_ctx = self.sodium_ctx();
        sodium_ctx.transaction(|| {
            let ss = StreamSink::new(&sodium_ctx);
            let s = ss.stream();
            let sodium_ctx = sodium_ctx.clone();
            let ss = StreamSink::downgrade(&ss);
            let listener = self.listen_weak(move |a:&A| {
                let ss = ss.upgrade().unwrap();
                let a = a.clone();
                sodium_ctx.post(move || ss.send(a.clone()))
            });
            s.node().add_keep_alive(&listener.gc_node);
            return s;
        })
    }

    pub fn once(&self) -> Stream<A> where A: Clone {
        let sodium_ctx = self.sodium_ctx().clone();
        Stream::_new(
            &sodium_ctx,
            "Stream::once",
            |s: &Stream<A>| {
                let s_ = s.clone();
                let sodium_ctx = sodium_ctx.clone();
                let sodium_ctx2 = sodium_ctx.clone();
                let _self = Stream::downgrade(self);
                let node = Node::new(
                    &sodium_ctx2,
                    "Stream::once node",
                    move || {
                        let _self = _self.upgrade().unwrap();
                        _self.with_firing_op(|firing_op: &mut Option<A>| {
                            if let Some(ref firing) = firing_op {
                                s_._send(firing.clone());
                                let node = s_.node();
                                sodium_ctx.post(move || {
                                    let deps;
                                    {
                                        let dependencies = node.data.dependencies.write().unwrap();
                                        deps = (*dependencies).clone();
                                    }
                                    for dep in deps {
                                        node.remove_dependency(&dep);
                                    }
                                });
                            }
                        })
                    },
                    vec![self.node()]
                );
                node.add_update_dependencies(vec![s.gc_node.clone()]);
                node
            }
        )
    }

    pub fn _listen<K:IsLambda1<A,()>+Send+Sync+'static>(&self, mut k: K, weak: bool) -> Listener {
        let self_ = self.clone();
        let self_gc_node = self.gc_node.clone();
        let node =
            Node::new(
                &self.sodium_ctx(),
                "Stream::listen node",
                move || {
                    self_.with_data(|data: &mut StreamData<A>| {
                        for firing in &data.firing_op {
                            k.call(firing)
                        }
                    });
                },
                vec![self.node()]
            );
        node.add_update_dependencies(vec![self_gc_node]);
        Listener::new(&self.sodium_ctx(), weak, node)
    }

    pub fn listen_weak<K:IsLambda1<A,()>+Send+Sync+'static>(&self, k: K) -> Listener {
        self._listen(k, true)
    }

    pub fn listen<K:IsLambda1<A,()>+Send+Sync+'static>(&self, k: K) -> Listener {
        self._listen(k, false)
    }

    pub fn _send(&self, a: A) {
        let sodium_ctx = self.sodium_ctx();
        let sodium_ctx = &sodium_ctx;
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
                is_first
            });
            {
                let self_node = self.node();
                let mut changed = self_node.data.changed.write().unwrap();
                *changed = true;
            }
            if is_first {
                let _self = self.clone();
                let self_node = _self.node();
                sodium_ctx.pre_post(move || {
                    _self.with_data(|data: &mut StreamData<A>| {
                        data.firing_op = None;
                        {
                            let mut changed = self_node.data.changed.write().unwrap();
                            *changed = false;
                        }
                    });
                });
            }
        });
    }

    pub fn downgrade(this: &Self) -> WeakStream<A> {
        WeakStream {
            data: Arc::downgrade(&this.data),
            gc_node: this.gc_node.clone()
        }
    }
}

impl<A> WeakStream<A> {
    pub fn upgrade(&self) -> Option<Stream<A>> {
        let data_op = self.data.upgrade();
        if let Some(data) = data_op {
            let s = Stream { data, gc_node: self.gc_node.clone() };
            s.gc_node.inc_ref();
            return Some(s);
        }
        None
    }
}
