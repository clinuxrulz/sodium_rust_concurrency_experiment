use crate::impl_::gc::{Finalize, Gc, GcCell, Trace, Tracer};
use crate::impl_::node::Node;
use crate::impl_::sodium_ctx::SodiumCtx;
use crate::impl_::sodium_ctx::SodiumCtxData;

use std::fmt;

#[derive(Clone)]
pub struct Listener {
    pub data: Gc<GcCell<ListenerData>>
}

impl Trace for Listener {
    fn trace(&self, tracer: &mut Tracer) {
        self.data.trace(tracer);
    }
}

pub struct ListenerData {
    pub sodium_ctx: SodiumCtx,
    pub is_weak: bool,
    pub node_op: Option<Node>
}

impl Trace for ListenerData {
    fn trace(&self, tracer: &mut Tracer) {
        if let Some(ref node) = self.node_op {
            node.trace(tracer);
        }
    }
}

impl Finalize for ListenerData {
    fn finalize(&mut self) {}
}

impl Listener {
    pub fn new(sodium_ctx: &SodiumCtx, is_weak: bool, node: Node) -> Listener {
        let listener = Listener {
            data: sodium_ctx.gc_ctx().new_gc(GcCell::new(ListenerData {
                sodium_ctx: sodium_ctx.clone(),
                node_op: Some(node),
                is_weak
            }))
        };
        if !is_weak {
            sodium_ctx.with_data(|data: &mut SodiumCtxData| {
                data.keep_alive.push(listener.clone());
            });
        }
        listener
    }

    pub fn unlisten(&self) {
        let is_weak;
        let sodium_ctx;
        {
            let mut l = self.data.borrow_mut();
            let data: &mut ListenerData = &mut l;
            data.node_op = None;
            is_weak = data.is_weak;
            sodium_ctx = data.sodium_ctx.clone();
        }
        if !is_weak {
            sodium_ctx.with_data(|data: &mut SodiumCtxData| {
                let ptr_str = format!("{:p}", self.data);
                data.keep_alive.retain(|l:&Listener| format!("{:p}",l.data) != ptr_str);
            });
        }
    }

    pub fn node_op(&self) -> Option<Node> {
        self.with_data(|data: &mut ListenerData| data.node_op.clone())
    }

    pub fn with_data<R,K:FnOnce(&mut ListenerData)->R>(&self, k: K) -> R {
        let mut l = self.data.borrow_mut();
        let data: &mut ListenerData = &mut l;
        k(data)
    }
}

impl fmt::Debug for Listener {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let node_op = self.node_op();
        write!(f, "(Listener")?;
        match node_op {
            Some(node) => {
                writeln!(f, "")?;
                writeln!(f, "{:?})", node)?;
            }
            None => {
                writeln!(f, ")")?;
            }
        }
        fmt::Result::Ok(())
    }
}
