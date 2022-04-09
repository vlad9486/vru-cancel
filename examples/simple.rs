use std::time::Duration;

use tokio::{signal, sync::mpsc, time};
use tokio_stream::{wrappers::ReceiverStream, StreamExt};

use vru_cancel::{Canceler, cancelable};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    // given a stream
    // in this example it is a clock sending a tick each second
    let stream = {
        let (tx, rx) = mpsc::channel(1);
        tokio::spawn(async move {
            while let Ok(()) = tx.send(()).await {
                time::sleep(Duration::from_secs(1)).await;
            }
        });
        ReceiverStream::new(rx)
    };

    // spawn a task with canceler, obtain trigger
    let trigger = Canceler::spawn(move |canceler| {
        tokio::spawn(async move {
            let mut t = 0;
            cancelable!(stream, canceler);
            while let Some(()) = stream.next().await {
                println!("tick {t}");
                t += 1;
            }
            println!("stream stopped");
        })
    });

    signal::ctrl_c().await.expect("failed to wait ctrlc");
    println!(" ... terminating");

    // the trigger will cancel the stream
    let handle = trigger();

    // and return join handle which we can await
    handle.await.unwrap();
}
