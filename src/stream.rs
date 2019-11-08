use crate::cell::Cell;
use crate::impl_::stream::Stream as StreamImpl;
use crate::impl_::node::Node;
use crate::impl_::lambda::IsLambda1;
use crate::impl_::lambda::IsLambda2;
use crate::listener::Listener;
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

    pub fn snapshot<B:Clone+Send+'static,C:Clone+Send+'static,FN:IsLambda2<A,B,C>+Send+'static>(&self, cb: &Cell<B>, f: FN) -> Stream<C> {
        Stream { impl_: self.impl_.snapshot(&cb.impl_, f) }
    }

    pub fn snapshot1<B:Send+Clone+'static>(&self, cb: &Cell<B>) -> Stream<B> {
        self.snapshot(cb, |_a: &A, b: &B| b.clone())
    }

    pub fn map<B:Send+Clone+'static,FN:IsLambda1<A,B>+Send+'static>(&self, f: FN) -> Stream<B> {
        Stream { impl_: self.impl_.map(f) }
    }

    pub fn filter<PRED:IsLambda1<A,bool>+Send+'static>(&self, pred: PRED) -> Stream<A> {
        Stream { impl_: self.impl_.filter(pred) }
    }

    pub fn or_else(&self, s2: &Stream<A>) -> Stream<A> {
        self.merge(s2, |lhs:&A, _rhs:&A| lhs.clone())
    }

    pub fn merge<FN:IsLambda2<A,A,A>+Send+'static>(&self, s2: &Stream<A>, f: FN) -> Stream<A> {
        Stream { impl_: self.impl_.merge(&s2.impl_, f) }
    }

    pub fn hold(&self, a: A) -> Cell<A> {
        Cell { impl_: self.impl_.hold(a) }
    }

    pub fn listen_weak<K:IsLambda1<A,()>+Send+'static>(&self, k: K) -> Listener {
        Listener { impl_: self.impl_.listen_weak(k) }
    }

    pub fn listen<K:IsLambda1<A,()>+Send+'static>(&self, k: K) -> Listener {
        Listener { impl_: self.impl_.listen(k) }
    }
}
