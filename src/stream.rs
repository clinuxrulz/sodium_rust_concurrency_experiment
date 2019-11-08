use crate::cell::Cell;
use crate::impl_::stream::Stream as StreamImpl;
use crate::impl_::node::Node;
use crate::impl_::lambda::IsLambda1;
use crate::impl_::lambda::IsLambda2;
use crate::sodium_ctx::SodiumCtx;

pub struct Stream<A> {
    pub impl_: StreamImpl<A>
}

impl<A> Clone for Stream<A> {
    fn clone(&self) -> Self {
        Stream {
            impl_: self.impl_.clone()
        }
    }
}

impl<A:Clone+Send+'static> Stream<A> {
    pub fn new(sodium_ctx: &SodiumCtx) -> Stream<A> {
        Stream {
            impl_: StreamImpl::new(&sodium_ctx.impl_)
        }
    }

    // use as dependency to lambda1, lambda2, etc.
    pub fn node(&self) -> Node {
        self.impl_.node()
    }

    pub fn snapshot<B:Clone+Send+'static,C:Clone+Send+'static,FN:IsLambda2<A,B,C>+Send+'static>(&self, cb: &Cell<B>, mut f: FN) -> Stream<C> {
        Stream { impl_: self.impl_.snapshot(&cb.impl_, f) }
    }
}