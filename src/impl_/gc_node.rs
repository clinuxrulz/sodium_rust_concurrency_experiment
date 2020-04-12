use std::sync::Arc;
use std::sync::Mutex;

pub type Tracer<'a> = dyn FnMut(&GcNode) + 'a;

pub type Trace = dyn Fn(&mut Tracer) + Send + Sync;

#[derive(PartialEq,Eq,Clone,Copy)]
enum Color {
    Black,
    Gray,
    Purple,
    White
}

#[derive(Clone)]
pub struct GcNode {
    id: u32,
    gc_ctx: GcCtx,
    data: Arc<Mutex<GcNodeData>>
}

struct GcNodeData {
    freed: bool,
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
    next_id: u32,
    roots: Vec<GcNode>,
    to_be_freed: Vec<GcNode>
}

impl GcCtx {
    pub fn new() -> GcCtx {
        GcCtx {
            data: Arc::new(Mutex::new(GcCtxData {
                next_id: 0,
                roots: Vec::new(),
                to_be_freed: Vec::new()
            }))
        }
    }

    fn with_data<R,K:FnOnce(&mut GcCtxData)->R>(&self, mut k:K) -> R {
        let mut l = self.data.lock();
        let data = l.as_mut().unwrap();
        k(data)
    }

    pub fn make_id(&self) -> u32 {
        self.with_data(|data: &mut GcCtxData| {
            let id = data.next_id;
            data.next_id = data.next_id + 1;
            id
        })
    }

    pub fn possible_root(&self, node: GcNode) {
        self.with_data(|data: &mut GcCtxData| data.roots.push(node));
    }

    pub fn collect_cycles(&self) {
        loop {
            trace!("start: collect_cycles");
            self.mark_roots();
            self.scan_roots();
            self.collect_roots();
            trace!("end: collect_cycles");
            let bail = self.with_data(|data: &mut GcCtxData| data.roots.is_empty());
            if bail {
                break;
            }
        }
    }

    fn mark_roots(&self) {
        trace!("start: mark_roots");
        let mut old_roots: Vec<GcNode> = Vec::new();
        self.with_data(
            |data: &mut GcCtxData|
                std::mem::swap(&mut old_roots, &mut data.roots)
        );
        let mut new_roots: Vec<GcNode> = Vec::new();
        for root in old_roots {
            let (color,ref_count) = root.with_data(|data: &mut GcNodeData| (data.color, data.ref_count));
            if color == Color::Purple {
                self.mark_gray(&root);
                new_roots.push(root);
            } else {
                let free_it =
                    root.with_data(
                        |data: &mut GcNodeData| {
                            data.buffered = false;
                            data.color == Color::Black && data.ref_count == 0
                        }
                    );
                if free_it {
                    self.with_data(
                        |data: &mut GcCtxData|
                            data.to_be_freed.push(root)
                    );
                }
            }
        }
        self.with_data(
            |data: &mut GcCtxData|
                std::mem::swap(&mut new_roots, &mut data.roots)
        );
        trace!("end: mark_roots");
    }

    fn mark_gray(&self, node: &GcNode) {
        let bail = node.with_data(|data: &mut GcNodeData| {
            if data.color == Color::Gray {
                return true;
            }
            data.color = Color::Gray;
            false
        });
        if bail {
            return;
        }

        let this = self.clone();
        node.trace(&mut |t: &GcNode| {
            trace!("mark_gray: gc node {} dec ref count", t.id);
            t.with_data(|data: &mut GcNodeData| data.ref_count = data.ref_count - 1);
            this.mark_gray(t);
        });
    }

    fn scan_roots(&self) {
        trace!("start: scan_roots");
        let mut roots = Vec::new();
        self.with_data(
            |data: &mut GcCtxData|
                std::mem::swap(&mut roots, &mut data.roots)
        );
        for root in &roots {
            self.scan(root);
        }
        self.with_data(
            |data: &mut GcCtxData|
                std::mem::swap(&mut roots, &mut data.roots)
        );
        trace!("end: scan_roots");
    }

    fn scan(&self, s: &GcNode) {
        let bail = s.with_data(|data: &mut GcNodeData| data.color != Color::Gray);
        if bail {
            return;
        }

        let ref_count = s.with_data(|data: &mut GcNodeData| data.ref_count);

        if ref_count > 0 {
            self.scan_black(s);
        } else {
            s.with_data(|data: &mut GcNodeData|
                data.color = Color::White
            );
            let this = self.clone();
            s.trace(|t| {
                this.scan(t);
            });
        }
    }

