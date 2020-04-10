use crate::impl_::gc_node::GcCtx;
use crate::impl_::listener::Listener;
use crate::impl_::node::{Node, NodeData, WeakNode};

use std::mem;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;

#[derive(Clone)]
pub struct SodiumCtx {
    gc_ctx: GcCtx,
    data: Arc<Mutex<SodiumCtxData>>,
    node_count: Arc<Mutex<usize>>,
    node_ref_count: Arc<Mutex<usize>>,
    threaded_mode: Arc<ThreadedMode>
}

pub struct SodiumCtxData {
    pub null_node_op: Option<Node>,
    pub changed_nodes: Vec<Node>,
    pub visited_nodes: Vec<Node>,
    pub transaction_depth: u32,
    pub pre_post: Vec<Box<dyn FnMut()+Send>>,
    pub post: Vec<Box<dyn FnMut()+Send>>,
    pub keep_alive: Vec<Listener>,
    pub collecting_cycles: bool,
    pub allow_add_roots: bool
}

pub struct ThreadedMode {
    pub spawner: ThreadSpawner
}

pub struct ThreadSpawner {
    pub spawn_fn: Box<dyn Fn(Box<dyn FnOnce()+Send>)->ThreadJoiner<()>+Send+Sync>
}

pub struct ThreadJoiner<R> {
    pub join_fn: Box<dyn FnOnce()->R+Send>
}

impl ThreadedMode {
    pub fn spawn<R:Send+'static,F:FnOnce()->R+Send+'static>(&self, f: F) -> ThreadJoiner<R> {
        let r: Arc<Mutex<Option<R>>> = Arc::new(Mutex::new(None));
        let thread_joiner;
        {
            let r = r.clone();
            thread_joiner = (self.spawner.spawn_fn)(Box::new(move || {
                let r2 = f();
                let mut l = r.lock();
                let r: &mut Option<R> = l.as_mut().unwrap();
                *r = Some(r2);
            }));
        }
        return ThreadJoiner {
            join_fn: Box::new(move || {
                (thread_joiner.join_fn)();
                let mut l = r.lock();
                let r: &mut Option<R> = l.as_mut().unwrap();
                let mut r2: Option<R> = None;
                mem::swap(r, &mut r2);
                r2.unwrap()
            })
        };
    }
}

impl<R> ThreadJoiner<R> {
    pub fn join(self) -> R {
        (self.join_fn)()
    }
}

pub fn single_threaded_mode() -> ThreadedMode {
    ThreadedMode {
        spawner: ThreadSpawner {
            spawn_fn: Box::new(|callback| {
                callback();
                ThreadJoiner {
                    join_fn: Box::new(|| {})
                }
            })
        }
    }
}

pub fn simple_threaded_mode() -> ThreadedMode {
    ThreadedMode {
        spawner: ThreadSpawner {
            spawn_fn: Box::new(|callback| {
                let h = thread::spawn(callback);
                ThreadJoiner {
                    join_fn: Box::new(move || { h.join().unwrap(); })
                }
            })
        }
    }
}

// TODO:
//pub fn thread_pool_threaded_mode(num_threads: usize) -> ThreadedMode

impl SodiumCtx {
    pub fn new() -> SodiumCtx {
        SodiumCtx {
            gc_ctx: GcCtx::new(),
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
                        collecting_cycles: false,
                        allow_add_roots: true
                    }
                )),
            node_count: Arc::new(Mutex::new(0)),
            node_ref_count: Arc::new(Mutex::new(0)),
            threaded_mode: Arc::new(single_threaded_mode())
        }
    }

    pub fn gc_ctx(&self) -> GcCtx {
        self.gc_ctx.clone()
    }

    pub fn null_node(&self) -> Node {
        self.with_data(|data: &mut SodiumCtxData| {
            if let Some(ref null_node) = data.null_node_op {
                return null_node.clone();
            }
            let null_node = Node::new(self, || {}, Vec::new());
            // Do not include null_node against total node count
            self.dec_node_ref_count();
            self.dec_node_count();
            //
            data.null_node_op = Some(null_node.clone());
            return null_node;
        })
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
                    .flat_map(|node: &WeakNode| node.upgrade())
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

    pub fn node_count(&self) -> usize {
        self.with_node_count(|node_count: &mut usize| *node_count)
    }

    pub fn inc_node_count(&self) {
        self.with_node_count(|node_count: &mut usize| *node_count = *node_count + 1);
    }

    pub fn dec_node_count(&self) {
        self.with_node_count(|node_count: &mut usize| *node_count = *node_count - 1);
    }

    pub fn with_node_count<R,K:FnOnce(&mut usize)->R>(&self, k: K) -> R {
        let mut l = self.node_count.lock();
        let node_count: &mut usize = l.as_mut().unwrap();
        k(node_count)
    }

    pub fn node_ref_count(&self) -> usize {
        self.with_node_ref_count(|node_ref_count: &mut usize| *node_ref_count)
    }

    pub fn inc_node_ref_count(&self) {
        self.with_node_ref_count(|node_ref_count: &mut usize| *node_ref_count = *node_ref_count + 1);
    }

    pub fn dec_node_ref_count(&self) {
        self.with_node_ref_count(|node_ref_count: &mut usize| *node_ref_count = *node_ref_count - 1);
    }

    pub fn with_node_ref_count<R,K:FnOnce(&mut usize)->R>(&self, k: K) -> R {
        let mut l = self.node_ref_count.lock();
        let node_ref_count: &mut usize = l.as_mut().unwrap();
        k(node_ref_count)
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
        // gc
        self.collect_cycles()
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
            handle = self.threaded_mode.spawn(move || {
                for dependency in &dependencies {
                    let visit_it = dependency.with_data(|data: &mut NodeData| !data.visited);
                    if visit_it {
                        _self.update_node(dependency);
                    }
                }
            });
        }
        handle.join();
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
            {
                let dependents = dependents.clone();
                let _self = self.clone();
                for dependent in dependents {
                    if let Some(dependent2) = dependent.upgrade() {
                        _self.update_node(&dependent2);
                    }
                }
            }
        }
    }

    pub fn collect_cycles(&self) {
        self.gc_ctx.collect_cycles();
    }
}
