use crate::node::Node;

#[derive(Clone)]
pub struct Listener {
    pub node_op: Option<Node>
}

impl Listener {
    pub fn new(node: Node) -> Listener {
        Listener { node_op: Some(node) }
    }

    pub fn unlisten(&mut self) {
        self.node_op = None;
    }
}
