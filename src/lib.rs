mod listener;
mod node;
mod sodium_ctx;
mod stream;
mod stream_sink;

#[cfg(test)]
mod tests {
    use crate::sodium_ctx::SodiumCtx;
    use crate::node::Node;
    use crate::node::NodeData;
    use crate::stream_sink::StreamSink;

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
    fn stream_sink() {
        let sodium_ctx = SodiumCtx::new();
        let s : StreamSink<i32> = StreamSink::new();
        let l = s.to_stream().listen(|a: &i32| {
            println!("{}", a);
        });
        s.send(&sodium_ctx, 1);
        s.send(&sodium_ctx, 2);
    }

    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
