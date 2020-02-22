use crate::impl_::dep::Dep;
use crate::impl_::gc::{Finalize, Gc, GcCell, GcWeak, Trace, Tracer};
use crate::impl_::lazy::Lazy;
use crate::impl_::listener::Listener;
use crate::impl_::node::Node;
use crate::impl_::node::NodeData;
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

use std::cell::RefCell;
use std::rc::Rc;
use std::mem;

pub struct Cell<A:'static> {
    data: Gc<GcCell<CellData<A>>>
}

impl<A> Trace for Cell<A> {
    fn trace(&self, tracer: &mut Tracer) {
        self.data.trace(tracer);
    }
}

pub struct WeakCell<A:'static> {
    data: GcWeak<GcCell<CellData<A>>>
}

impl<A> Clone for Cell<A> {
    fn clone(&self) -> Self {
        Cell {
            data: self.data.clone()
        }
    }
}

pub struct CellData<A:'static> {
    value: Lazy<A>,
    next_value_op: Option<A>,
    stream: Stream<A>,
    node: Node
}

impl<A> Trace for CellData<A> {
    fn trace(&self, tracer: &mut Tracer) {
        self.stream.trace(tracer);
        self.node.trace(tracer);
    }
}

impl<A> Finalize for CellData<A> {
    fn finalize(&mut self) {}
}

