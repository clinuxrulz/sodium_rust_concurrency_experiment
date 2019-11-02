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
        let s : StreamSink<i32> = StreamSink::new();
        let l = s.to_stream().listen(|a: &i32| {
            println!("{}", a);
        });
        s.send(&sodium_ctx, 1);
        s.send(&sodium_ctx, 2);
    }

    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
