use std::{pin::Pin, time::Instant};
use futures_lite::Future;

use tracing::{info, error, span::{Span}};
use tracing_futures::Instrument;

type Result<T> = anyhow::Result<T>;
pub type PinnedFut<'a, T=()> = Pin<Box<dyn Future<Output=Result<T>> + Send + 'a>>;

pub struct TracingTask<'a, R=()> {
    span: Span,
    future: PinnedFut<'a, R>,
    is_long_lived: bool
}

impl<'a, R: Send + Sync> TracingTask<'a, R> {
    pub fn new<T: Future<Output=Result<R>> + Send + 'a>(span: Span, fut: T) -> TracingTask<'a, R> {
        TracingTask {
            span,
            future: Box::pin(fut),
            is_long_lived: true
        }
    }

    pub fn new_short_lived<T: Future<Output=Result<R>> + Send + 'a>(span: Span, fut: T) -> TracingTask<'a, R> {
        TracingTask {
            span,
            future: Box::pin(fut),
            is_long_lived: false
        }
    }
}

impl<'a, R: Send + Sync + 'a> TracingTask<'a, R> {
    pub fn instrument(self) -> PinnedFut<'a, R> {
        let span = self.span;
        let future = self.future;
        let is_long_lived = self.is_long_lived;

        let fut_wrap = async move {
            if is_long_lived {
                info!("Starting...");
            }
            let t = Instant::now();

            let r = future.await;
            if r.is_err() {
                let err = r.err().unwrap();
                error!(error = ?err, elapsed = ?t.elapsed(), "Finished with");
                return Err(err);
            }
            info!(elapsed = ?t.elapsed(), "Finished [OK]...");
            Ok(r.unwrap())
        };

        Box::pin(fut_wrap.instrument(span))
    }
}

pub fn clean_fn(s: &str) -> String {
    let s = String::from(s);
    let name = s.split("::")
        .collect::<Vec<&str>>()
        .into_iter().rev()
        .take(2).rev()
        .collect::<Vec<&str>>()
        .join("::");

    let mut final_name = String::from("");
    let mut skip = 0;
    for c in name.chars() {
        if c == '<' {
            skip += 1;
        } else if c == '>' {
            skip -= 1;
        } else if skip < 1 {
            final_name.push(c);
        }
    }
    final_name
}

#[macro_export]
macro_rules! function {
    () => {{
        fn f() {}
        fn type_name_of<T>(_: T) -> &'static str {
            std::any::type_name::<T>()
        }
        let name = type_name_of(f);
        &name[..name.len() - 3]
    }}
}

#[macro_export]
macro_rules! span {
    ($($tts:tt)*) => {
        tracing::span!(tracing::Level::ERROR, "task", name = $crate::clean_fn($crate::function!()).as_str(), $($tts)*);
    };
    ($name:expr) => {
        tracing::span!(tracing::Level::ERROR, "task", name = $name);
    };
    () => {
        tracing::span!(tracing::Level::ERROR, "task", name = $crate::clean_fn($crate::function!()).as_str());
    };
}