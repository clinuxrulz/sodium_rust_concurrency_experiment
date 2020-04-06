
type Tracer = dyn FnMut(&GcNode);

type Trace = dyn Fn(&mut Tracer);

enum Color {
    Black,
    Gray,
    Purple,
    White
}

struct GcNode {
    ref_count: u32,
    color: Color,
    buffered: bool,
    deconstructor: Box<dyn Fn()>,
    trace: Box<Trace>
}

impl GcNode {
    pub fn new<
        DECONSTRUCTOR: 'static + Fn(),
        TRACE: 'static + Fn(&mut Tracer)
    >(
        deconstructor: DECONSTRUCTOR,
        trace: TRACE
    ) -> GcNode {
        GcNode {
            ref_count: 1,
            color: Color::Black,
            buffered: false,
            deconstructor: Box::new(deconstructor),
            trace: Box::new(trace)
        }
    }

    pub fn inc_ref(&mut self) {
        self.ref_count = self.ref_count + 1;
    }

    pub fn dec_ref(&mut self) {
        self.ref_count = self.ref_count - 1;
    }

    pub fn free(&self) {
        (self.deconstructor)();
    }

    pub fn trace(&self, tracer: &mut Tracer) {
        (self.trace)(tracer);
    }
}
