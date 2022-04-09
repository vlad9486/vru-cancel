// Copyright 2021 Vladislav Melnik
// SPDX-License-Identifier: MIT

#![deny(clippy::all)]
#![warn(clippy::pedantic)]
// #![warn(clippy::restriction)]
#![warn(clippy::nursery)]
#![warn(clippy::cargo)]
// nursery
#![allow(clippy::use_self)]

use std::{
    pin::Pin,
    task::{Context, Poll},
    future::Future,
};

use tokio::sync::watch;
use tokio_stream::Stream;

pub struct CancelToken(());

pin_project_lite::pin_project! {
    /// The stream that end when `fut` is ready.
    #[must_use]
    pub struct Cancelable<S, F> {
        #[pin]
        pub stream: S,
        #[pin]
        pub fut: F,
    }
}

impl<S, F> Stream for Cancelable<S, F>
where
    S: Stream,
    F: Future<Output = CancelToken>,
{
    type Item = S::Item;

    #[inline]
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();

        if let Poll::Ready(CancelToken(())) = this.fut.poll(cx) {
            Poll::Ready(None)
        } else {
            this.stream.poll_next(cx)
        }
    }
}

#[derive(Clone)]
pub struct Canceler(watch::Receiver<bool>);

impl Canceler {
    /// Execute a closure immediately with `Canceler`.
    /// Return a closure which cancels all streams.
    #[inline]
    pub fn spawn<F, T>(f: F) -> impl FnOnce() -> T
    where
        F: FnOnce(Self) -> T,
    {
        let (tx, rx) = watch::channel(false);
        let output = f(Canceler(rx));

        move || {
            if tx.send(true).is_err() {
                // canceler already dropped, nothing to cancel
            }
            output
        }
    }

    /// Create a cancel token. Use `cancelable` macros instead.
    #[inline]
    pub async fn cancel(&mut self) -> CancelToken {
        while !*self.0.borrow() {
            // value is currently false; wait for it to change
            if self.0.changed().await.is_err() {
                // channel was closed
                break;
            }
        }
        CancelToken(())
    }
}

/// Make the stream cancelable.
#[macro_export]
macro_rules! cancelable {
    ($stream:ident, $canceler:expr) => {
        // TODO: make `_canceler` and `_fut` anonymous to avoid accidental eclipse
        let mut _canceler = $canceler.clone();
        let _fut = _canceler.cancel();
        tokio::pin!(_fut);
        let mut $stream = vru_cancel::Cancelable {
            stream: $stream,
            fut: _fut,
        };
    };
}
