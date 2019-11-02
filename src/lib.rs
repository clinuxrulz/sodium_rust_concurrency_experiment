mod listener;
mod node;
mod sodium_ctx;
mod stream;
mod stream_sink;

#[cfg(test)]
mod tests {
    use crate::sodium_ctx::SodiumCtx;
    use crate::stream_sink::StreamSink;

    #[test]
    fn stream_sink() {
        let sodium_ctx = SodiumCtx::new();
        let s : StreamSink<i32> = StreamSink::new(&sodium_ctx);
        let l = s.to_stream().listen(|a: &i32| {
            println!("{}", a);
        });
        s.send(&sodium_ctx, 1);
        s.send(&sodium_ctx, 2);
    }

    #[test]
    fn stream_map() {
        let sodium_ctx = SodiumCtx::new();
        let s : StreamSink<i32> = StreamSink::new(&sodium_ctx);
        let s2 = s.to_stream().map(|a: &i32| a * 2);
        let _l = s2.listen(|a: &i32| {
            println!("{}", a);
        });
        s.send(&sodium_ctx, 1);
        s.send(&sodium_ctx, 2);
    }

    #[test]
    fn stream_filter() {
        let sodium_ctx = SodiumCtx::new();
        let s : StreamSink<i32> = StreamSink::new(&sodium_ctx);
        let s2 = s.to_stream().filter(|a| a & 1 == 0);
        let _l = s2.listen(|a: &i32| {
            println!("{}", a);
        });
        s.send(&sodium_ctx, 1);
        s.send(&sodium_ctx, 2);
    }

    #[test]
    fn stream_merge() {
        let sodium_ctx = SodiumCtx::new();
        let s1 : StreamSink<i32> = StreamSink::new(&sodium_ctx);
        let s2 : StreamSink<i32> = StreamSink::new(&sodium_ctx);
        let s3 = s1.to_stream().merge(&s2.to_stream(), |a, b| a + b);
        let _l = s3.listen(|a| println!("{}", a));
        s1.send(&sodium_ctx, 1);
        s2.send(&sodium_ctx, 2);
        sodium_ctx.transaction(|| {
            s1.send(&sodium_ctx, 1);
            s2.send(&sodium_ctx, 2);
        });
    }

    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
