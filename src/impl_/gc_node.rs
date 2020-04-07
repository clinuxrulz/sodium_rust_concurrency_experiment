use std::sync::Arc;
use std::sync::Mutex;

type Tracer = dyn FnMut(&GcNode);

type Trace = dyn Fn(&mut Tracer);

enum Color {
    Black,
    Gray,
    Purple,
    White
}

struct GcNode {
    data: *mut GcNodeData
}

struct GcNodeData {
    ref_count: u32,
    color: Color,
    buffered: bool,
    deconstructor: Box<dyn Fn()>,
    trace: Box<Trace>
}

struct GcCtx {
    data: Arc<Mutex<GcCtxData>>
}

struct GcCtxData {
    roots: Vec<GcNode>
}

impl GcCtx {
    pub fn with_data<R,K:FnOnce(&mut GcCtxData)->R>(&self, mut k:K) -> R {
        let mut l = self.data.lock();
        let data = l.as_mut().unwrap();
        k(data)
    }

    pub fn possible_root(&self, node: GcNode) {
        self.with_data(|data: &mut GcCtxData| data.roots.push(node));
    }
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
            data: Box::into_raw(Box::new(GcNodeData {
                ref_count: 1,
                color: Color::Black,
                buffered: false,
                deconstructor: Box::new(deconstructor),
                trace: Box::new(trace)
            }))
        }
    }

    pub fn with_data<R,K:FnOnce(&mut GcNodeData)->R>(&self, mut k: K)->R {
        unsafe { k(&mut *self.data) }
    }

    pub fn inc_ref(&mut self) {
        self.with_data(
            |data: &mut GcNodeData|
                data.ref_count = data.ref_count + 1
        );
    }

    pub fn dec_ref(&mut self) {
        let (ref_count, buffered) =
            self.with_data(
                |data: &mut GcNodeData| {
                    data.ref_count = data.ref_count - 1;
                    (data.ref_count, data.buffered)
                }
            );
        if ref_count == 0 {
            self.free();
        } else {

        }
    }

    pub fn free(&self) {
        self.with_data(|data: &mut GcNodeData| {
            (data.deconstructor)();
        });
    }

    pub fn trace(&self, tracer: &mut Tracer) {
        self.with_data(|data: &mut GcNodeData| {
            (data.trace)(tracer);
        });
    }
}
