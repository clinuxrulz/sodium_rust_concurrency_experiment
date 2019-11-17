use std::sync::Arc;
use std::sync::Mutex;
use std::sync::Weak;

use crate::impl_::sodium_ctx::SodiumCtx;

pub struct Node {
    pub data: Arc<Mutex<NodeData>>,
    pub gc_data: Arc<Mutex<NodeGcData>>,
    pub sodium_ctx: SodiumCtx
}

#[derive(Clone)]
pub struct WeakNode {
    pub data: Weak<Mutex<NodeData>>,
    pub gc_data: Arc<Mutex<NodeGcData>>,
    pub sodium_ctx: SodiumCtx
}

pub struct NodeData {
    pub visited: bool,
    pub changed: bool,
    pub update: Box<dyn FnMut()+Send>,
    pub dependencies: Vec<Node>,
    pub dependents: Vec<WeakNode>,
    pub keep_alive: Vec<Node>,
    pub sodium_ctx: SodiumCtx
}

pub struct NodeGcData {
    pub ref_count: usize,
    pub visited: bool
}

impl Clone for Node {
    fn clone(&self) -> Self {
        self.inc_ref_count();
        Node {
            data: self.data.clone(),
            sodium_ctx: self.sodium_ctx.clone(),
            gc_data: self.gc_data.clone()
        }
    }
}

impl Drop for Node {
    fn drop(&mut self) {
        let ref_count = self.dec_ref_count();
        if ref_count > 0 {
            self.sodium_ctx.add_gc_root(self);
        }
    }
}

impl Drop for NodeData {
    fn drop(&mut self) {
        self.sodium_ctx.dec_node_count();
    }
}

impl Node {
    pub fn new<UPDATE:FnMut()+Send+'static>(sodium_ctx: &SodiumCtx, update: UPDATE, dependencies: Vec<Node>) -> Self {
        let result =
            Node {
                data:
                    Arc::new(Mutex::new(
                        NodeData {
                            visited: false,
                            changed: false,
                            update: Box::new(update),
                            dependencies: dependencies.clone(),
                            dependents: Vec::new(),
                            keep_alive: Vec::new(),
                            sodium_ctx: sodium_ctx.clone()
                        }
                    )),
                gc_data: Arc::new(Mutex::new(
                    NodeGcData {
                        ref_count: 1,
                        visited: false
                    }
                )),
                sodium_ctx: sodium_ctx.clone()
            };
        for dependency in dependencies {
            let mut l = dependency.data.lock();
            let dependency2: &mut NodeData = l.as_mut().unwrap();
            dependency2.dependents.push(Node::downgrade(&result));
        }
        sodium_ctx.inc_node_count();
        return result;
    }

    pub fn with_gc_data<R,K:FnOnce(&mut NodeGcData)->R>(&self, k: K) -> R {
        let mut l = self.gc_data.lock();
        let gc_data: &mut NodeGcData = l.as_mut().unwrap();
        k(gc_data)
    }

    pub fn ref_count(&self) -> usize {
        self.with_gc_data(|data: &mut NodeGcData| data.ref_count)
    }

    pub fn inc_ref_count(&self) -> usize {
        self.with_gc_data(|data: &mut NodeGcData| {
            data.ref_count = data.ref_count + 1;
            data.ref_count
        })
    }

    pub fn dec_ref_count(&self) -> usize {
        self.with_gc_data(|data: &mut NodeGcData| {
            data.ref_count = data.ref_count - 1;
            data.ref_count
        })
    }

    pub fn downgrade(this: &Self) -> WeakNode {
        WeakNode {
            data: Arc::downgrade(&this.data),
            gc_data: this.gc_data.clone(),
            sodium_ctx: this.sodium_ctx.clone()
        }
    }

    pub fn add_dependency(&self, dependency: Node) {
        let dependency2 = dependency.clone();
        self.with_data(move |data: &mut NodeData| {
            data.dependencies.push(dependency2);
        });
        dependency.with_data(|data: &mut NodeData| {
            data.dependents.push(Node::downgrade(self));
        });
    }

    pub fn remove_dependency(&self, dependency: &Node) {
        self.with_data(|data: &mut NodeData| {
            data.dependencies.retain(|n: &Node| !Arc::ptr_eq(&n.data, &dependency.data));
        });
        dependency.with_data(|data: &mut NodeData| {
            data.dependents.retain(|n: &WeakNode| {
                if let Some(n) = n.upgrade() {
                    !Arc::ptr_eq(&n.data, &self.data)
                } else {
                    false
                }
            })
        });
    }

    pub fn add_keep_alive(&self, node: &Node) {
        self.with_data(|data: &mut NodeData| {
            data.keep_alive.push(node.clone());
        });
    }

    pub fn with_data<R,K:FnOnce(&mut NodeData)->R>(&self, k: K) -> R {
        let mut l = self.data.lock();
        let data: &mut NodeData = l.as_mut().unwrap();
        k(data)
    }
}

impl WeakNode {
    pub fn upgrade(&self) -> Option<Node> {
        let node_op = self.data.upgrade().map(|data| Node { data, sodium_ctx: self.sodium_ctx.clone(), gc_data: self.gc_data.clone() });
        if let Some(ref node) = &node_op {
            node.inc_ref_count();
        }
        node_op
    }
}
