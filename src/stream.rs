use crate::Cell;
use crate::impl_::dep::Dep;
use crate::impl_::stream::Stream as StreamImpl;
use crate::impl_::node::Node;
use crate::impl_::lambda::IsLambda1;
use crate::impl_::lambda::IsLambda2;
use crate::impl_::lambda::IsLambda3;
use crate::impl_::lambda::lambda2;
use crate::impl_::lambda::lambda3_deps;
use crate::Lazy;
use crate::listener::Listener;
use crate::sodium_ctx::SodiumCtx;

pub struct Stream<A:'static> {
    pub impl_: StreamImpl<A>
}

impl<A> Clone for Stream<A> {
    fn clone(&self) -> Self {
        Stream {
            impl_: self.impl_.clone()
        }
    }
}

impl<A:Clone+'static> Stream<Option<A>> {
    pub fn filter_option(&self) -> Stream<A> {
        self.filter(|a: &Option<A>| a.is_some()).map(|a: &Option<A>| a.clone().unwrap())
    }
}

impl<A:Clone+'static> Stream<A> {
    pub fn new(sodium_ctx: &SodiumCtx) -> Stream<A> {
        Stream {
            impl_: StreamImpl::new(&sodium_ctx.impl_)
        }
    }

    pub fn node(&self) -> Node {
        self.impl_.node()
    }

    // use as dependency to lambda1, lambda2, etc.
    pub fn to_dep(&self) -> Dep {
        Dep::new(self.impl_.clone())
    }

    pub fn snapshot<B:Clone+'static,C:Clone+'static,FN:IsLambda2<A,B,C>+'static>(&self, cb: &Cell<B>, f: FN) -> Stream<C> {
        Stream { impl_: self.impl_.snapshot(&cb.impl_, f) }
    }

    pub fn snapshot1<B:Clone+'static>(&self, cb: &Cell<B>) -> Stream<B> {
        self.snapshot(cb, |_a: &A, b: &B| b.clone())
    }

    pub fn snapshot3<B:Clone+'static,C:Clone+'static,D:Clone+'static,FN:IsLambda3<A,B,C,D>+'static>(&self, cb: &Cell<B>, cc: &Cell<C>, mut f: FN) -> Stream<D> {
        let deps: Vec<Dep> = lambda3_deps(&f);
        let cc = cc.clone();
        self.snapshot(cb, lambda2(move |a: &A, b: &B| f.call(a, b, &cc.sample()), deps))
    }

    pub fn map<B:Clone+'static,FN:IsLambda1<A,B>+'static>(&self, f: FN) -> Stream<B> {
        Stream { impl_: self.impl_.map(f) }
    }

    pub fn map_to<B:Clone+'static>(&self, b: B) -> Stream<B> {
        self.map(move |_:&A| b.clone())
    }

    pub fn filter<PRED:IsLambda1<A,bool>+'static>(&self, pred: PRED) -> Stream<A> {
        Stream { impl_: self.impl_.filter(pred) }
    }

    pub fn or_else(&self, s2: &Stream<A>) -> Stream<A> {
        self.merge(s2, |lhs:&A, _rhs:&A| lhs.clone())
    }

    pub fn merge<FN:IsLambda2<A,A,A>+'static>(&self, s2: &Stream<A>, f: FN) -> Stream<A> {
        Stream { impl_: self.impl_.merge(&s2.impl_, f) }
    }

    pub fn hold(&self, a: A) -> Cell<A> {
        Cell { impl_: self.impl_.hold(a) }
    }

    pub fn hold_lazy(&self, a: Lazy<A>) -> Cell<A> {
        Cell { impl_: self.impl_.hold_lazy(a) }
    }

    pub fn gate(&self, cpred: &Cell<bool>) -> Stream<A> {
        let cpred = cpred.clone();
        self.filter(move |_: &A| cpred.sample())
    }

    pub fn once(&self) -> Stream<A> {
        Stream { impl_: self.impl_.once() }
    }

    pub fn collect<B,S,F>(&self, init_state: S, f: F) -> Stream<B>
        where B: Clone + 'static,
              S: Clone + 'static,
              F: IsLambda2<A,S,(B,S)> + 'static
    {
        self.collect_lazy(Lazy::new(move || init_state.clone()), f)
    }

    pub fn collect_lazy<B,S,F>(&self, init_state: Lazy<S>, f: F) -> Stream<B>
        where B: Clone + 'static,
              S: Clone + 'static,
              F: IsLambda2<A,S,(B,S)> + 'static
    {
        Stream { impl_: self.impl_.collect_lazy(init_state, f) }
    }

    pub fn accum<S,F>(&self, init_state: S, f: F) -> Cell<S>
        where S: Clone + 'static,
              F: IsLambda2<A,S,S> + 'static
    {
        self.accum_lazy(Lazy::new(move || init_state.clone()), f)
    }

    pub fn accum_lazy<S,F>(&self, init_state: Lazy<S>, f: F) -> Cell<S>
        where S: Clone + 'static,
              F: IsLambda2<A,S,S> + 'static
    {
        Cell { impl_: self.impl_.accum_lazy(init_state, f) }
    }

    pub fn listen_weak<K:IsLambda1<A,()>+'static>(&self, k: K) -> Listener {
        Listener { impl_: self.impl_.listen_weak(k) }
    }

    pub fn listen<K:IsLambda1<A,()>+'static>(&self, k: K) -> Listener {
        Listener { impl_: self.impl_.listen(k) }
    }
}
