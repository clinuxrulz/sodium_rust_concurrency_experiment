use crate::Cell;
use crate::Stream;

pub struct Operational {}

impl Operational {

    pub fn updates<A:Clone+'static>(ca: &Cell<A>) -> Stream<A> {
        Stream { impl_: ca.impl_.updates() }
    }

    pub fn value<A:Clone+'static>(ca: &Cell<A>) -> Stream<A> {
        Stream { impl_: ca.impl_.value() }
    }

    pub fn defer<A:Clone+'static>(sa: &Stream<A>) -> Stream<A> {
        Stream { impl_: sa.impl_.defer() }
    }
}
