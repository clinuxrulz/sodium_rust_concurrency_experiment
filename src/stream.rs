use std::sync::Arc;
use std::sync::Mutex;
use crate::node::Node;
use crate::listener::Listener;

pub struct Stream<A> {
    pub data: Arc<Mutex<StreamData<A>>>
}

impl<A> Clone for Stream<A> {
    fn clone(&self) -> Self {
        Stream {
            data: self.data.clone()
        }
    }
}

pub struct StreamData<A> {
    pub firing_op: Option<A>,
    pub node: Node
}

impl<A:Send+'static> Stream<A> {
    pub fn new() -> Stream<A> {
        Stream {
            data: Arc::new(Mutex::new(StreamData {
                firing_op: None,
                node: Node::new(|| {}, Vec::new())
            }))
        }
    }

    pub fn node(&self) -> Node {
        self.with_data(|data: &mut StreamData<A>| data.node.clone())
    }

    pub fn listen<K: FnMut(&A)+Send+'static>(&self, mut k: K) -> Listener {
        let self_ = self.clone();
        let node =
            Node::new(
                move || {
                    self_.with_data(|data: &mut StreamData<A>| {
                        for firing in &data.firing_op {
                            k(firing)
                        }
                    });
                },
                vec![self.node()]
            );
        Listener::new(node)
    }

    pub fn with_data<R,K:FnOnce(&mut StreamData<A>)->R>(&self, k: K) -> R {
        let mut l = self.data.lock();
        let data: &mut StreamData<A> = l.as_mut().unwrap();
        k(data)
    }
}
