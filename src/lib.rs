mod impl_;
mod cell;
mod cell_sink;
mod listener;
mod sodium_ctx;
mod stream;
mod stream_sink;

pub use self::cell::Cell;
pub use self::cell_sink::CellSink;
pub use self::impl_::lambda::IsLambda1;
pub use self::impl_::lambda::IsLambda2;
pub use self::impl_::lambda::IsLambda3;
pub use self::impl_::lambda::IsLambda4;
pub use self::impl_::lambda::IsLambda5;
pub use self::impl_::lambda::IsLambda6;
pub use self::impl_::lambda::Lambda;
pub use self::impl_::lambda::lambda1;
pub use self::impl_::lambda::lambda2;
pub use self::impl_::lambda::lambda3;
pub use self::impl_::lambda::lambda4;
pub use self::impl_::lambda::lambda5;
pub use self::impl_::lambda::lambda6;
pub use self::impl_::node::Node;
pub use self::stream::Stream;
pub use self::stream_sink::StreamSink;

#[cfg(test)]
mod tests {
    use crate::impl_::cell_sink::CellSink;
    use crate::impl_::sodium_ctx::SodiumCtx;
    use crate::impl_::stream_sink::StreamSink;

    #[test]
    fn stream_sink() {
        let sodium_ctx = SodiumCtx::new();
        let s : StreamSink<i32> = StreamSink::new(&sodium_ctx);
        let _l = s.to_stream().listen_weak(|a: &i32| {
            println!("{}", a);
        });
        s.send(1);
        s.send(2);
    }

    #[test]
    fn stream_map() {
        let sodium_ctx = SodiumCtx::new();
        let s : StreamSink<i32> = StreamSink::new(&sodium_ctx);
        let s2 = s.to_stream().map(|a: &i32| a * 2);
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
        let s2 = s.to_stream().filter(|a: &i32| a & 1 == 0);
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
        let s3 = s1.to_stream().merge(&s2.to_stream(), |a: &i32, b: &i32| a + b);
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
        let c1 = cs1.to_cell();
        let _l = c1.listen_weak(|a| println!("{}", a));
        cs1.send(2);
        cs1.send(3);
        cs1.send(4);
    }

    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
