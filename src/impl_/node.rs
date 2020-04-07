use std::sync::Arc;
use std::sync::Mutex;
use std::fmt;
use std::collections::HashMap;
use std::collections::HashSet;

use crate::impl_::gc_node::{GcNode, Tracer};
use crate::impl_::sodium_ctx::SodiumCtx;

pub struct Node {
    pub data: Arc<Mutex<NodeData>>,
    pub gc_node: GcNode,
    pub sodium_ctx: SodiumCtx
}

pub struct NodeData {
    pub visited: bool,
    pub changed: bool,
    pub update: Box<dyn FnMut()+Send>,
    pub update_dependencies: Vec<Node>,
    pub dependencies: Vec<Node>,
    pub dependents: Vec<Node>,
    pub keep_alive: Vec<Node>,
    pub sodium_ctx: SodiumCtx
}

impl Clone for Node {
    fn clone(&self) -> Self {
        self.sodium_ctx.inc_node_ref_count();
        self.gc_node.inc_ref();
        Node {
            data: self.data.clone(),
            gc_node: self.gc_node.clone(),
            sodium_ctx: self.sodium_ctx.clone()
        }
    }
}

impl Drop for Node {
    fn drop(&mut self) {
        self.sodium_ctx.dec_node_ref_count();
        self.gc_node.dec_ref();
    }
}

impl Drop for NodeData {
    fn drop(&mut self) {
        self.sodium_ctx.dec_node_count();
    }
}

impl Node {
    pub fn new<UPDATE:FnMut()+Send+'static>(sodium_ctx: &SodiumCtx, update: UPDATE, dependencies: Vec<Node>) -> Self {
        let result_forward_ref: Arc<Mutex<Option<Node>>> = Arc::new(Mutex::new(None));
        let deconstructor;
        let trace;
        { // deconstructor
            let result_forward_ref = result_forward_ref.clone();
            deconstructor = move || {
                let node;
                {
                    let l = result_forward_ref.lock();
                    let node2 = l.as_ref().unwrap();
                    let node2: &Option<Node> = &node2;
                    node = node2.clone().unwrap();
                }
                let mut dependencies = Vec::new();
                node.with_data(|data: &mut NodeData| {
                    std::mem::swap(&mut data.dependencies, &mut &mut dependencies);
                    data.dependents.clear();
                    data.update = Box::new(|| {});
                });
                for dependency in dependencies {
                    let mut l = dependency.data.lock();
                    let dependency = l.as_mut().unwrap();
                    dependency.dependents.retain(|dependent| !Arc::ptr_eq(&dependent.data, &node.data));
                }
            };
        }
        { // trace
            let result_forward_ref = result_forward_ref.clone();
            trace = move |tracer: &mut Tracer| {
                let node;
                {
                    let l = result_forward_ref.lock();
                    let node2 = l.as_ref().unwrap();
                    let node2: &Option<Node> = &node2;
                    node = node2.clone().unwrap();
                }
                let mut dependencies = Vec::new();
                node.with_data(|data: &mut NodeData|
                    std::mem::swap(&mut data.dependencies, &mut &mut dependencies)
                );
                for dependency in &dependencies {
                    let gc_node =
                        dependency.with_data(|data: &mut NodeData|
                            dependency.gc_node.clone()
                        );
                    tracer(&gc_node);
                }
                node.with_data(|data: &mut NodeData|
                    std::mem::swap(&mut data.dependencies, &mut &mut dependencies)
                );
            };
        }
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
                gc_node: GcNode::new(
                    &sodium_ctx.gc_ctx(),
                    deconstructor,
                    trace
                ),
                sodium_ctx: sodium_ctx.clone()
            };
        {
            let result = result.clone();
            let mut l = result_forward_ref.lock();
            let mut result_forward_ref = l.as_mut().unwrap();
            let result_forward_ref: &mut Option<Node> = &mut result_forward_ref;
            *result_forward_ref = Some(result);
        }
        for dependency in dependencies {
            let mut l = dependency.data.lock();
            let dependency2: &mut NodeData = l.as_mut().unwrap();
            dependency2.dependents.push(result.clone());
        }
        sodium_ctx.inc_node_ref_count();
        sodium_ctx.inc_node_count();
        return result;
    }

    pub fn add_update_dependencies(&self, update_dependencies: Vec<Node>) {
        self.with_data(move |data: &mut NodeData| {
            for dep in update_dependencies {
                data.update_dependencies.push(dep);
            }
        });
    }

    pub fn add_dependency(&self, dependency: Node) {
        let dependency2 = dependency.clone();
        self.with_data(move |data: &mut NodeData| {
            data.dependencies.push(dependency2);
        });
        dependency.with_data(|data: &mut NodeData| {
            data.dependents.push(self.clone());
        });
    }

    pub fn remove_dependency(&self, dependency: &Node) {
        self.with_data(|data: &mut NodeData| {
            data.dependencies.retain(|n: &Node| !Arc::ptr_eq(&n.data, &dependency.data));
        });
        dependency.with_data(|data: &mut NodeData| {
            data.dependents.retain(|n: &Node| {
                !Arc::ptr_eq(&n.data, &self.data)
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
                    data.dependents.clone()
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
