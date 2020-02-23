use bacon_rajan_cc::{Cc, Trace, Tracer};

pub struct Dep {
    pub data: Cc<Box<dyn Trace>>
}

impl Trace for Dep {
    fn trace(&self, tracer: &mut Tracer) {
        self.data.trace(tracer);
    }
}

impl Dep {
    pub fn new<X:Trace+'static>(x: X) -> Dep {
        Dep { data: Cc::new(Box::new(x)) }
    }
}

impl Clone for Dep {
    fn clone(&self) -> Self {
        Dep { data: self.data.clone() }
    }
}
