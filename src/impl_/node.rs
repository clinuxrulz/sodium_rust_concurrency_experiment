use crate::impl_::dep::Dep;
use crate::impl_::lambda::{IsLambda0};
use crate::impl_::sodium_ctx::SodiumCtx;

use std::cell::{Ref, RefCell, RefMut};
use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt;
use bacon_rajan_cc::{Cc, Trace, Tracer, Weak};

pub struct Node {
    pub data: Cc<RefCell<NodeData>>,
    pub sodium_ctx: SodiumCtx
}

impl Trace for Node {
    fn trace(&self, tracer: &mut Tracer) {
        tracer(&self.data);
    }
}

#[derive(Clone)]
pub struct WeakNode {
    pub data: Weak<RefCell<NodeData>>,
    pub sodium_ctx: SodiumCtx
}

pub struct NodeData {
    pub visited: bool,
    pub changed: bool,
    pub update: Box<dyn IsLambda0<()>>,
    pub dependencies: Vec<Node>,
    pub dependents: Vec<WeakNode>,
    pub keep_alive: Vec<Dep>,
    pub sodium_ctx: SodiumCtx
}

impl Trace for NodeData {
    fn trace(&self, tracer: &mut Tracer) {
        {
            let deps_op = self.update.deps_op();
            if let Some(deps) = deps_op {
                for dep in deps {
                    dep.trace(tracer);
                }
            }
        }
        for node in &self.dependencies {
            node.trace(tracer);
        }
        /*
        for node in &self.keep_alive {
            node.trace(tracer);
        }*/
    }
}

impl Clone for Node {
    fn clone(&self) -> Self {
        self.sodium_ctx.inc_node_ref_count();
        Node {
            data: self.data.clone(),
            sodium_ctx: self.sodium_ctx.clone(),
        }
    }
}

impl Drop for Node {
    fn drop(&mut self) {
        self.sodium_ctx.dec_node_ref_count();
    }
}

impl Drop for NodeData {
    fn drop(&mut self) {
        self.sodium_ctx.dec_node_count();
    }
}

impl Node {
    pub fn new<UPDATE:IsLambda0<()>+'static>(sodium_ctx: &SodiumCtx, update: UPDATE, dependencies: Vec<Node>) -> Self {
        let result =
            Node {
                data:
                    Cc::new(RefCell::new(
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
                sodium_ctx: sodium_ctx.clone()
            };
        for dependency in dependencies {
            let mut dependency2: RefMut<NodeData> = dependency.data.borrow_mut();
            dependency2.dependents.push(Node::downgrade(&result));
        }
        sodium_ctx.inc_node_ref_count();
        sodium_ctx.inc_node_count();
        return result;
    }

    pub fn downgrade(this: &Self) -> WeakNode {
        WeakNode {
            data: Cc::downgrade(&this.data),
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
            let mem_loc_str = format!("{:p}", dependency.data);
            for i in 0..data.dependencies.len() {
                let match_ = format!("{:p}", data.dependencies[i].data) == mem_loc_str;
                if match_ {
                    data.dependencies.remove(i);
                    break;
                }
            }
        });
        dependency.with_data(|data: &mut NodeData| {
            let mem_loc_str = format!("{:p}", self.data);
            for i in 0..data.dependents.len() {
                let n_op = data.dependents[i].upgrade();
                if let Some(n) = n_op {
                    if format!("{:p}", n.data) == mem_loc_str {
                        data.dependents.remove(i);
                        break;
                    }
                }
            }
        });
    }

    pub fn add_keep_alive(&self, dep: Dep) {
        self.with_data(|data: &mut NodeData| {
            data.keep_alive.push(dep);
        });
    }

    pub fn add_keep_alives(&self, deps: Vec<Dep>) {
        self.with_data(|data: &mut NodeData| {
            for dep in deps {
                data.keep_alive.push(dep);
            }
        });
    }

    pub fn with_data<R,K:FnOnce(&mut NodeData)->R>(&self, k: K) -> R {
        let mut l: RefMut<NodeData> = self.data.borrow_mut();
        let data: &mut NodeData = &mut l;
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
                let l: Ref<NodeData> = node.data.borrow();
                let node_data: &NodeData = &l;
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
                let l: Ref<NodeData> = node.data.borrow();
                let node_data: &NodeData = &l;
                let node_data: *const NodeData = node_data;
                return self.visited.contains(&node_data);
            }
            pub fn mark_visitied(&mut self, node: &Node) {
                let l = node.data.borrow();
                let node_data: &NodeData = &l;
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
        let node_op = self.data.upgrade().map(|data| Node { data, sodium_ctx: self.sodium_ctx.clone() });
        if let Some(ref node) = &node_op {
            node.sodium_ctx.inc_node_ref_count();
        }
        node_op
    }
}
