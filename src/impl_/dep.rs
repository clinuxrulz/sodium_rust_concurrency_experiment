use crate::impl_::gc::{Finalize, Gc, GcDep, Trace};

pub struct Dep {
    pub data: GcDep
}

impl Trace for Dep {
    fn trace(&self, tracer: &mut dyn FnMut(&GcDep)) {
        tracer(&self.data);
    }
}

impl Dep {
    pub fn new<X:Into<GcDep>>(x: X) -> Dep {
        Dep { data: x.into() }
    }
}

impl Clone for Dep {
    fn clone(&self) -> Self {
        Dep { data: self.data.clone() }
    }
}
