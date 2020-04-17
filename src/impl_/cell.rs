use crate::impl_::gc_node::{GcNode, Tracer};
use crate::impl_::lazy::Lazy;
use crate::impl_::listener::Listener;
use crate::impl_::node::{Node, NodeData};
use crate::impl_::sodium_ctx::SodiumCtx;
use crate::impl_::sodium_ctx::SodiumCtxData;
use crate::impl_::stream::Stream;
use crate::impl_::stream::WeakStream;
use crate::impl_::lambda::IsLambda1;
use crate::impl_::lambda::IsLambda2;
use crate::impl_::lambda::IsLambda3;
use crate::impl_::lambda::IsLambda4;
use crate::impl_::lambda::IsLambda5;
use crate::impl_::lambda::IsLambda6;
use crate::impl_::lambda::{lambda1, lambda2, lambda3, lambda2_deps, lambda3_deps, lambda4_deps, lambda5_deps, lambda6_deps};

use std::mem;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::Weak;

pub struct Cell<A> {
    pub data: Arc<Mutex<CellData<A>>>,
    pub gc_node: GcNode
}

pub struct WeakCell<A> {
    pub data: Weak<Mutex<CellData<A>>>,
    pub gc_node: GcNode
}

impl<A> Clone for Cell<A> {
    fn clone(&self) -> Self {
        self.gc_node.inc_ref();
        Cell {
            data: self.data.clone(),
            gc_node: self.gc_node.clone()
        }
    }
}

impl<A> Drop for Cell<A> {
    fn drop(&mut self) {
        self.gc_node.dec_ref();
    }
}

pub struct CellData<A> {
    node: Node,
    stream: Stream<A>,
    value: Lazy<A>,
    next_value_op: Option<A>
}

impl<A> Cell<A> {
    pub fn with_data<R,K:FnOnce(&mut CellData<A>)->R>(&self, k: K) -> R {
        let mut l = self.data.lock();
        let data: &mut CellData<A> = l.as_mut().unwrap();
        k(data)
    }

    pub fn sodium_ctx(&self) -> SodiumCtx {
        self.with_data(|data: &mut CellData<A>| data.stream.sodium_ctx())
    }

    pub fn node(&self) -> Node {
        self.with_data(|data: &mut CellData<A>| data.node.clone())
    }
}

