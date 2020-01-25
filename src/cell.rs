use crate::impl_::cell::Cell as CellImpl;
use crate::impl_::dep::Dep;
use crate::impl_::lambda::IsLambda1;
use crate::impl_::lambda::IsLambda2;
use crate::impl_::lambda::IsLambda3;
use crate::impl_::lambda::IsLambda4;
use crate::impl_::lambda::IsLambda5;
use crate::impl_::lambda::IsLambda6;
use crate::impl_::lazy::Lazy;
use crate::listener::Listener;
use crate::sodium_ctx::SodiumCtx;
use crate::stream::Stream;
use crate::impl_::node::Node;

pub struct Cell<A:'static> {
    pub impl_: CellImpl<A>
}

impl<A> Clone for Cell<A> {
    fn clone(&self) -> Self {
        Cell { impl_: self.impl_.clone() }
    }
}

impl<A:Clone+'static> Cell<A> {
    pub fn new(sodium_ctx: &SodiumCtx, value: A) -> Cell<A> {
        Cell { impl_: CellImpl::new(&sodium_ctx.impl_, value) }
    }

    pub fn sample(&self) -> A {
        self.impl_.sample()
    }

    pub fn sample_lazy(&self) -> Lazy<A> {
        self.impl_.sample_lazy()
    }

    pub fn node(&self) -> Node {
        self.impl_.updates().node()
    }

    // use as dependency to lambda1, lambda2, etc.
    pub fn to_dep(&self) -> Dep {
        Dep::new(self.impl_.clone())
    }

    pub fn updates(&self) -> Stream<A> {
        Stream { impl_: self.impl_.updates() }
    }

    pub fn value(&self) -> Stream<A> {
        Stream { impl_: self.impl_.value() }
    }

    pub fn map<B:Clone+'static,FN:IsLambda1<A,B>+'static>(&self, f: FN) -> Cell<B> {
        Cell { impl_: self.impl_.map(f) }
    }

    pub fn lift2<B:Clone+'static,C:Clone+'static,FN:IsLambda2<A,B,C>+'static>(&self, cb: &Cell<B>, f: FN) -> Cell<C> {
        Cell { impl_: self.impl_.lift2(&cb.impl_, f) }
    }

    pub fn lift3<B:Clone+'static,C:Clone+'static,D:Clone+'static,FN:IsLambda3<A,B,C,D>+'static>(&self, cb: &Cell<B>, cc: &Cell<C>, f: FN) -> Cell<D> {
        Cell { impl_: self.impl_.lift3(&cb.impl_, &cc.impl_, f) }
    }

    pub fn lift4<B:Clone+'static,C:Clone+'static,D:Clone+'static,E:Clone+'static,FN:IsLambda4<A,B,C,D,E>+'static>(&self, cb: &Cell<B>, cc: &Cell<C>, cd: &Cell<D>, f: FN) -> Cell<E> {
        Cell { impl_: self.impl_.lift4(&cb.impl_, &cc.impl_, &cd.impl_, f) }
    }

    pub fn lift5<B:Clone+'static,C:Clone+'static,D:Clone+'static,E:Clone+'static,F:Clone+'static,FN:IsLambda5<A,B,C,D,E,F>+'static>(&self, cb: &Cell<B>, cc: &Cell<C>, cd: &Cell<D>, ce: &Cell<E>, f: FN) -> Cell<F> {
        Cell { impl_: self.impl_.lift5(&cb.impl_, &cc.impl_, &cd.impl_, &ce.impl_, f) }
    }

    pub fn lift6<B:Clone+'static,C:Clone+'static,D:Clone+'static,E:Clone+'static,F:Clone+'static,G:Clone+'static,FN:IsLambda6<A,B,C,D,E,F,G>+'static>(&self, cb: &Cell<B>, cc: &Cell<C>, cd: &Cell<D>, ce: &Cell<E>, cf: &Cell<F>, f: FN) -> Cell<G> {
        Cell { impl_: self.impl_.lift6(&cb.impl_, &cc.impl_, &cd.impl_, &ce.impl_, &cf.impl_, f) }
    }

    pub fn switch_s(csa: &Cell<Stream<A>>) -> Stream<A> {
        Stream { impl_: CellImpl::switch_s(&csa.map(|sa: &Stream<A>| sa.impl_.clone()).impl_) }
    }

    pub fn switch_c(cca: &Cell<Cell<A>>) -> Cell<A> {
        Cell { impl_: CellImpl::switch_c(&cca.map(|ca: &Cell<A>| ca.impl_.clone()).impl_) }
    }

    pub fn listen_weak<K: FnMut(&A)+'static>(&self, k: K) -> Listener {
        Listener { impl_: self.impl_.listen_weak(k) }
    }

    pub fn listen<K:IsLambda1<A,()>+'static>(&self, k: K) -> Listener {
        Listener { impl_: self.impl_.listen(k) }
    }
}
