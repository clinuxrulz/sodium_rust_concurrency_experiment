use crate::sodium_ctx::SodiumCtx;

use std::sync::Arc;
use std::sync::Mutex;
use std::sync::Weak;

#[derive(Clone)]
pub struct Node {
    data: Arc<Mutex<NodeData>>
}

#[derive(Clone)]
pub struct WeakNode {
    data: Weak<Mutex<NodeData>>
}

pub struct NodeData {
    pub visited: bool,
    pub changed: bool,
    pub update: Box<dyn FnMut()+Send>,
    pub dependencies: Vec<Node>,
    pub dependents: Vec<WeakNode>
}

impl Node {
    pub fn new<UPDATE:FnMut()+Send+'static>(update: UPDATE, dependencies: Vec<Node>) -> Self {
        let result =
            Node {
                data:
                    Arc::new(Mutex::new(
                        NodeData {
                            visited: false,
                            changed: false,
                            update: Box::new(update),
                            dependencies: dependencies.clone(),
                            dependents: Vec::new()
                        }
                    ))
            };
        for dependency in dependencies {
            let mut l = dependency.data.lock();
            let dependency2: &mut NodeData = l.as_mut().unwrap();
            dependency2.dependents.push(Node::downgrade(&result));
        }
        return result;
    }

    pub fn downgrade(this: &Self) -> WeakNode {
        WeakNode {
            data: Arc::downgrade(&this.data)
        }
    }

    pub fn add_dependency(&self, dependency: Node) {
        self.with_data(|data: &mut NodeData| {
            data.dependencies.push(dependency.clone());
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

    pub fn with_data<R,K:FnOnce(&mut NodeData)->R>(&self, k: K) -> R {
        let mut l = self.data.lock();
        let data: &mut NodeData = l.as_mut().unwrap();
        k(data)
    }
}

impl WeakNode {
    pub fn upgrade(&self) -> Option<Node> {
        self.data.upgrade().map(|data| Node { data })
    }
}
