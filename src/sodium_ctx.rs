use std::sync::Arc;
use std::sync::Mutex;
use std::mem;
use crate::node::Node;

pub struct SodiumCtx {
    data: Arc<Mutex<SodiumCtxData>>
}

pub struct SodiumCtxData {
    pub dirty_nodes: Vec<Node>,
    pub transaction_depth: u32,
    pub post: Vec<Box<dyn FnMut()>>
}

impl SodiumCtx {
    pub fn new() -> SodiumCtx {
        SodiumCtx {
            data:
                Arc::new(Mutex::new(
                    SodiumCtxData {
                        dirty_nodes: Vec::new(),
                        transaction_depth: 0,
                        post: Vec::new()
                    }
                ))
        }
    }

    pub fn transaction<R,K:FnOnce()->R>(&self, k:K) -> R {
        self.with_data(|data: &mut SodiumCtxData| {
            data.transaction_depth = data.transaction_depth + 1;
        });
        let result = k();
        let post =
            self.with_data(|data: &mut SodiumCtxData| {
                data.transaction_depth = data.transaction_depth - 1;
                if (data.transaction_depth == 0) {
                    data.end_of_transaction();
                    let mut post: Vec<Box<dyn FnMut()>> = Vec::new();
                    mem::swap(&mut post, &mut data.post);
                    return post;
                }
                return Vec::new();
            });
        for mut k in post {
            k();
        }
        return result;
    }

    pub fn with_data<R,K:FnOnce(&mut SodiumCtxData)->R>(&self, k: K) -> R {
        let mut l = self.data.lock();
        let data: &mut SodiumCtxData = l.as_mut().unwrap();
        k(data)
    }
}

impl SodiumCtxData {
    pub fn end_of_transaction(&mut self) {

    }
}