    fn scan_black(&self, s: &GcNode) {
        s.with_data(|data: &mut GcNodeData| data.color = Color::Black);
        let this = self.clone();
        s.trace(|t| {
            trace!("scan_black: gc node {} inc ref count", t.id);
            let color =
                t.with_data(|data: &mut GcNodeData| {
                    data.ref_count = data.ref_count + 1;
                    data.color
                });
            if color != Color::Black {
                this.scan_black(t);
            }
        });
    }

    fn collect_roots(&self) {
        let mut white = Vec::new();
        let mut roots = Vec::new();
        self.with_data(|data: &mut GcCtxData| roots.append(&mut data.roots));
        for root in roots {
            root.with_data(|data: &mut GcNodeData| data.buffered = false);
            self.collect_white(&root, &mut white);
        }
        for i in white {
            if !i.with_data(|data: &mut GcNodeData| data.freed) {
                trace!("collect_roots: freeing white node {}", i.id);
                i.free();
            }
        }
        let mut to_be_freed = Vec::new();
        self.with_data(|data: &mut GcCtxData| to_be_freed.append(&mut data.to_be_freed));
        for i in to_be_freed {
            if !i.with_data(|data: &mut GcNodeData| data.freed) {
                trace!("collect_roots: freeing to_be_freed node {}", i.id);
                i.free();
            }
        }
    }

    fn collect_white(&self, s: &GcNode, white: &mut Vec<GcNode>) {
        if s.with_data(|data: &mut GcNodeData| data.color == Color::White && !data.buffered) {
            s.with_data(|data: &mut GcNodeData| data.color = Color::Black);
            let this = self.clone();
            s.trace(|t| {
                this.collect_white(t, white);
            });
            white.push(s.clone());
        }
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
            id: gc_ctx.make_id(),
            gc_ctx: gc_ctx.clone(),
            data: Arc::new(Mutex::new(GcNodeData {
                freed: false,
                ref_count: 1,
                color: Color::Black,
                buffered: false,
                deconstructor: Box::new(deconstructor),
                trace: Box::new(trace)
            }))
        }
    }

    fn with_data<R,K:FnOnce(&mut GcNodeData)->R>(&self, k: K)->R {
        let mut l = self.data.lock();
        let data = l.as_mut().unwrap();
        k(data)
    }

    pub fn ref_count(&self) -> u32 {
        self.with_data(|data: &mut GcNodeData| data.ref_count)
    }

    pub fn inc_ref_if_alive(&self) -> bool {
        self.with_data(
            |data: &mut GcNodeData| {
                if data.ref_count != 0 {
                    data.ref_count = data.ref_count + 1;
                    data.color = Color::Black;
                    true
                } else {
                    false
                }
            }
        )
    }

    pub fn inc_ref(&self) {
        self.with_data(
            |data: &mut GcNodeData| {
                data.ref_count = data.ref_count + 1;
                data.color = Color::Black;
            }
        );
    }

    pub fn dec_ref(&self) {
        if self.with_data(|data: &mut GcNodeData| data.ref_count == 0) {
            return;
        }
        let (ref_count, buffered) =
            self.with_data(
                |data: &mut GcNodeData| {
                    data.ref_count = data.ref_count - 1;
                    (data.ref_count, data.buffered)
                }
            );
        if ref_count == 0 {
            self.with_data(|data: &mut GcNodeData| {
                data.color = Color::Black;
            });
            if !buffered {
                self.free();
            }
        } else {
            self.with_data(|data: &mut GcNodeData| {
                data.buffered = true;
                data.color = Color::Purple
            });
            self.gc_ctx.possible_root(self.clone());
        }
    }

    pub fn free(&self) {
        let mut deconstructor: Box<dyn Fn()+Send+Sync> = Box::new(|| {});
        self.with_data(|data: &mut GcNodeData| {
            std::mem::swap(&mut deconstructor, &mut data.deconstructor);
            data.trace = Box::new(|_tracer: &mut Tracer| {});
            data.freed = true;
        });
        deconstructor();
    }

    pub fn trace<TRACER: FnMut(&GcNode)>(&self, mut tracer: TRACER) {
        let mut trace: Box<Trace> = Box::new(|_tracer: &mut Tracer| {});
        self.with_data(|data: &mut GcNodeData| {
            std::mem::swap(&mut trace, &mut data.trace);
        });
        trace(&mut tracer);
        self.with_data(|data: &mut GcNodeData| {
            std::mem::swap(&mut trace, &mut data.trace);
        });
    }
}
