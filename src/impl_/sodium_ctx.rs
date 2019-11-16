use crate::impl_::listener::Listener;
use crate::impl_::node::Node;
use crate::impl_::node::NodeData;
use crate::impl_::node::WeakNode;

use std::mem;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;

#[derive(Clone)]
pub struct SodiumCtx {
    data: Arc<Mutex<SodiumCtxData>>
}

pub struct SodiumCtxData {
    pub null_node_op: Option<Node>,
    pub changed_nodes: Vec<Node>,
    pub visited_nodes: Vec<Node>,
    pub transaction_depth: u32,
    pub pre_post: Vec<Box<dyn FnMut()+Send>>,
    pub post: Vec<Box<dyn FnMut()+Send>>,
    pub keep_alive: Vec<Listener>,
    pub gc_roots: Vec<WeakNode>
}

impl SodiumCtx {
    pub fn new() -> SodiumCtx {
        SodiumCtx {
            data:
                Arc::new(Mutex::new(
                    SodiumCtxData {
                        null_node_op: None,
                        changed_nodes: Vec::new(),
                        visited_nodes: Vec::new(),
                        transaction_depth: 0,
                        pre_post: Vec::new(),
                        post: Vec::new(),
                        keep_alive: Vec::new(),
                        gc_roots: Vec::new()
                    }
                ))
        }
    }

    pub fn null_node(&self) -> Node {
        self.with_data(|data: &mut SodiumCtxData| {
            if let Some(ref null_node) = data.null_node_op {
                return null_node.clone();
            }
            let null_node = Node::new(self, || {}, Vec::new());
            data.null_node_op = Some(null_node.clone());
            return null_node;
        })
    }

    pub fn add_gc_root(&self, node: &Node) {
        self.with_data(|data: &mut SodiumCtxData| data.gc_roots.push(Node::downgrade(node)));
    }

    pub fn transaction<R,K:FnOnce()->R>(&self, k:K) -> R {
        self.with_data(|data: &mut SodiumCtxData| {
            data.transaction_depth = data.transaction_depth + 1;
        });
        let result = k();
        let is_end_of_transaction =
            self.with_data(|data: &mut SodiumCtxData| {
                data.transaction_depth = data.transaction_depth - 1;
                return data.transaction_depth == 0;
            });
        if is_end_of_transaction {
            self.end_of_transaction();
        }
        return result;
    }

    pub fn add_dependents_to_changed_nodes(&self, node: Node) {
        self.with_data(|data: &mut SodiumCtxData| {
            node.with_data(|data2: &mut NodeData| {
                data2.dependents
                    .iter()
                    .flat_map(|node: &WeakNode| {
                        node.upgrade()
                    })
                    .for_each(|node: Node| {
                        data.changed_nodes.push(node);
                    });
            });
        });
    }

    pub fn pre_post<K:FnMut()+Send+'static>(&self, k: K) {
        self.with_data(|data: &mut SodiumCtxData| {
            data.pre_post.push(Box::new(k));
        });
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
        self.with_data(|data: &mut SodiumCtxData| {
            data.transaction_depth = data.transaction_depth + 1;
        });
        loop {
            let changed_nodes: Vec<Node> =
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
        self.with_data(|data: &mut SodiumCtxData| {
            data.transaction_depth = data.transaction_depth - 1;
        });
        // pre_post
        let pre_post =
            self.with_data(|data: &mut SodiumCtxData| {
                let mut pre_post: Vec<Box<dyn FnMut()+Send>> = Vec::new();
                mem::swap(&mut pre_post, &mut data.pre_post);
                return pre_post;
            });
        for mut k in pre_post {
            k();
        }
        // post
        let post =
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
        let bail = node.with_data(|data: &mut NodeData| data.visited.clone());
        if bail {
            return;
        }
        let dependencies: Vec<Node> =
            node.with_data(|data: &mut NodeData| {
                data.visited = true;
                data.dependencies.clone()
            });
        {
            let node = node.clone();
            self.pre_post(move || {
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
        }
        // if self changed then update dependents
        if node.with_data(|data: &mut NodeData| data.changed) {
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
