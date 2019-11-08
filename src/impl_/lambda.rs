use crate::impl_::node::Node;

pub struct Lambda<FN> {
    f: FN,
    deps: Vec<Node>
}

pub trait IsLambda1<A,B> {
    fn call(&mut self, a: &A) -> B;
    fn deps_op<'r>(&'r self) -> Option<&'r Vec<Node>>;
}

pub trait IsLambda2<A,B,C> {
    fn call(&mut self, a: &A, b: &B) -> C;
    fn deps_op<'r>(&'r self) -> Option<&'r Vec<Node>>;
}

pub trait IsLambda3<A,B,C,D> {
    fn call(&mut self, a: &A, b: &B, c: &C) -> D;
    fn deps_op<'r>(&'r self) -> Option<&'r Vec<Node>>;
}

pub trait IsLambda4<A,B,C,D,E> {
    fn call(&mut self, a: &A, b: &B, c: &C, d: &D) -> E;
    fn deps_op<'r>(&'r self) -> Option<&'r Vec<Node>>;
}

pub trait IsLambda5<A,B,C,D,E,F> {
    fn call(&mut self, a: &A, b: &B, c: &C, d: &D, e: &E) -> F;
    fn deps_op<'r>(&'r self) -> Option<&'r Vec<Node>>;
}

pub trait IsLambda6<A,B,C,D,E,F,G> {
    fn call(&mut self, a: &A, b: &B, c: &C, d: &D, e: &E, f: &F) -> G;
    fn deps_op<'r>(&'r self) -> Option<&'r Vec<Node>>;
}

impl<A,B,FN:FnMut(&A)->B> IsLambda1<A,B> for Lambda<FN> {

    fn call(&mut self, a: &A) -> B {
        (self.f)(a)
    }

    fn deps_op<'r>(&'r self) -> Option<&'r Vec<Node>> {
        Some(&self.deps)
    }
}

impl<A,B,FN:FnMut(&A)->B> IsLambda1<A,B> for FN {

    fn call(&mut self, a: &A) -> B {
        self(a)
    }

    fn deps_op<'r>(&'r self) -> Option<&'r Vec<Node>> {
        None
    }
}

impl<A,B,C,FN:FnMut(&A,&B)->C> IsLambda2<A,B,C> for Lambda<FN> {

    fn call(&mut self, a: &A, b: &B) -> C {
        (self.f)(a,b)
    }

    fn deps_op<'r>(&'r self) -> Option<&'r Vec<Node>> {
        Some(&self.deps)
    }
}

impl<A,B,C,FN:FnMut(&A,&B)->C> IsLambda2<A,B,C> for FN {

    fn call(&mut self, a: &A, b: &B) -> C {
        self(a,b)
    }

    fn deps_op<'r>(&'r self) -> Option<&'r Vec<Node>> {
        None
    }
}

impl<A,B,C,D,FN:FnMut(&A,&B,&C)->D> IsLambda3<A,B,C,D> for Lambda<FN> {

    fn call(&mut self, a: &A, b: &B, c: &C) -> D {
        (self.f)(a,b,c)
    }

    fn deps_op<'r>(&'r self) -> Option<&'r Vec<Node>> {
        Some(&self.deps)
    }
}

impl<A,B,C,D,FN:FnMut(&A,&B,&C)->D> IsLambda3<A,B,C,D> for FN {

    fn call(&mut self, a: &A, b: &B, c: &C) -> D {
        self(a,b,c)
    }

    fn deps_op<'r>(&'r self) -> Option<&'r Vec<Node>> {
        None
    }
}

impl<A,B,C,D,E,FN:FnMut(&A,&B,&C,&D)->E> IsLambda4<A,B,C,D,E> for Lambda<FN> {

    fn call(&mut self, a: &A, b: &B, c: &C, d: &D) -> E {
        (self.f)(a,b,c,d)
    }

    fn deps_op<'r>(&'r self) -> Option<&'r Vec<Node>> {
        Some(&self.deps)
    }
}

impl<A,B,C,D,E,FN:FnMut(&A,&B,&C,&D)->E> IsLambda4<A,B,C,D,E> for FN {

    fn call(&mut self, a: &A, b: &B, c: &C, d: &D) -> E {
        self(a,b,c,d)
    }

    fn deps_op<'r>(&'r self) -> Option<&'r Vec<Node>> {
        None
    }
}

impl<A,B,C,D,E,F,FN:FnMut(&A,&B,&C,&D,&E)->F> IsLambda5<A,B,C,D,E,F> for Lambda<FN> {

    fn call(&mut self, a: &A, b: &B, c: &C, d: &D, e: &E) -> F {
        (self.f)(a,b,c,d,e)
    }

    fn deps_op<'r>(&'r self) -> Option<&'r Vec<Node>> {
        Some(&self.deps)
    }
}

impl<A,B,C,D,E,F,FN:FnMut(&A,&B,&C,&D,&E)->F> IsLambda5<A,B,C,D,E,F> for FN {

    fn call(&mut self, a: &A, b: &B, c: &C, d: &D, e: &E) -> F {
        self(a,b,c,d,e)
    }

    fn deps_op<'r>(&'r self) -> Option<&'r Vec<Node>> {
        None
    }
}

impl<A,B,C,D,E,F,G,FN:FnMut(&A,&B,&C,&D,&E,&F)->G> IsLambda6<A,B,C,D,E,F,G> for Lambda<FN> {

    fn call(&mut self, a: &A, b: &B, c: &C, d: &D, e: &E, f: &F) -> G {
        (self.f)(a,b,c,d,e,f)
    }

    fn deps_op<'r>(&'r self) -> Option<&'r Vec<Node>> {
        Some(&self.deps)
    }
}

impl<A,B,C,D,E,F,G,FN:FnMut(&A,&B,&C,&D,&E,&F)->G> IsLambda6<A,B,C,D,E,F,G> for FN {

    fn call(&mut self, a: &A, b: &B, c: &C, d: &D, e: &E, f: &F) -> G {
        self(a,b,c,d,e,f)
    }

    fn deps_op<'r>(&'r self) -> Option<&'r Vec<Node>> {
        None
    }
}

pub fn lambda1<A,B,FN:FnMut(&A)->B>(f: FN, deps: Vec<Node>) -> Lambda<FN> {
    Lambda { f, deps }
}

pub fn lambda2<A,B,C,FN:FnMut(&A,&B)->C>(f: FN, deps: Vec<Node>) -> Lambda<FN> {
    Lambda { f, deps }
}

pub fn lambda3<A,B,C,D,FN:FnMut(&A,&B,&C)->D>(f: FN, deps: Vec<Node>) -> Lambda<FN> {
    Lambda { f, deps }
}

pub fn lambda4<A,B,C,D,E,FN:FnMut(&A,&B,&C,&D)->E>(f: FN, deps: Vec<Node>) -> Lambda<FN> {
    Lambda { f, deps }
}

pub fn lambda5<A,B,C,D,E,F,FN:FnMut(&A,&B,&C,&D,&E)->F>(f: FN, deps: Vec<Node>) -> Lambda<FN> {
    Lambda { f, deps }
}

pub fn lambda6<A,B,C,D,E,F,G,FN:FnMut(&A,&B,&C,&D,&E,&F)->G>(f: FN, deps: Vec<Node>) -> Lambda<FN> {
    Lambda { f, deps }
}