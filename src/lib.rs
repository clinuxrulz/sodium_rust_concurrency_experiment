mod node;
mod sodium_ctx;
mod stream;
mod stream_sink;

#[cfg(test)]
mod tests {
    use crate::node::Node;
    use crate::node::NodeData;

    #[test]
    fn node() {
        let node1 = Node::new(|| {}, vec![]);
        let node2 = Node::new(|| {}, vec![node1.clone()]);
        let node3 = Node::new(|| {}, vec![node1]);
        let node4 = Node::new(|| {}, vec![node2, node3.clone()]);
        node4.remove_dependency(&node3);
        //node4.with_data(|data: &mut NodeData| {
        //});
    }

    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
