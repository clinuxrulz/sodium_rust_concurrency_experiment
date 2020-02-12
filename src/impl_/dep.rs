use std::rc::Rc;
use crate::impl_::gc::{GcDep, Trace};

pub struct Dep {
    pub data: Rc<dyn Trace>
}

impl Trace for Dep {
    fn trace(&self, tracer: &mut dyn FnMut(&GcDep)) {
        self.data.trace(tracer);
    }
}

impl Dep {
    pub fn new<X:Trace+'static>(x: X) -> Dep {
        Dep { data: Rc::new(x) }
    }
}

impl Clone for Dep {
    fn clone(&self) -> Self {
        Dep { data: self.data.clone() }
    }
}
