use std::sync::Arc;
use std::sync::RwLock;
use std::sync::Weak;
use std::fmt;
use std::collections::HashMap;
use std::collections::HashSet;

use crate::impl_::gc_node::{GcNode, Tracer};
use crate::impl_::sodium_ctx::SodiumCtx;

pub struct Node {
    pub data: Arc<NodeData>,
    pub gc_node: GcNode,
    pub sodium_ctx: SodiumCtx
}

pub struct NodeData {
    pub visited: RwLock<bool>,
    pub changed: RwLock<bool>,
    pub update: RwLock<Box<dyn FnMut()+Send+Sync>>,
    pub update_dependencies: RwLock<Vec<GcNode>>,
    pub dependencies: RwLock<Vec<Node>>,
    pub dependents: RwLock<Vec<WeakNode>>,
    pub keep_alive: RwLock<Vec<GcNode>>,
    pub sodium_ctx: SodiumCtx
}

#[derive(Clone)]
pub struct WeakNode {
    pub data: Weak<NodeData>,
    pub gc_node: GcNode,
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
    pub fn new<NAME:ToString,UPDATE:FnMut()+Send+Sync+'static>(sodium_ctx: &SodiumCtx, name:NAME, update: UPDATE, dependencies: Vec<Node>) -> Self {
        let result_forward_ref: Arc<RwLock<Option<Weak<NodeData>>>> = Arc::new(RwLock::new(None));
        let deconstructor;
        let trace;
        { // deconstructor
            let result_forward_ref = result_forward_ref.clone();
            deconstructor = move || {
                let node_data;
                {
                    let node1 = result_forward_ref.read().unwrap();
                    let node2: &Option<Weak<NodeData>> = &*node1;
                    let node3: Option<Weak<NodeData>> = node2.clone();
                    let node_data_op = node3.unwrap().upgrade();
                    if node_data_op.is_none() {
                        return;
                    }
                    node_data = node_data_op.unwrap();
                }
                let mut dependencies = Vec::new();
                {
                    let mut dependencies2 = node_data.dependencies.write().unwrap();
                    std::mem::swap(&mut *dependencies2, &mut dependencies);
                }
                let mut dependents = Vec::new();
                {
                    let mut dependents2 = node_data.dependents.write().unwrap();
                    std::mem::swap(&mut *dependents2, &mut dependents);
                }
                let mut keep_alive = Vec::new();
                {
                    let mut keep_alive2 = node_data.keep_alive.write().unwrap();
                    std::mem::swap(&mut *keep_alive2, &mut keep_alive);
                }
                {
                    let mut update_dependencies = node_data.update_dependencies.write().unwrap();
                    update_dependencies.clear();
                }
                {
                    let mut update = node_data.update.write().unwrap();
                    *update = Box::new(|| {});
                }
                for dependency in dependencies {
                    let mut dependency_dependents = dependency.data.dependents.write().unwrap();
                    dependency_dependents.retain(
                        |dependent|
                            if let Some(dependent_data) = dependent.data.upgrade() {
                                !Arc::ptr_eq(&dependent_data, &node_data)
                            } else {
                                false
                            }
                    );
                }
                for dependent in dependents {
                    if let Some(dependent_data) = dependent.data.upgrade() {
                        let mut dependent_dependencies = dependent_data.dependencies.write().unwrap();
                        dependent_dependencies.retain(|dependency| !Arc::ptr_eq(&dependency.data, &node_data));
                    }
                }
                for gc_node in keep_alive {
                    gc_node.dec_ref();
                }
                {
                    let mut node = result_forward_ref.write().unwrap();
                    *node = None;
                }
            };
        }
        { // trace
            let result_forward_ref = result_forward_ref.clone();
            trace = move |tracer: &mut Tracer| {
                let node_data_op;
                {
                    let node1 = result_forward_ref.read().unwrap();
                    let node2: &Option<Weak<NodeData>> = &*node1;
                    let node3: Option<Weak<NodeData>> = node2.clone();
                    node_data_op = node3.unwrap().upgrade();
                }
                if let Some(node_data) = node_data_op {
                    {
                        let dependencies = node_data.dependencies.read().unwrap();
                        for dependency in &*dependencies {
                            tracer(&dependency.gc_node);
                        }
                    }
                    {
                        let update_dependencies = node_data.update_dependencies.read().unwrap();
                        for update_dependency in &*update_dependencies {
                            tracer(update_dependency);
                        }
                    }
                    {
                        let keep_alive = node_data.keep_alive.read().unwrap();
                        for gc_node in &*keep_alive {
                            tracer(gc_node);
                        }
                    }
                }
            };
        }
        let result =
            Node {
                data:
                    Arc::new(NodeData {
                        visited: RwLock::new(false),
                        changed: RwLock::new(false),
                        update: RwLock::new(Box::new(update)),
                        update_dependencies: RwLock::new(Vec::new()),
                        dependencies: RwLock::new(dependencies.clone()),
                        dependents: RwLock::new(Vec::new()),
                        keep_alive: RwLock::new(Vec::new()),
                        sodium_ctx: sodium_ctx.clone()
                    }),
                gc_node: GcNode::new(
                    &sodium_ctx.gc_ctx(),
                    name.to_string(),
                    deconstructor,
                    trace
                ),
                sodium_ctx: sodium_ctx.clone()
            };
        {
            let mut result_forward_ref = result_forward_ref.write().unwrap();
            *result_forward_ref = Some(Arc::downgrade(&result.data));
        }
        for dependency in dependencies {
            let mut dependency_dependents = dependency.data.dependents.write().unwrap();
            dependency_dependents.push(Node::downgrade(&result));
        }
        sodium_ctx.inc_node_ref_count();
        sodium_ctx.inc_node_count();
        return result;
    }

    pub fn add_update_dependencies(&self, update_dependencies: Vec<GcNode>) {
        let mut update_dependencies2 = self.data.update_dependencies.write().unwrap();
        for dep in update_dependencies {
            update_dependencies2.push(dep);
        }
    }

    pub fn add_dependency(&self, dependency: Node) {
        {
            let mut dependencies = self.data.dependencies.write().unwrap();
            dependencies.push(dependency.clone());
        }
        {
            let mut dependency_dependents = dependency.data.dependents.write().unwrap();
            dependency_dependents.push(Node::downgrade(self));
        }
    }

    pub fn remove_dependency(&self, dependency: &Node) {
        {
            let mut dependencies = self.data.dependencies.write().unwrap();
            dependencies.retain(|n: &Node| !Arc::ptr_eq(&n.data, &dependency.data));
        }
        {
            let mut dependency_dependents = dependency.data.dependents.write().unwrap();
            dependency_dependents.retain(
                |n: &WeakNode|
                    if let Some(n_data) = n.data.upgrade() {
                        !Arc::ptr_eq(&n_data, &self.data)
                    } else {
                        false
                    }
            );
        }
    }

    pub fn add_keep_alive(&self, gc_node: &GcNode) {
        gc_node.inc_ref();
        let mut keep_alive = self.data.keep_alive.write().unwrap();
        keep_alive.push(gc_node.clone());
    }

    pub fn downgrade(this: &Self) -> WeakNode {
        WeakNode {
            data: Arc::downgrade(&this.data),
            gc_node: this.gc_node.clone(),
            sodium_ctx: this.sodium_ctx.clone()
        }
    }
}

