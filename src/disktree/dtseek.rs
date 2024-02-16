use crate::disktree::dptr::Dp;

pub(crate) trait DtSeek {
    fn pos(&mut self) -> std::io::Result<Dp>;

    fn seek(&mut self, dp: Dp) -> std::io::Result<Dp>;

    fn fast_forward(&mut self) -> std::io::Result<Dp>;
}

impl<S> DtSeek for S
where
    S: std::io::Seek,
{
    fn pos(&mut self) -> std::io::Result<Dp> {
        self.stream_position().map(Dp::from)
    }

    fn seek(&mut self, dp: Dp) -> std::io::Result<Dp> {
        self.seek(std::io::SeekFrom::Start(dp.into())).map(Dp::from)
    }

    fn fast_forward(&mut self) -> std::io::Result<Dp> {
        self.seek(std::io::SeekFrom::End(0)).map(Dp::from)
    }
}
