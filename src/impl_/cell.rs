use crate::impl_::listener::Listener;
use crate::impl_::node::Node;
use crate::impl_::node::NodeData;
use crate::impl_::sodium_ctx::SodiumCtx;
use crate::impl_::stream::Stream;
use crate::impl_::lambda::IsLambda1;
use crate::impl_::lambda::IsLambda2;
use crate::impl_::lambda::IsLambda3;
use crate::impl_::lambda::IsLambda4;
use crate::impl_::lambda::IsLambda5;
use crate::impl_::lambda::IsLambda6;

use std::mem;
use std::sync::Arc;
use std::sync::Mutex;

pub struct Cell<A> {
    data: Arc<Mutex<CellData<A>>>
}

impl<A> Clone for Cell<A> {
    fn clone(&self) -> Self {
        Cell {
            data: self.data.clone()
        }
    }
}

pub struct CellData<A> {
    value: A,
    next_value_op: Option<A>,
    stream: Stream<A>,
    node: Node
}

impl<A:Send+'static> Cell<A> {
    pub fn new(sodium_ctx: &SodiumCtx, value: A) -> Cell<A> {
        Cell {
            data: Arc::new(Mutex::new(CellData {
                value: value,
                next_value_op: None,
                stream: Stream::new(sodium_ctx),
                node: sodium_ctx.null_node()
            }))
        }
    }

    pub fn _new(sodium_ctx: &SodiumCtx, stream: Stream<A>, value: A) -> Cell<A> where A: Clone {
        let stream2 = stream.clone();
        let c = Cell {
            data: Arc::new(Mutex::new(CellData {
                value: value,
                next_value_op: None,
                stream: stream.clone(),
                node: sodium_ctx.null_node()
            }))
        };
        let sodium_ctx = sodium_ctx.clone();
        let node: Node;
        {
            let c = c.clone();
            node = Node::new(
                move || {
                    let firing_op = stream2.with_firing_op(|firing_op| firing_op.clone());
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
                                        data.value = next_value;
                                    }
                                })
                            });
                        }
                    }
                },
                vec![stream.node()]
            );
        }
        c.with_data(|data: &mut CellData<A>| data.node = node);
        c
    }

    pub fn sodium_ctx(&self) -> SodiumCtx {
        self.with_data(|data: &mut CellData<A>| data.stream.sodium_ctx())
    }

    pub fn sample(&self) -> A where A: Clone {
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
                let a: A = self.with_data(|data: &mut CellData<A>| data.value.clone());
                sodium_ctx.post(move || {
                    sodium_ctx2.transaction(|| {
                        spark._send(&sodium_ctx2, a.clone());
                        sodium_ctx2.add_dependents_to_changed_nodes(spark.node());
                    });
                });
            }
            s1.or_else(&spark)
        })
    }

    pub fn map<B:Send+'static,FN:IsLambda1<A,B>+Send+'static>(&self, mut f: FN) -> Cell<B> where A: Clone, B: Clone {
        let init = f.call(&self.sample());
        self.updates().map(f).hold(init)
    }

    pub fn lift2<B:Send+Clone+'static,C:Send+Clone+'static,FN:IsLambda2<A,B,C>+Send+'static>(&self, cb: &Cell<B>, mut f: FN) -> Cell<C> where A: Clone {
        let sodium_ctx = self.sodium_ctx();
        let lhs = self.sample();
        let rhs = cb.sample();
        let init = f.call(&lhs, &rhs);
        let state: Arc<Mutex<(A,B)>> = Arc::new(Mutex::new((lhs, rhs)));
        let s1: Stream<()>;
        let s2: Stream<()>;
        {
            let state = state.clone();
            s1 = self.updates().map(move |a: &A| {
                let mut l = state.lock();
                let state2: &mut (A,B) = l.as_mut().unwrap();
                state2.0 = a.clone();
            });
        }
        {
            let state = state.clone();
            s2 = cb.updates().map(move |b: &B| {
                let mut l = state.lock();
                let state2: &mut (A,B) = l.as_mut().unwrap();
                state2.1 = b.clone();
            });
        }
        let s = s1.or_else(&s2).map(move |_: &()| {
            let l = state.lock();
            let state2: &(A,B) = l.as_ref().unwrap();
            f.call(&state2.0, &state2.1)
        });
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
        self
            .lift2(
                cb,
                |a: &A, b: &B| (a.clone(), b.clone())
            )
            .lift2(
                cc,
                move |(ref a, ref b): &(A,B), c: &C| f.call(a, b, c)
            )
    }

    pub fn lift4<
        B:Send+Clone+'static,
        C:Send+Clone+'static,
        D:Send+Clone+'static,
        E:Send+Clone+'static,
        FN:IsLambda4<A,B,C,D,E>+Send+'static
    >(&self, cb: &Cell<B>, cc: &Cell<C>, cd: &Cell<D>, mut f: FN) -> Cell<E> where A: Clone {
        self
            .lift3(
                cb,
                cc,
                |a: &A, b: &B, c: &C| (a.clone(), b.clone(), c.clone())
            )
            .lift2(
                cd,
                move |(ref a, ref b, ref c): &(A,B,C), d: &D| f.call(a, b, c, d)
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
        self
            .lift3(
                cb,
                cc,
                |a: &A, b: &B, c: &C| (a.clone(), b.clone(), c.clone())
            )
            .lift3(
                cd,
                ce,
                move |(ref a, ref b, ref c): &(A,B,C), d: &D, e: &E| f.call(a, b, c, d, e)
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
                move |(ref a, ref b, ref c, ref d): &(A,B,C,D), e: &E, f2: &F| f.call(a, b, c, d, e, f2)
            )
    }

    pub fn switch_s(csa: &Cell<Stream<A>>) -> Stream<A> where A: Clone {
        let csa = csa.clone();
        let sodium_ctx = csa.sodium_ctx();
        Stream::_new(
            &sodium_ctx,
            |sa: &Stream<A>| {
                let sodium_ctx = sodium_ctx.clone();
                let inner_s: Arc<Mutex<Stream<A>>> = Arc::new(Mutex::new(csa.sample()));
                let sa = sa.clone();
                let node1: Node;
                {
                    let inner_s = inner_s.clone();
                    node1 = Node::new(
                        move || {
                            let l = inner_s.lock();
                            let inner_s: &Stream<A> = l.as_ref().unwrap();
                            inner_s.with_firing_op(|firing_op: &mut Option<A>| {
                                if let Some(ref firing) = firing_op {
                                    sa._send(&sodium_ctx, firing.clone());
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
                    node2 = Node::new(
                        move || {
                            csa.updates().with_firing_op(|firing_op: &mut Option<Stream<A>>| {
                                if let Some(ref firing) = firing_op {
                                    let old_deps =
                                        node1.with_data(|data: &mut NodeData| {
                                            data.dependencies.clone()
                                        });
                                    for dep in old_deps {
                                        node1.remove_dependency(&dep);
                                    }
                                    node1.add_dependency(firing.node());
                                    let mut l = inner_s.lock();
                                    let inner_s: &mut Stream<A> = l.as_mut().unwrap();
                                    *inner_s = firing.clone();
                                }
                            });
                        },
                        vec![csa2.updates().node()]
                    );
                }
                return Node::new(
                    || {},
                    vec![node1, node2]
                );
            }
        )
    }

    pub fn switch_c(cca: &Cell<Cell<A>>) -> Cell<A> where A: Clone {
        Cell::switch_s(&cca.map(|ca: &Cell<A>| ca.updates()))
            .or_else(&cca.updates().map(|ca: &Cell<A>| ca.sample()))
            .hold(cca.sample().sample())
    }

    pub fn listen_weak<K: FnMut(&A)+Send+'static>(&self, mut k: K) -> Listener where A: Clone {
        self.sodium_ctx().transaction(|| {
            self.value().listen_weak(k)
        })
    }

    pub fn with_data<R,K:FnOnce(&mut CellData<A>)->R>(&self, k: K) -> R {
        let mut l = self.data.lock();
        let data: &mut CellData<A> = l.as_mut().unwrap();
        k(data)
    }
}
