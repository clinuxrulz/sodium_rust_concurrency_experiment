use crate::impl_::cell::Cell as CellImpl;
use crate::sodium_ctx::SodiumCtx;
use crate::impl_::node::Node;

pub struct Cell<A> {
    pub impl_: CellImpl<A>
}
