use std::sync::Arc;
use std::sync::Mutex;
use std::sync::Weak;
use std::fmt;
use std::collections::HashMap;
use std::collections::HashSet;

use crate::impl_::sodium_ctx::SodiumCtx;
use crate::impl_::sodium_ctx::SodiumCtxData;

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
    pub update_dependencies: Vec<WeakNode>,
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
        self.sodium_ctx.inc_node_ref_count();
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
        self.sodium_ctx.dec_node_ref_count();
        let ref_count = self.dec_ref_count();
        if ref_count > 0 {
            // TODO: Use buffered flag here per node rather than collecting_cycles flag.
            let collecting_cycles =
                self.sodium_ctx.with_data(|data: &mut SodiumCtxData| data.collecting_cycles);
            if !collecting_cycles {
                self.sodium_ctx.add_gc_root(self);
            }
        } else {
            let dependencies = self.with_data(|data: &mut NodeData| data.dependencies.clone());
            for dependency in dependencies {
                self.remove_dependency(&dependency);
            }
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
                            update_dependencies: Vec::new(),
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
        sodium_ctx.inc_node_ref_count();
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

    pub fn add_update_dependencies(&self, update_dependencies: Vec<Node>) {
        self.with_data(move |data: &mut NodeData| {
            for dep in update_dependencies {
                data.update_dependencies.push(Node::downgrade(&dep));
            }
        });
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

impl fmt::Debug for Node {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut node_to_id;
        {
            let mut next_id: usize = 1;
            let mut node_id_map: HashMap<*const NodeData,usize> = HashMap::new();
            node_to_id = move |node: &Node| {
                let l = node.data.lock();
                let node_data: &NodeData = l.as_ref().unwrap();
                let node_data: *const NodeData = node_data;
                let existing_op = node_id_map.get(&node_data).map(|x| x.clone());
                let node_id;
                if let Some(existing) = existing_op {
                    node_id = existing;
                } else {
                    node_id = next_id;
                    next_id = next_id + 1;
                    node_id_map.insert(node_data, node_id);
                }
                return format!("N{}", node_id);
            };
        }
        struct Util {
            visited: HashSet<*const NodeData>
        }
        impl Util {
            pub fn new() -> Util {
                Util {
                    visited: HashSet::new()
                }
            }
            pub fn is_visited(&self, node: &Node) -> bool {
                let l = node.data.lock();
                let node_data: &NodeData = l.as_ref().unwrap();
                let node_data: *const NodeData = node_data;
                return self.visited.contains(&node_data);
            }
            pub fn mark_visitied(&mut self, node: &Node) {
                let l = node.data.lock();
                let node_data: &NodeData = l.as_ref().unwrap();
                let node_data: *const NodeData = node_data;
                self.visited.insert(node_data);
            }
        }
        let mut util = Util::new();
        let mut stack = vec![self.clone()];
        loop {
            let node_op = stack.pop();
            if node_op.is_none() {
                break;
            }
            let node = node_op.unwrap();
            let node = &node;
            if util.is_visited(node) {
                continue;
            }
            util.mark_visitied(node);
            write!(f, "(Node {} (dependencies [", node_to_id(node))?;
            let dependencies = node.with_data(|data: &mut NodeData| data.dependencies.clone());
            {
                let mut first: bool = true;
                for dependency in dependencies {
                    if !first {
                        write!(f, ", ")?;
                    } else {
                        first = false;
                    }
                    write!(f, "{}", node_to_id(&dependency))?;
                    stack.push(dependency);
                }
            }
            write!(f, "]) (dependents [")?;
            let dependents: Vec<Node> =
                node.with_data(|data: &mut NodeData|
                    data
                        .dependents
                        .iter()
                        .flat_map(|dependent| dependent.upgrade())
                        .collect()
                );
            {
                let mut first: bool = true;
                for dependent in dependents {
                    if !first {
                        write!(f, ", ")?;
                    } else {
                        first = false;
                    }
                    write!(f, "{}", node_to_id(&dependent))?;
                }
            }
            writeln!(f, "])")?;
        }
        return fmt::Result::Ok(());
    }
}

impl WeakNode {
    pub fn upgrade(&self) -> Option<Node> {
        let node_op = self.data.upgrade().map(|data| Node { data, sodium_ctx: self.sodium_ctx.clone(), gc_data: self.gc_data.clone() });
        if let Some(ref node) = &node_op {
            node.sodium_ctx.inc_node_ref_count();
            node.inc_ref_count();
        }
        node_op
    }
}
