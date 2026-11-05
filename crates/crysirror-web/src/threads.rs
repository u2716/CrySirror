// SPDX-License-Identifier: MPL-2.0
//! Shared-memory CPU workers; the presenter keeps exclusive GPU ownership.
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc,
};
use wasm_bindgen::prelude::*;

/// Start real shared engine workers and verify two independent tasks overlap
/// on distinct worker threads before publishing the host as ready.
#[wasm_bindgen(js_name = initCpuWorkers)]
pub async fn init_cpu_workers(count: usize) -> Result<String, JsValue> {
    if !(2..=16).contains(&count) {
        return Err(js_sys::Error::new("CPU worker count must be 2..16").into());
    }
    wasm_bindgen_futures::JsFuture::from(wasm_bindgen_rayon::init_thread_pool(count)).await?;
    crysirror_tasks::initialize_browser_workers(count)
        .map_err(|e| js_sys::Error::new(&e.to_string()))?;
    let pool = crysirror_tasks::TaskPool::new("browser-verify", 2, 2)
        .map_err(|e| js_sys::Error::new(&e.to_string()))?;
    let started = Arc::new(AtomicUsize::new(0));
    let mut jobs = Vec::new();
    for bit in [1, 2] {
        let started = started.clone();
        jobs.push(
            pool.spawn(move |cancel| {
                let index = rayon::current_thread_index()
                    .ok_or_else(|| anyhow::anyhow!("CPU job did not run on a parallel worker"))?;
                started.fetch_or(bit, Ordering::AcqRel);
                let start = web_time::Instant::now();
                while started.load(Ordering::Acquire) != 3 {
                    cancel.check()?;
                    anyhow::ensure!(
                        start.elapsed().as_secs_f64() < 3.,
                        "independent CPU jobs did not overlap"
                    );
                    std::hint::spin_loop();
                }
                Ok(index)
            })
            .map_err(|e| js_sys::Error::new(&e.to_string()))?,
        );
    }
    let left = jobs
        .remove(0)
        .await
        .map_err(|e| js_sys::Error::new(&e.to_string()))?;
    let right = jobs
        .remove(0)
        .await
        .map_err(|e| js_sys::Error::new(&e.to_string()))?;
    if left == right {
        return Err(js_sys::Error::new("CPU jobs used the same worker").into());
    }
    verify_queue_contract()
        .await
        .map_err(|e| js_sys::Error::new(&e.to_string()))?;
    Ok(
        serde_json::json!({"workers":count,"overlap":true,"worker_indices":[left,right],"fifo":true,"backpressure":true,"cancellation":true})
            .to_string(),
    )
}

async fn verify_queue_contract() -> anyhow::Result<()> {
    let pool = crysirror_tasks::TaskPool::new("browser-fifo", 1, 3)?;
    let gate = Arc::new(AtomicBool::new(false));
    let sequence = Arc::new(AtomicUsize::new(0));
    let mut jobs = Vec::new();
    for expected in 0..3 {
        let gate = gate.clone();
        let sequence = sequence.clone();
        jobs.push(pool.spawn(move |cancel| {
            let start = web_time::Instant::now();
            while expected == 0 && !gate.load(Ordering::Acquire) {
                cancel.check()?;
                anyhow::ensure!(
                    start.elapsed().as_secs_f64() < 3.,
                    "FIFO verification timed out"
                );
                std::hint::spin_loop();
            }
            anyhow::ensure!(
                sequence.fetch_add(1, Ordering::AcqRel) == expected,
                "single-worker pool reordered jobs"
            );
            Ok(())
        })?);
    }
    anyhow::ensure!(
        pool.reserve()
            .err()
            .is_some_and(|e| e.is::<crysirror_tasks::CapacityExceeded>()),
        "browser backpressure failed"
    );
    gate.store(true, Ordering::Release);
    for job in jobs {
        job.await?;
    }
    let gate = Arc::new(AtomicBool::new(false));
    let opened = gate.clone();
    let first = pool.spawn(move |cancel| {
        let start = web_time::Instant::now();
        while !opened.load(Ordering::Acquire) {
            cancel.check()?;
            anyhow::ensure!(
                start.elapsed().as_secs_f64() < 3.,
                "cancellation verification timed out"
            );
            std::hint::spin_loop();
        }
        Ok(())
    })?;
    let ran = Arc::new(AtomicBool::new(false));
    let observed = ran.clone();
    let cancelled = pool.spawn(move |_| {
        observed.store(true, Ordering::Release);
        Ok(())
    })?;
    cancelled.cancel();
    gate.store(true, Ordering::Release);
    first.await?;
    anyhow::ensure!(
        cancelled.await.is_err() && !ran.load(Ordering::Acquire),
        "cancelled browser work executed"
    );
    Ok(())
}
