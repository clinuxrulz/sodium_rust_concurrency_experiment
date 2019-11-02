use crate::node::Node;
use crate::sodium_ctx::SodiumCtx;
use crate::stream::Stream;

use std::sync::Arc;
use std::sync::Mutex;

pub struct Cell<A> {
    data: Arc<Mutex<CellData<A>>>
}

pub struct CellData<A> {
    value: A,
    stream: Stream<A>,
    node: Node
}

impl<A:Send+'static> Cell<A> {
    pub fn new(sodium_ctx: &SodiumCtx, value: A) -> Cell<A> {
        Cell {
            data: Arc::new(Mutex::new(CellData {
                value: value,
                stream: Stream::new(sodium_ctx),
                node: sodium_ctx.null_node()
            }))
        }
    }
}
