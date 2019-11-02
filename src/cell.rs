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

    pub fn _new(sodium_ctx: &SodiumCtx, stream: Stream<A>, value: A) -> Cell<A> where A: Copy {
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

    pub fn with_data<R,K:FnOnce(&mut CellData<A>)->R>(&self, k: K) -> R {
        let mut l = self.data.lock();
        let data: &mut CellData<A> = l.as_mut().unwrap();
        k(data)
    }
}
