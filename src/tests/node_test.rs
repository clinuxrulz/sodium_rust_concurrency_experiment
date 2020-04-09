
use crate::impl_::sodium_ctx::SodiumCtx;
use crate::impl_::node::Node;

#[test]
fn node_mem_1() {
    let sodium_ctx = SodiumCtx::new();
    {
        let node1 =
            Node::new(
                &sodium_ctx,
                || {},
                vec![]
            );
        let node2 =
            Node::new(
                &sodium_ctx,
                || {},
                vec![node1.clone()]
            );
        let node3 =
            Node::new(
                &sodium_ctx,
                || {},
                vec![node2.clone()]
            );
        node1.add_dependency(node3);
    }
    assert_memory_freed(&sodium_ctx);
}

pub fn assert_memory_freed(sodium_ctx: &SodiumCtx) {
    sodium_ctx.collect_cycles();
    let node_count = sodium_ctx.node_count();
    let node_ref_count = sodium_ctx.node_ref_count();
    println!();
    println!("node_count {}", node_count);
    println!("node_ref_count {}", node_ref_count);
    assert_eq!(node_count, 0);
}
