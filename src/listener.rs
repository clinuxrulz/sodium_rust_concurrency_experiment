use crate::node::Node;

#[derive(Clone)]
pub struct Listener {
    pub node: Node
}

impl Listener {
    pub fn new(node: Node) -> Listener {
        Listener { node }
    }
}
