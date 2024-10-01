use crate::hexdb::dptr::P;

pub(crate) trait DbSeek {
    fn pos(&mut self) -> std::io::Result<P>;

    fn seek(&mut self, dp: P) -> std::io::Result<P>;

    fn fast_forward(&mut self) -> std::io::Result<P>;
}

impl<S> DbSeek for S
where
    S: std::io::Seek,
{
    fn pos(&mut self) -> std::io::Result<P> {
        self.stream_position().map(P::from)
    }

    fn seek(&mut self, dp: P) -> std::io::Result<P> {
        self.seek(std::io::SeekFrom::Start(dp.into())).map(P::from)
    }

    fn fast_forward(&mut self) -> std::io::Result<P> {
        self.seek(std::io::SeekFrom::End(0)).map(P::from)
    }
}
