use axum::{
    body,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    BoxError,
};
use bytes::Bytes;
use futures_util::TryStream;
use std::{io, path::Path};
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncSeekExt},
};
use tokio_util::io::ReaderStream;

#[derive(Debug)]
pub struct FileStream<S> {
    /// stream.
    pub stream: S,
    /// The file name of the file.
    pub file_name: Option<String>,
    /// The size of the file.
    pub content_size: Option<u64>,
}

impl<S> FileStream<S>
where
    S: TryStream + Send + 'static,
    S::Ok: Into<Bytes>,
    S::Error: Into<BoxError>,
{
    pub fn new(stream: S) -> Self {
        Self {
            stream,
            file_name: None,
            content_size: None,
        }
    }

    pub async fn from_path(path: impl AsRef<Path>) -> io::Result<FileStream<ReaderStream<File>>> {
        let file = File::open(&path).await?;
        let mut content_size = None;
        let mut file_name = None;

        if let Ok(metadata) = file.metadata().await {
            content_size = Some(metadata.len());
        }

        if let Some(file_name_os) = path.as_ref().file_name() {
            if let Some(file_name_str) = file_name_os.to_str() {
                file_name = Some(file_name_str.to_owned());
            }
        }

        Ok(FileStream {
            stream: ReaderStream::new(file),
            file_name,
            content_size,
        })
    }

    pub fn file_name(mut self, file_name: impl Into<String>) -> Self {
        self.file_name = Some(file_name.into());
        self
    }

    pub fn content_size(mut self, len: u64) -> Self {
        self.content_size = Some(len);
        self
    }

    pub fn into_range_response(self, start: u64, end: u64, total_size: u64) -> Response {
        let mut resp = Response::builder().header(header::CONTENT_TYPE, "video/mp4");
        resp = resp.status(StatusCode::PARTIAL_CONTENT);

        println!("bytes {start}-{end}/{total_size}");
        resp = resp.header(
            header::CONTENT_RANGE,
            format!("bytes {start}-{end}/{total_size}"),
        );

        let content_length = total_size - start;
        resp = resp.header(header::CONTENT_LENGTH, content_length);

        resp.body(body::Body::from_stream(self.stream))
            .unwrap_or_else(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("build FileStream responsec error: {e}"),
                )
                    .into_response()
            })
    }

    pub async fn try_range_response(
        file_path: impl AsRef<Path>,
        start: u64,
        mut end: u64,
    ) -> io::Result<Response> {
        // open file
        let mut file = File::open(file_path).await?;

        // get file metadata
        let metadata = file.metadata().await?;
        let total_size = metadata.len();
        println!("total size: {total_size}");

        if end == 0 {
            end = total_size - 1;
        }

        // range check
        if start > total_size {
            return Ok((StatusCode::RANGE_NOT_SATISFIABLE, "Range Not Satisfiable").into_response());
        }
        if start > end {
            return Ok((StatusCode::RANGE_NOT_SATISFIABLE, "Range Not Satisfiable").into_response());
        }
        if end >= total_size {
            return Ok((StatusCode::RANGE_NOT_SATISFIABLE, "Range Not Satisfiable").into_response());
        }

        // get file stream and seek to start to return range response
        file.seek(std::io::SeekFrom::Start(start)).await?;

        let stream = ReaderStream::new(file.take(end - start + 1));

        Ok(FileStream::new(stream).into_range_response(start, end, total_size))
    }
}

impl<S> IntoResponse for FileStream<S>
where
    S: TryStream + Send + 'static,
    S::Ok: Into<Bytes>,
    S::Error: Into<BoxError>,
{
    fn into_response(self) -> Response {
        let mut resp = Response::builder().header(header::CONTENT_TYPE, "video/mp4");

        if let Some(file_name) = self.file_name {
            resp = resp.header(
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{file_name}\""),
            );
            let file_split: Vec<&str> = file_name.split(".").collect();
            if let Some(file_stem) = file_split.get(0) {
                resp = resp.header(
                    header::HeaderName::from_static("file_stem"),
                    header::HeaderValue::from_str(file_stem).unwrap(),
                );
            }
        }

        if let Some(content_size) = self.content_size {
            resp = resp.header(header::CONTENT_LENGTH, content_size);
        }

        resp.body(body::Body::from_stream(self.stream))
            .unwrap_or_else(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("build FileStream responsec error: {e}"),
                )
                    .into_response()
            })
    }
}
