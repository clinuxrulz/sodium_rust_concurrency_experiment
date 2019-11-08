use crate::impl_::node::Node;
use crate::impl_::sodium_ctx::SodiumCtx;
use crate::impl_::sodium_ctx::SodiumCtxData;

use std::sync::Arc;
use std::sync::Mutex;

#[derive(Clone)]
pub struct Listener {
    pub data: Arc<Mutex<ListenerData>>
}

pub struct ListenerData {
    pub sodium_ctx: SodiumCtx,
    pub is_weak: bool,
    pub node_op: Option<Node>
}

impl Listener {
    pub fn new(sodium_ctx: &SodiumCtx, is_weak: bool, node: Node) -> Listener {
        let listener = Listener {
            data: Arc::new(Mutex::new(ListenerData {
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
            let mut l = self.data.lock();
            let data: &mut ListenerData = l.as_mut().unwrap();
            data.node_op = None;
            is_weak = data.is_weak;
            sodium_ctx = data.sodium_ctx.clone();
        }
        if !is_weak {
            sodium_ctx.with_data(|data: &mut SodiumCtxData| {
                data.keep_alive.retain(|l:&Listener| !Arc::ptr_eq(&l.data,&self.data))
            });
        }
    }
}
