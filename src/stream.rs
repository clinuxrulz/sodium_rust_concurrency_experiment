use std::sync::Arc;
use std::sync::Mutex;
use crate::node::Node;

#[derive(Clone)]
pub struct Stream<A> {
    pub data: Arc<Mutex<StreamData<A>>>
}

pub struct StreamData<A> {
    pub firing_op: Arc<Mutex<Option<A>>>,
    pub node: Node
}

impl<A> Stream<A> {
    pub fn new() -> Stream<A> {
        Stream {
            data: Arc::new(Mutex::new(StreamData {
                firing_op: Arc::new(Mutex::new(None)),
                node: Node::new(|| {}, Vec::new())
            }))
        }
    }
}
