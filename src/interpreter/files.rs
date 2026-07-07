use std::fs::File;
use std::io::{BufReader, BufWriter};

#[derive(Debug)]
pub(super) struct ReaderState {
    pub(super) reader: BufReader<File>,
    pub(super) eof: bool,
}

#[derive(Debug)]
pub(super) enum FileState {
    Read(ReaderState),
    Write(BufWriter<File>),
    Append(BufWriter<File>),
}
