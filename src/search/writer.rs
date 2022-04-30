use std::thread;
use std::time::Duration;
use flume::RecvTimeoutError;
use anyhow::{anyhow, Result};
use tantivy::{Document, Index, IndexWriter, Term};
use tokio::sync::oneshot;
use std::thread::JoinHandle;

const MEMORY_ARENA: usize = 300 << 20;
const AUTO_COMMIT_SECS: u64 = 15;

pub async fn start_writer(index: Index) -> Result<Writer> {
    let (tx, rx) = flume::bounded(4);
    let handle = thread::spawn(move || run_writer(index, rx));

    let (waker, ack) = oneshot::channel();
    if let Err(_) = tx.send_async(WriterOp::__Ping(waker)).await {
        handle
            .join()
            .expect("Join correctly")?;

        // Should never happen theoretically as our rx will only be
        // dropped if the thread died unexpectedly.
        return Err(anyhow!("Failed to start writer due to unknown error."));
    };

    if let Err(_) = ack.await {
        handle
            .join()
            .expect("Join correctly")?;

        // Should never happen theoretically as our rx will only be
        // dropped if the thread died unexpectedly.
        return Err(anyhow!("Failed to start writer due to unknown error."));
    };

    Ok(Writer {
        tx,
        handle,
    })
}

pub struct Writer {
    tx: flume::Sender<WriterOp>,
    handle: JoinHandle<Result<()>>,
}

pub enum WriterOp {
    AddDocuments(Vec<Document>),
    RemoveDocuments(Term),
    ClearAll,

    /// A simple Ping to check if the worker is alive still after creation.
    __Ping(oneshot::Sender<()>),
}

fn run_writer(index: Index, tasks: flume::Receiver<WriterOp>) -> anyhow::Result<()> {
    let mut writer = index.writer(MEMORY_ARENA)?;
    let mut op_since_last_commit = false;

    loop {
        if !op_since_last_commit {
            info!("parking writer until new events present");
            if let Ok(op) = tasks.recv() {
                op_since_last_commit = true;
                handle_message(op, &mut writer);
            } else {
                info!("writer actor channel dropped, shutting down...");
                break;
            }

            continue;
        }

        match tasks.recv_timeout(Duration::from_secs(AUTO_COMMIT_SECS)) {
            Err(RecvTimeoutError::Timeout) => {
                info!("running auto commit");

                writer.commit()?;
                op_since_last_commit = false;
            },
            Err(RecvTimeoutError::Disconnected) => {
                info!("writer actor channel dropped, shutting down...");
                break;
            },
            Ok(op) => {
                handle_message(op, &mut writer);
            },
        }
    }

    writer.commit()?;
    writer.wait_merging_threads()?;

    Ok(())
}

fn handle_message(op: WriterOp, writer: &mut IndexWriter) -> anyhow::Result<()> {
    match op {
        WriterOp::__Ping(waker) => {
            let _ = waker.send(());
        },
        WriterOp::AddDocuments(docs) => {
            for doc in docs {
                writer.add_document(doc)?;
            }
        },
        WriterOp::RemoveDocuments(term) => {
            writer.delete_term(term);
        },
        WriterOp::ClearAll  => {
            writer.delete_all_documents()?;
        },
    };

    Ok(())
}