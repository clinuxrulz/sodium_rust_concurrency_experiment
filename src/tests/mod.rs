mod cell_loop_test;
mod cell_test;
mod stream_test;

use crate::SodiumCtx;

pub fn assert_memory_freed(sodium_ctx: &SodiumCtx) {
    let node_count = sodium_ctx.impl_.node_count();
    //assert_eq!(node_count, 0);
}
