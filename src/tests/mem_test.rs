use crate::CellSink;
use crate::SodiumCtx;
use crate::StreamSink;

use crate::impl_::dep::Dep;
use crate::impl_::lambda::lambda0;
use crate::impl_::node::Node as NodeImpl;
use crate::impl_::node::NodeData;
use crate::impl_::stream_sink::StreamSink as StreamSinkImpl;

#[test]
fn node_mem() {
    let sodium_ctx = crate::impl_::sodium_ctx::SodiumCtx::new();
    {
        let ss = StreamSinkImpl::<i32>::new(&sodium_ctx);
        let node = NodeImpl::new(
            &sodium_ctx,
            || {},
            vec![]
        );
        let node2 = NodeImpl::downgrade(&node);
        let node3 = node.clone();
        node.with_data(
            |data: &mut NodeData|
                data.update = Box::new(lambda0(
                    move || {
                        let _node = node2.upgrade().unwrap();
                        //node2.clone();
                    },
                    vec![Dep::new(node3)]
                ))
        )
    }
    sodium_ctx.collect_cycles();
    let node_count = sodium_ctx.node_count();
    let node_ref_count = sodium_ctx.node_ref_count();
    println!();
    println!("node_count {}", node_count);
    println!("node_ref_count {}", node_ref_count);
    assert_eq!(node_count, 0);
}

#[test]
fn map_s_mem() {
    let sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &sodium_ctx;
    {
        let ss1: StreamSink<i32> = sodium_ctx.new_stream_sink();
        let s1 = ss1.stream();
        let _s2 = s1.map_to(5);
        let l = _s2.listen_weak(|_:&u32| {});
        l.unlisten();
    }
    sodium_ctx.impl_.collect_cycles();
    let node_count = sodium_ctx.impl_.node_count();
    let node_ref_count = sodium_ctx.impl_.node_ref_count();
    println!();
    println!("node_count {}", node_count);
    println!("node_ref_count {}", node_ref_count);
    assert_eq!(node_count, 0);
}

#[test]
fn map_c_mem() {
    let sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &sodium_ctx;
    {
        let cs1: CellSink<i32> = sodium_ctx.new_cell_sink(3);
        let c1 = cs1.cell();
        let _c2 = c1.map(|a:&i32| a + 5);
        let l = _c2.listen_weak(|_:&i32| {});
        l.unlisten();
    }
    sodium_ctx.impl_.collect_cycles();
    let node_count = sodium_ctx.impl_.node_count();
    let node_ref_count = sodium_ctx.impl_.node_ref_count();
    println!();
    println!("node_count {}", node_count);
    println!("node_ref_count {}", node_ref_count);
    assert_eq!(node_count, 0);
}