impl WeakNode {
    pub fn upgrade(&self) -> Option<Node> {
        if let Some(data) = self.data.upgrade() {
            self.gc_node.inc_ref();
            self.sodium_ctx.inc_node_ref_count();
            Some(Node {
                data,
                gc_node: self.gc_node.clone(),
                sodium_ctx: self.sodium_ctx.clone()
            })
        } else {
            None
        }
    }
}

impl fmt::Debug for Node {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut node_to_id;
        {
            let mut next_id: usize = 1;
            let mut node_id_map: HashMap<*const NodeData,usize> = HashMap::new();
            node_to_id = move |node: &Node| {
                let node_data: &NodeData = &node.data;
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
                let node_data: &NodeData = &node.data;
                let node_data: *const NodeData = node_data;
                return self.visited.contains(&node_data);
            }
            pub fn mark_visitied(&mut self, node: &Node) {
                let node_data: &NodeData = &node.data;
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
            let dependencies = node.data.dependencies.read().unwrap();
            {
                let mut first: bool = true;
                for dependency in &*dependencies {
                    if !first {
                        write!(f, ", ")?;
                    } else {
                        first = false;
                    }
                    write!(f, "{}", node_to_id(&dependency))?;
                    stack.push(dependency.clone());
                }
            }
            write!(f, "]) (dependents [")?;
            let dependents = node.data.dependents.read().unwrap();
            {
                let mut first: bool = true;
                for dependent in &*dependents {
                    if !first {
                        write!(f, ", ")?;
                    } else {
                        first = false;
                    }
                    let dependent = dependent.upgrade();
                    if let Some(dependent2) = dependent {
                        write!(f, "{}", node_to_id(&dependent2))?;
                    }
                }
            }
            writeln!(f, "])")?;
        }
        return fmt::Result::Ok(());
    }
}
