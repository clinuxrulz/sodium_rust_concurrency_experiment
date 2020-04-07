use std::sync::Arc;
use std::sync::Mutex;

pub type Tracer = dyn FnMut(&GcNode);

pub type Trace = dyn Fn(&mut Tracer) + Send + Sync;

enum Color {
    Black,
    Gray,
    Purple,
    White
}

#[derive(Clone)]
pub struct GcNode {
    gc_ctx: GcCtx,
    data: Arc<Mutex<GcNodeData>>
}

struct GcNodeData {
    ref_count: u32,
    color: Color,
    buffered: bool,
    deconstructor: Box<dyn Fn()+Send+Sync>,
    trace: Box<Trace>
}

#[derive(Clone)]
pub struct GcCtx {
    data: Arc<Mutex<GcCtxData>>
}

struct GcCtxData {
    roots: Vec<GcNode>
}

impl GcCtx {
    pub fn new() -> GcCtx {
        GcCtx {
            data: Arc::new(Mutex::new(GcCtxData {
                roots: Vec::new()
            }))
        }
    }

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
        DECONSTRUCTOR: 'static + Fn() + Send + Sync,
        TRACE: 'static + Fn(&mut Tracer) + Send + Sync
    >(
        gc_ctx: &GcCtx,
        deconstructor: DECONSTRUCTOR,
        trace: TRACE
    ) -> GcNode {
        GcNode {
            gc_ctx: gc_ctx.clone(),
            data: Arc::new(Mutex::new(GcNodeData {
                ref_count: 1,
                color: Color::Black,
                buffered: false,
                deconstructor: Box::new(deconstructor),
                trace: Box::new(trace)
            }))
        }
    }

    pub fn with_data<R,K:FnOnce(&mut GcNodeData)->R>(&self, mut k: K)->R {
        let mut l = self.data.lock();
        let data = l.as_mut().unwrap();
        k(data)
    }

    pub fn inc_ref(&self) {
        self.with_data(
            |data: &mut GcNodeData|
                data.ref_count = data.ref_count + 1
        );
    }

    pub fn dec_ref(&self) {
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
            self.gc_ctx.possible_root(self.clone());
        }
    }

    pub fn free(&self) {
        self.with_data(|data: &mut GcNodeData| {
            (data.deconstructor)();
            data.deconstructor = Box::new(|| {});
            data.trace = Box::new(|_tracer| {});
        });
    }

    pub fn trace(&self, tracer: &mut Tracer) {
        self.with_data(|data: &mut GcNodeData| {
            (data.trace)(tracer);
        });
    }
}