impl<A:'static> Cell<A> {
    pub fn new(sodium_ctx: &SodiumCtx, value: A) -> Cell<A> where A: Clone {
        Cell {
            data: sodium_ctx.gc_ctx().new_gc(GcCell::new(CellData {
                value: Lazy::of_value(value),
                next_value_op: None,
                stream: Stream::new(sodium_ctx),
                node: sodium_ctx.null_node()
            }))
        }
    }

    pub fn _new(sodium_ctx: &SodiumCtx, stream: Stream<A>, value: Lazy<A>) -> Cell<A> where A: Clone {
        let stream2 = stream.clone();
        let init_value =
            stream.with_firing_op(|firing_op: &mut Option<A>| {
                if let Some(ref firing) = firing_op {
                    let firing = firing.clone();
                    Lazy::new(move || firing.clone())
                } else {
                    value
                }
            });
        let c = Cell {
            data: sodium_ctx.gc_ctx().new_gc(GcCell::new(CellData {
                value: init_value,
                next_value_op: None,
                stream: stream.clone(),
                node: sodium_ctx.null_node()
            }))
        };
        let sodium_ctx = sodium_ctx.clone();
        let node: Node;
        {
            let c_ = Cell::downgrade(&c);
            let stream2 = Stream::downgrade(&stream2);
            let sodium_ctx2 = sodium_ctx.clone();
            node = Node::new(
                &sodium_ctx2,
                move || {
                    let c_ = c_.upgrade().unwrap();
                    let stream2 = stream2.upgrade().unwrap();
                    let firing_op = stream2.with_firing_op(|firing_op| firing_op.clone());
                    if let Some(firing) = firing_op {
                        let is_first =
                            c_.with_data(|data: &mut CellData<A>| {
                                let is_first = data.next_value_op.is_none();
                                data.next_value_op = Some(firing);
                                is_first
                            });
                        if is_first {
                            sodium_ctx.post(move || {
                                c_.with_data(|data: &mut CellData<A>| {
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
                vec![stream.node()]
            );
            node.add_keep_alive(Dep::new(c.clone()));
            node.add_keep_alive(Dep::new(stream.clone()));
        }
        c.with_data(|data: &mut CellData<A>| data.node = node);
        c
    }

    pub fn sodium_ctx(&self) -> SodiumCtx {
        self.with_data(|data: &mut CellData<A>| data.stream.sodium_ctx())
    }

    pub fn node(&self) -> Node {
        self.with_data(|data: &mut CellData<A>| data.node.clone())
    }

    pub fn sample(&self) -> A where A: Clone {
        self.with_data(|data: &mut CellData<A>| data.value.run())
    }

    pub fn sample_lazy(&self) -> Lazy<A> {
        self.with_data(|data: &mut CellData<A>| data.value.clone())
    }

    pub fn updates(&self) -> Stream<A> {
        self.with_data(|data| data.stream.clone())
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
                        node.with_data(|data: &mut NodeData| { data.changed = true; });
                        sodium_ctx2.with_data(|data: &mut SodiumCtxData| data.changed_nodes.push(node));
                        spark._send(a.clone());
                    });
                });
            }
            s1.or_else(&spark)
        })
    }

    pub fn map<B:'static,FN:IsLambda1<A,B>+'static>(&self, mut f: FN) -> Cell<B> where A: Clone, B: Clone {
        let init = f.call(&self.sample());
        self.updates().map(f).hold(init)
    }

    pub fn lift2<B:Clone+'static,C:Clone+'static,FN:IsLambda2<A,B,C>+'static>(&self, cb: &Cell<B>, f: FN) -> Cell<C> where A: Clone {
        let sodium_ctx = self.sodium_ctx();
        let lhs = self.sample_lazy();
        let rhs = cb.sample_lazy();
        let init: Lazy<C>;
        let f_deps = lambda2_deps(&f);
        let f = Rc::new(RefCell::new(f));
        {
            let lhs = lhs.clone();
            let rhs = rhs.clone();
            let f = f.clone();
            init = Lazy::new(move || {
                let mut l = f.borrow_mut();
                let f = &mut l;
                f.call(&lhs.run(), &rhs.run())
            });
        }
        let state: Rc<RefCell<(Lazy<A>,Lazy<B>)>> = Rc::new(RefCell::new((lhs, rhs)));
        let s1: Stream<()>;
        let s2: Stream<()>;
        {
            let state = state.clone();
            s1 = self.updates().map(move |a: &A| {
                let mut l = state.borrow_mut();
                let state2: &mut (Lazy<A>,Lazy<B>) = &mut l;
                state2.0 = Lazy::of_value(a.clone());
            });
        }
        {
            let state = state.clone();
            s2 = cb.updates().map(move |b: &B| {
                let mut l = state.borrow_mut();
                let state2: &mut (Lazy<A>,Lazy<B>) = &mut l;
                state2.1 = Lazy::of_value(b.clone());
            });
        }
        let s = s1.or_else(&s2).map(lambda1(move |_: &()| {
            let l = state.borrow();
            let state2: &(Lazy<A>,Lazy<B>) = &l;
            let mut l = f.borrow_mut();
            let f = &mut l;
            f.call(&state2.0.run(), &state2.1.run())
        }, f_deps));
        Cell::_new(
            &sodium_ctx,
            s,
            init
        )
    }

    pub fn lift3<
        B:Clone+'static,
        C:Clone+'static,
        D:Clone+'static,
        FN:IsLambda3<A,B,C,D>+'static
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
        B:Clone+'static,
        C:Clone+'static,
        D:Clone+'static,
        E:Clone+'static,
        FN:IsLambda4<A,B,C,D,E>+'static
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
        B:Clone+'static,
        C:Clone+'static,
        D:Clone+'static,
        E:Clone+'static,
        F:Clone+'static,
        FN:IsLambda5<A,B,C,D,E,F>+'static
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
        B:Clone+'static,
        C:Clone+'static,
        D:Clone+'static,
        E:Clone+'static,
        F:Clone+'static,
        G:Clone+'static,
        FN:IsLambda6<A,B,C,D,E,F,G>+'static
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
            |sa: &Stream<A>| {
                let inner_s: Gc<GcCell<Stream<A>>> = sodium_ctx.gc_ctx().new_gc(GcCell::new(csa.sample()));
                let sa = sa.clone();
                let node1: Node;
                {
                    let inner_s = Gc::downgrade(&inner_s);
                    let sa = Stream::downgrade(&sa);
                    node1 = Node::new(
                        &sodium_ctx,
                        move || {
                            let inner_s = inner_s.upgrade().unwrap();
                            let l = inner_s.borrow();
                            let inner_s: &Stream<A> = &l;
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
                node1.add_keep_alive(Dep::new(inner_s.clone()));
                node1.add_keep_alive(Dep::new(sa));
                let node2: Node;
                {
                    let inner_s = Gc::downgrade(&inner_s);
                    let node1 = Node::downgrade(&node1);
                    let csa2 = csa.clone();
                    let sodium_ctx = sodium_ctx.clone();
                    let sodium_ctx2 = sodium_ctx.clone();
                    node2 = Node::new(
                        &sodium_ctx2,
                        move || {
                            let node1 = node1.upgrade().unwrap();
                            csa.updates().with_firing_op(|firing_op: &mut Option<Stream<A>>| {
                                if let Some(ref firing) = firing_op {
                                    let firing = firing.clone();
                                    let node1 = node1.clone();
                                    let inner_s = inner_s.clone();
                                    sodium_ctx.pre_post(move || {
                                        let old_deps =
                                        node1.with_data(|data: &mut NodeData| {
                                            data.dependencies.clone()
                                        });
                                        for dep in old_deps {
                                            node1.remove_dependency(&dep);
                                        }
                                        node1.add_dependency(firing.node());
                                        let inner_s = inner_s.upgrade().unwrap();
                                        let mut l = inner_s.borrow_mut();
                                        let inner_s: &mut Stream<A> = &mut l;
                                        *inner_s = firing.clone();
                                    });
                                }
                            });
                        },
                        vec![csa2.updates().node()]
                    );
                }
                node2.add_keep_alive(Dep::new(node1.clone()));
                node2.add_keep_alive(Dep::new(inner_s));
                node1.add_keep_alive(Dep::new(node2));
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
            |sa: &Stream<A>| {
                let node1 = Node::new(
                    &sodium_ctx,
                    || {},
                    vec![cca.updates().node()]
                );
                let init_inner_s = cca.sample().updates();
                let last_inner_s = sodium_ctx.gc_ctx().new_gc(GcCell::new(init_inner_s.clone()));
                let node2 = Node::new(
                    &sodium_ctx,
                    || {},
                    vec![node1.clone(), init_inner_s.node()]
                );
                let node1_update;
                {
                    let sodium_ctx = sodium_ctx.clone();
                    let node1 = Node::downgrade(&node1);
                    let node2 = Node::downgrade(&node2);
                    let cca = Cell::downgrade(&cca);
                    let sa = Stream::downgrade(sa);
                    let last_inner_s = Gc::downgrade(&last_inner_s);
                    node1_update = move || {
                        let cca = cca.upgrade().unwrap();
                        let node1 = node1.upgrade().unwrap();
                        let node2 = node2.upgrade().unwrap();
                        let sa = sa.upgrade().unwrap();
                        let last_inner_s = last_inner_s.upgrade().unwrap();
                        cca.updates().with_firing_op(|firing_op: &mut Option<Cell<A>>| {
                            if let Some(ref firing) = firing_op {
                                // will be overwriten by node2 firing if there is one
                                sodium_ctx.update_node(&firing.updates().node());
                                sa._send(firing.sample());
                                //
                                node1.with_data(|data: &mut NodeData| data.changed = true);
                                node2.with_data(|data: &mut NodeData| data.changed = true);
                                let new_inner_s = firing.updates();
                                new_inner_s.with_firing_op(|firing2_op: &mut Option<A>| {
                                    if let Some(ref firing2) = firing2_op {
                                        sa._send(firing2.clone());
                                    }
                                });
                                let mut l = last_inner_s.borrow_mut();
                                let last_inner_s: &mut Stream<A> = &mut l;
                                node2.remove_dependency(&last_inner_s.node());
                                node2.add_dependency(new_inner_s.node());
                                node2.with_data(|data: &mut NodeData| data.changed = true);
                                *last_inner_s = new_inner_s;
                            }
                        });
                    };
                }
                node1.add_keep_alives(vec![
                    Dep::new(cca.clone()),
                    Dep::new(node1.clone()),
                    Dep::new(node2.clone()),
                    Dep::new(sa.clone()),
                    Dep::new(last_inner_s.clone())
                ]);
                node1.with_data(|data: &mut NodeData| data.update = Box::new(node1_update));
                {
                    let last_inner_s = Gc::downgrade(&last_inner_s);
                    let sa = Stream::downgrade(sa);
                    let node2_update = move || {
                        let last_inner_s = last_inner_s.upgrade().unwrap();
                        let l = last_inner_s.borrow();
                        let last_inner_s: &Stream<A> = &l;
                        last_inner_s.with_firing_op(|firing_op: &mut Option<A>| {
                            if let Some(ref firing) = firing_op {
                                let sa = sa.upgrade().unwrap();
                                sa._send(firing.clone());
                            }
                        });
                    };
                    node2.with_data(|data: &mut NodeData| data.update = Box::new(node2_update));
                }
                node2
            }
        )
        .hold_lazy(Lazy::new(move || cca2.sample().sample()))
    }

    pub fn listen_weak<K: FnMut(&A)+'static>(&self, k: K) -> Listener where A: Clone {
        self.sodium_ctx().transaction(|| {
            self.value().listen_weak(k)
        })
    }

    pub fn listen<K:IsLambda1<A,()>+'static>(&self, k: K) -> Listener where A: Clone {
        self.sodium_ctx().transaction(|| {
            self.value().listen(k)
        })
    }

    pub fn with_data<R,K:FnOnce(&mut CellData<A>)->R>(&self, k: K) -> R {
        let mut l = self.data.borrow_mut();
        let data: &mut CellData<A> = &mut l;
        k(data)
    }

    pub fn downgrade(this: &Self) -> WeakCell<A> {
        WeakCell {
            data: Gc::downgrade(&this.data)
        }
    }
}

impl<A> WeakCell<A> {
    pub fn upgrade(&self) -> Option<Cell<A>> {
        self.data.upgrade().map(|data| Cell { data })
    }
}