impl<A:Send+'static> Cell<A> {
    pub fn new(sodium_ctx: &SodiumCtx, value: A) -> Cell<A> where A: Clone {
        let cell_data = Arc::new(Mutex::new(CellData {
            node: sodium_ctx.null_node(),
            stream: Stream::new(sodium_ctx),
            value: Lazy::of_value(value),
            next_value_op: None,
        }));
        let gc_node_trace;
        {
            let cell_data = cell_data.clone();
            gc_node_trace = move |tracer: &mut Tracer| {
                let l = cell_data.lock();
                let cell_data = l.as_ref().unwrap();
                tracer(&cell_data.node.gc_node);
                tracer(&cell_data.stream.gc_node);
            };
        }
        Cell {
            data: cell_data,
            gc_node: GcNode::new(&sodium_ctx.gc_ctx(), "Cell::new", || {}, gc_node_trace)
        }
    }

    pub fn _new(sodium_ctx: &SodiumCtx, stream: Stream<A>, value: Lazy<A>) -> Cell<A> where A: Clone {
        let init_value =
            stream.with_firing_op(|firing_op: &mut Option<A>| {
                if let Some(ref firing) = firing_op {
                    let firing = firing.clone();
                    Lazy::new(move || firing.clone())
                } else {
                    value
                }
            });
        let cell_data = Arc::new(Mutex::new(CellData {
            node: sodium_ctx.null_node(),
            stream: stream.clone(),
            value: init_value,
            next_value_op: None,
        }));
        let gc_node_trace;
        {
            let cell_data = cell_data.clone();
            gc_node_trace = move |tracer: &mut Tracer| {
                let l = cell_data.lock();
                let cell_data = l.as_ref().unwrap();
                tracer(&cell_data.node.gc_node);
                tracer(&cell_data.stream.gc_node);
            };
        }
        let c = Cell {
            data: cell_data,
            gc_node: GcNode::new(&sodium_ctx.gc_ctx(), "Cell::hold", || {}, gc_node_trace)
        };
        let sodium_ctx = sodium_ctx.clone();
        let node: Node;
        {
            let c = c.clone();
            let c_gc_node = c.gc_node.clone();
            let stream_node = stream.node();
            let stream_gc_node = stream.gc_node.clone();
            let sodium_ctx2 = sodium_ctx.clone();
            node = Node::new(
                &sodium_ctx2,
                "Cell::hold inner node",
                move || {
                    let firing_op = stream.with_firing_op(|firing_op| firing_op.clone());
                    if let Some(firing) = firing_op {
                        let is_first =
                            c.with_data(|data: &mut CellData<A>| {
                                let is_first = data.next_value_op.is_none();
                                data.next_value_op = Some(firing);
                                is_first
                            });
                        if is_first {
                            let c = c.clone();
                            sodium_ctx.post(move || {
                                c.with_data(|data: &mut CellData<A>| {
                                    let mut next_value_op: Option<A> = None;
                                    mem::swap(&mut next_value_op, &mut data.next_value_op);
                                    if let Some(next_value) = next_value_op {
                                        data.value = Lazy::of_value(next_value);
                                    }
                                })
                            });
                        }
                    }
                },
                vec![stream_node]
            );
            node.add_update_dependencies(vec![stream_gc_node, c_gc_node]);
        }
        c.with_data(|data: &mut CellData<A>| data.node = node);
        c
    }

    pub fn sample(&self) -> A where A: Clone {
        self.with_data(|data: &mut CellData<A>| data.value.run())
    }

    pub fn sample_lazy(&self) -> Lazy<A> {
        self.with_data(|data: &mut CellData<A>| data.value.clone())
    }

    pub fn updates(&self) -> Stream<A> {
        self.with_data(|data: &mut CellData<A>| data.stream.clone())
    }

    pub fn value(&self) -> Stream<A> where A: Clone {
        let sodium_ctx = self.sodium_ctx();
        sodium_ctx.transaction(|| {
            let s1 = self.updates();
            let spark: Stream<A> = Stream::new(&sodium_ctx);
            let sodium_ctx2 = sodium_ctx.clone();
            {
                let spark = spark.clone();
                let self_ = self.clone();
                sodium_ctx.post(move || {
                    let a = self_.with_data(|data: &mut CellData<A>| data.value.run());
                    sodium_ctx2.transaction(|| {
                        let node = spark.node();
                        {
                            let mut changed = node.data.changed.write().unwrap();
                            *changed = true;
                        }
                        sodium_ctx2.with_data(|data: &mut SodiumCtxData| data.changed_nodes.push(node));
                        spark._send(a.clone());
                    });
                });
            }
            s1.or_else(&spark)
        })
    }

    pub fn map<B:Send+'static,FN:IsLambda1<A,B>+Send+Sync+'static>(&self, mut f: FN) -> Cell<B> where A: Clone, B: Clone {
        let init = f.call(&self.sample());
        self.updates().map(f).hold(init)
    }

    pub fn lift2<B:Send+Clone+'static,C:Send+Clone+'static,FN:IsLambda2<A,B,C>+Send+'static>(&self, cb: &Cell<B>, f: FN) -> Cell<C> where A: Clone {
        let sodium_ctx = self.sodium_ctx();
        let lhs = self.sample_lazy();
        let rhs = cb.sample_lazy();
        let init: Lazy<C>;
        let f_deps = lambda2_deps(&f);
        let f = Arc::new(Mutex::new(f));
        {
            let lhs = lhs.clone();
            let rhs = rhs.clone();
            let f = f.clone();
            init = Lazy::new(move || {
                let mut l = f.lock();
                let f = l.as_mut().unwrap();
                f.call(&lhs.run(), &rhs.run())
            });
        }
        let state: Arc<Mutex<(Lazy<A>,Lazy<B>)>> = Arc::new(Mutex::new((lhs, rhs)));
        let s1: Stream<()>;
        let s2: Stream<()>;
        {
            let state = state.clone();
            s1 = self.updates().map(move |a: &A| {
                let mut l = state.lock();
                let state2: &mut (Lazy<A>,Lazy<B>) = l.as_mut().unwrap();
                state2.0 = Lazy::of_value(a.clone());
            });
        }
        {
            let state = state.clone();
            s2 = cb.updates().map(move |b: &B| {
                let mut l = state.lock();
                let state2: &mut (Lazy<A>,Lazy<B>) = l.as_mut().unwrap();
                state2.1 = Lazy::of_value(b.clone());
            });
        }
        let s = s1.or_else(&s2).map(lambda1(move |_: &()| {
            let l = state.lock();
            let state2: &(Lazy<A>,Lazy<B>) = l.as_ref().unwrap();
            let mut l = f.lock();
            let f = l.as_mut().unwrap();
            f.call(&state2.0.run(), &state2.1.run())
        }, f_deps));
        Cell::_new(
            &sodium_ctx,
            s,
            init
        )
    }

    pub fn lift3<
        B:Send+Clone+'static,
        C:Send+Clone+'static,
        D:Send+Clone+'static,
        FN:IsLambda3<A,B,C,D>+Send+'static
    >(&self, cb: &Cell<B>, cc: &Cell<C>, mut f: FN) -> Cell<D> where A: Clone {
        let f_deps = lambda3_deps(&f);
        self
            .lift2(
                cb,
                |a: &A, b: &B| (a.clone(), b.clone())
            )
            .lift2(
                cc,
                lambda2(move |(ref a, ref b): &(A,B), c: &C| f.call(a, b, c), f_deps)
            )
    }

    pub fn lift4<
        B:Send+Clone+'static,
        C:Send+Clone+'static,
        D:Send+Clone+'static,
        E:Send+Clone+'static,
        FN:IsLambda4<A,B,C,D,E>+Send+'static
    >(&self, cb: &Cell<B>, cc: &Cell<C>, cd: &Cell<D>, mut f: FN) -> Cell<E> where A: Clone {
        let f_deps = lambda4_deps(&f);
        self
            .lift3(
                cb,
                cc,
                |a: &A, b: &B, c: &C| (a.clone(), b.clone(), c.clone())
            )
            .lift2(
                cd,
                lambda2(move |(ref a, ref b, ref c): &(A,B,C), d: &D| f.call(a, b, c, d), f_deps)
            )
    }

    pub fn lift5<
        B:Send+Clone+'static,
        C:Send+Clone+'static,
        D:Send+Clone+'static,
        E:Send+Clone+'static,
        F:Send+Clone+'static,
        FN:IsLambda5<A,B,C,D,E,F>+Send+'static
    >(&self, cb: &Cell<B>, cc: &Cell<C>, cd: &Cell<D>, ce: &Cell<E>, mut f: FN) -> Cell<F> where A: Clone {
        let f_deps = lambda5_deps(&f);
        self
            .lift3(
                cb,
                cc,
                |a: &A, b: &B, c: &C| (a.clone(), b.clone(), c.clone())
            )
            .lift3(
                cd,
                ce,
                lambda3(move |(ref a, ref b, ref c): &(A,B,C), d: &D, e: &E| f.call(a, b, c, d, e), f_deps)
            )
    }

    pub fn lift6<
        B:Send+Clone+'static,
        C:Send+Clone+'static,
        D:Send+Clone+'static,
        E:Send+Clone+'static,
        F:Send+Clone+'static,
        G:Send+Clone+'static,
        FN:IsLambda6<A,B,C,D,E,F,G>+Send+'static
    >(&self, cb: &Cell<B>, cc: &Cell<C>, cd: &Cell<D>, ce: &Cell<E>, cf: &Cell<F>, mut f: FN) -> Cell<G> where A: Clone {
        let f_deps = lambda6_deps(&f);
        self
            .lift4(
                cb,
                cc,
                cd,
                |a: &A, b: &B, c: &C, d: &D| (a.clone(), b.clone(), c.clone(), d.clone())
            )
            .lift3(
                ce,
                cf,
                lambda3(move |(ref a, ref b, ref c, ref d): &(A,B,C,D), e: &E, f2: &F| f.call(a, b, c, d, e, f2), f_deps)
            )
    }

    pub fn switch_s(csa: &Cell<Stream<A>>) -> Stream<A> where A: Clone {
        let csa = csa.clone();
        let sodium_ctx = csa.sodium_ctx();
        Stream::_new(
            &sodium_ctx,
            "Cell::switch_s",
            |sa: &Stream<A>| {
                let inner_s: Arc<Mutex<WeakStream<A>>> = Arc::new(Mutex::new(Stream::downgrade(&csa.sample())));
                let sa = sa.clone();
                let node1: Node;
                {
                    let inner_s = inner_s.clone();
                    let sa = Stream::downgrade(&sa);
                    node1 = Node::new(
                        &sodium_ctx,
                        "switch_s inner node",
                        move || {
                            let l = inner_s.lock();
                            let inner_s: &WeakStream<A> = l.as_ref().unwrap();
                            let inner_s = inner_s.upgrade().unwrap();
                            inner_s.with_firing_op(|firing_op: &mut Option<A>| {
                                if let Some(ref firing) = firing_op {
                                    let sa = sa.upgrade().unwrap();
                                    sa._send(firing.clone());
                                }
                            });
                        },
                        vec![csa.sample().node()]
                    );
                }
                let node2: Node;
                {
                    let node1: Node = node1.clone();
                    let csa2 = csa.clone();
                    let sodium_ctx = sodium_ctx.clone();
                    let sodium_ctx2 = sodium_ctx.clone();
                    node2 = Node::new(
                        &sodium_ctx2,
                        "switch_s outer node",
                        move || {
                            csa.updates().with_firing_op(|firing_op: &mut Option<Stream<A>>| {
                                if let Some(ref firing) = firing_op {
                                    let firing = firing.clone();
                                    let node1 = node1.clone();
                                    let inner_s = inner_s.clone();
                                    sodium_ctx.pre_post(move || {
                                        let old_deps;
                                        {
                                            let dependencies = node1.data.dependencies.read().unwrap();
                                            old_deps = (*dependencies).clone();
                                        }
                                        for dep in old_deps {
                                            node1.remove_dependency(&dep);
                                        }
                                        node1.add_dependency(firing.node());
                                        let mut l = inner_s.lock();
                                        let inner_s: &mut WeakStream<A> = l.as_mut().unwrap();
                                        *inner_s = Stream::downgrade(&firing);
                                    });
                                }
                            });
                        },
                        vec![csa2.updates().node()]
                    );
                }
                node2.add_update_dependencies(vec![node1.gc_node.clone()]);
                node1.add_keep_alive(&node2.gc_node);
                return node1;
            }
        )
    }

    pub fn switch_c(cca: &Cell<Cell<A>>) -> Cell<A> where A: Clone {
        let cca2 = cca.clone();
        let cca = cca.clone();
        let sodium_ctx = cca.sodium_ctx();
        Stream::_new(
            &sodium_ctx,
            "Cell::switch_c",
            |sa: &Stream<A>| {
                let node1 = Node::new(
                    &sodium_ctx,
                    "switch_c outer node",
                    || {},
                    vec![cca.updates().node()]
                );
                let init_inner_s = cca.sample().updates();
                let last_inner_s = Arc::new(Mutex::new(Stream::downgrade(&init_inner_s)));
                let node2 = Node::new(
                    &sodium_ctx,
                    "switch_c inner node",
                    || {},
                    vec![node1.clone(), init_inner_s.node()]
                );
                let node1_update;
                {
                    let sodium_ctx = sodium_ctx.clone();
                    let node1 = node1.clone();
                    let node2 = node2.clone();
                    let cca = cca.clone();
                    let sa = Stream::downgrade(sa);
                    let last_inner_s = last_inner_s.clone();
                    node1_update = move || {
                        cca.updates().with_firing_op(|firing_op: &mut Option<Cell<A>>| {
                            if let Some(ref firing) = firing_op {
                                // will be overwriten by node2 firing if there is one
                                sodium_ctx.update_node(&firing.updates().node());
                                let sa = sa.upgrade().unwrap();
                                sa._send(firing.sample());
                                //
                                {
                                    let mut changed = node1.data.changed.write().unwrap();
                                    *changed = true;
                                }
                                {
                                    let mut changed = node2.data.changed.write().unwrap();
                                    *changed = true;
                                }
                                let new_inner_s = firing.updates();
                                new_inner_s.with_firing_op(|firing2_op: &mut Option<A>| {
                                    if let Some(ref firing2) = firing2_op {
                                        sa._send(firing2.clone());
                                    }
                                });
                                let mut l = last_inner_s.lock();
                                let last_inner_s: &mut WeakStream<A> = l.as_mut().unwrap();
                                node2.remove_dependency(&last_inner_s.upgrade().unwrap().node());
                                node2.add_dependency(new_inner_s.node());
                                {
                                    let mut changed = node2.data.changed.write().unwrap();
                                    *changed = true;
                                }
                                *last_inner_s = Stream::downgrade(&new_inner_s);
                            }
                        });
                    };
                }
                node1.add_update_dependencies(vec![node1.gc_node.clone(), node2.gc_node.clone()]);
                {
                    let mut update = node1.data.update.write().unwrap();
                    *update = Box::new(node1_update);
                }
                {
                    let last_inner_s = last_inner_s.clone();
                    let sa = Stream::downgrade(sa);
                    let node2_update = move || {
                        let l = last_inner_s.lock();
                        let last_inner_s: &WeakStream<A> = l.as_ref().unwrap();
                        let last_inner_s = last_inner_s.upgrade().unwrap();
                        last_inner_s.with_firing_op(|firing_op: &mut Option<A>| {
                            if let Some(ref firing) = firing_op {
                                let sa = sa.upgrade().unwrap();
                                sa._send(firing.clone());
                            }
                        });
                    };
                    {
                        let mut update = node2.data.update.write().unwrap();
                        *update = Box::new(node2_update);
                    }
                }
                node2
            }
        )
        .hold_lazy(Lazy::new(move || cca2.sample().sample()))
    }

    pub fn listen_weak<K: FnMut(&A)+Send+Sync+'static>(&self, k: K) -> Listener where A: Clone {
        self.sodium_ctx().transaction(|| {
            self.value().listen_weak(k)
        })
    }

    pub fn listen<K:IsLambda1<A,()>+Send+Sync+'static>(&self, k: K) -> Listener where A: Clone {
        self.sodium_ctx().transaction(|| {
            self.value().listen(k)
        })
    }

    pub fn downgrade(this: &Self) -> WeakCell<A> {
        WeakCell {
            data: Arc::downgrade(&this.data),
            gc_node: this.gc_node.clone()
        }
    }
}

impl<A> WeakCell<A> {
    pub fn upgrade(&self) -> Option<Cell<A>> {
        if let Some(data) = self.data.upgrade() {
            self.gc_node.inc_ref();
            return Some(Cell { data, gc_node: self.gc_node.clone() });
        }
        None
    }
}
