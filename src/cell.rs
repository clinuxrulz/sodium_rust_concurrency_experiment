use crate::listener::Listener;
use crate::node::Node;
use crate::sodium_ctx::SodiumCtx;
use crate::stream::Stream;

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
                    spark._send(&sodium_ctx2, a.clone());
                });
            }
            s1.or_else(&spark)
        })
    }

    pub fn lift2<B:Send+Clone+'static,C:Send+Clone+'static,FN:FnMut(&A,&B)->C+Send+'static>(&self, cb: &Cell<B>, mut f: FN) -> Cell<C> where A: Clone {
        let sodium_ctx = self.sodium_ctx();
        let lhs = self.sample();
        let rhs = cb.sample();
        let init = f(&lhs, &rhs);
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
            f(&state2.0, &state2.1)
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
        FN:FnMut(&A,&B,&C)->D+Send+'static
    >(&self, cb: &Cell<B>, cc: &Cell<C>, mut f: FN) -> Cell<D> where A: Clone {
        self
            .lift2(
                cb,
                |a: &A, b: &B| (a.clone(), b.clone())
            )
            .lift2(
                cc,
                move |(ref a, ref b): &(A,B), c: &C| f(a, b, c)
            )
    }

    pub fn listen_weak<K: FnMut(&A)+Send+'static>(&self, mut k: K) -> Listener where A: Clone {
        self.value().listen_weak(k)
    }

    pub fn with_data<R,K:FnOnce(&mut CellData<A>)->R>(&self, k: K) -> R {
        let mut l = self.data.lock();
        let data: &mut CellData<A> = l.as_mut().unwrap();
        k(data)
    }
}
