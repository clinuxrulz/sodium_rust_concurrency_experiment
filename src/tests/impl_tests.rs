use crate::impl_::cell_sink::CellSink;
use crate::impl_::sodium_ctx::SodiumCtx;
use crate::impl_::stream_sink::StreamSink;

#[test]
fn stream_sink() {
    let sodium_ctx = SodiumCtx::new();
    let s : StreamSink<i32> = StreamSink::new(&sodium_ctx);
    let _l = s.stream().listen_weak(|a: &i32| {
        println!("{}", a);
    });
    s.send(1);
    s.send(2);
}

#[test]
fn stream_map() {
    let sodium_ctx = SodiumCtx::new();
    let s : StreamSink<i32> = StreamSink::new(&sodium_ctx);
    let s2 = s.stream().map(|a: &i32| a * 2);
    let _l = s2.listen_weak(|a: &i32| {
        println!("{}", a);
    });
    s.send(1);
    s.send(2);
}

#[test]
fn stream_filter() {
    let sodium_ctx = SodiumCtx::new();
    let s : StreamSink<i32> = StreamSink::new(&sodium_ctx);
    let s2 = s.stream().filter(|a: &i32| a & 1 == 0);
    let _l = s2.listen_weak(|a: &i32| {
        println!("{}", a);
    });
    s.send(1);
    s.send(2);
}

#[test]
fn stream_merge() {
    let sodium_ctx = SodiumCtx::new();
    let s1 : StreamSink<i32> = StreamSink::new(&sodium_ctx);
    let s2 : StreamSink<i32> = StreamSink::new(&sodium_ctx);
    let s3 = s1.stream().merge(&s2.stream(), |a: &i32, b: &i32| a + b);
    let _l = s3.listen_weak(|a: &i32| println!("{}", a));
    s1.send(1);
    s2.send(2);
    sodium_ctx.transaction(|| {
        s1.send(1);
        s2.send(2);
    });
}

#[test]
fn cell_sink() {
    let sodium_ctx = SodiumCtx::new();
    let cs1 : CellSink<i32> = CellSink::new(&sodium_ctx, 1);
    let c1 = cs1.cell();
    let _l = c1.listen_weak(|a| println!("{}", a));
    cs1.send(2);
    cs1.send(3);
    cs1.send(4);
}
