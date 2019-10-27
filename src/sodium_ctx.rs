use std::mem;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use crate::node::Node;
use crate::node::NodeData;

#[derive(Clone)]
pub struct SodiumCtx {
    data: Arc<Mutex<SodiumCtxData>>
}

pub struct SodiumCtxData {
    pub changed_nodes: Vec<Node>,
    pub visited_nodes: Vec<Node>,
    pub transaction_depth: u32,
    pub post: Vec<Box<dyn FnMut()+Send>>
}

impl SodiumCtx {
    pub fn new() -> SodiumCtx {
        SodiumCtx {
            data:
                Arc::new(Mutex::new(
                    SodiumCtxData {
                        changed_nodes: Vec::new(),
                        visited_nodes: Vec::new(),
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
        let post_op =
            self.with_data(|data: &mut SodiumCtxData| {
                data.transaction_depth = data.transaction_depth - 1;
                if data.transaction_depth == 0 {
                    let mut post: Vec<Box<dyn FnMut()+Send>> = Vec::new();
                    mem::swap(&mut post, &mut data.post);
                    return Some(post);
                }
                return None;
            });
        if let Some(post) = post_op {
            self.end_of_transaction();
            for mut k in post {
                k();
            }
        }
        return result;
    }
    
    pub fn post<K:FnMut()+Send+'static>(&self, k:K) {
        self.with_data(|data: &mut SodiumCtxData| {
            data.post.push(Box::new(k));
        });
    }

    pub fn with_data<R,K:FnOnce(&mut SodiumCtxData)->R>(&self, k: K) -> R {
        let mut l = self.data.lock();
        let data: &mut SodiumCtxData = l.as_mut().unwrap();
        k(data)
    }

    pub fn end_of_transaction(&self) {
        loop {
            let mut changed_nodes: Vec<Node> =
                self.with_data(|data: &mut SodiumCtxData| {
                    let mut changed_nodes: Vec<Node> = Vec::new();
                    mem::swap(&mut changed_nodes, &mut data.changed_nodes);
                    return changed_nodes;
                });
            if changed_nodes.is_empty() {
                break;
            }
            for node in changed_nodes {
                self.update_node(&node);
            }
        }
        // post
        let mut post =
            self.with_data(|data: &mut SodiumCtxData| {
                let mut post: Vec<Box<dyn FnMut()+Send>> = Vec::new();
                mem::swap(&mut post, &mut data.post);
                return post;
            });
        for mut k in post {
            k();
        }
    }

    pub fn update_node(&self, node: &Node) {
        let dependencies: Vec<Node> =
            node.with_data(|data: &mut NodeData| {
                data.visited = true;
                data.dependencies.clone()
            });
        {
            let node = node.clone();
            self.post(move || {
                node.with_data(|data: &mut NodeData| data.visited = false);
            });
        }
        // visit dependencies
        let handle;
        {
            let dependencies = dependencies.clone();
            let _self = self.clone();
            handle = thread::spawn(move || {
                for dependency in &dependencies {
                    let visit_it = dependency.with_data(|data: &mut NodeData| !data.visited);
                    if visit_it {
                        _self.update_node(dependency);
                    }
                }
            });
        }
        handle.join().unwrap();
        // any dependencies changed?
        let any_changed =
            dependencies
                .iter()
                .any(|node: &Node| { node.with_data(|data: &mut NodeData| data.changed) });
        // if dependencies changed, then execute update on current node
        if any_changed {
            let mut update: Box<dyn FnMut()+Send> = Box::new(|| {});
            node.with_data(|data: &mut NodeData| {
                mem::swap(&mut update, &mut data.update);
            });
            update();
            node.with_data(|data: &mut NodeData| {
                mem::swap(&mut update, &mut data.update);
            });
            // if self changed then update dependents
            let dependents = node.with_data(|data: &mut NodeData| {
                data.dependents.clone()
            });
            let handle;
            {
                let dependents = dependents.clone();
                let _self = self.clone();
                handle = thread::spawn(move || {
                    for dependent in dependents {
                        if let Some(dependent2) = dependent.upgrade() {
                            _self.update_node(&dependent2);
                        }
                    }
                });
                handle.join().unwrap();
            }
        }
    }
}
