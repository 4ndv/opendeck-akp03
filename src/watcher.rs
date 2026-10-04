use std::time::{Duration, Instant};

use futures_lite::StreamExt;
use mirajazz::{
    device::{DeviceWatcher, list_devices},
    error::MirajazzError,
    types::{DeviceLifecycleEvent, HidDeviceInfo},
};
use openaction::OUTBOUND_EVENT_MANAGER;
use tokio::task::JoinHandle;
use tokio::time::MissedTickBehavior;
use tokio_util::sync::CancellationToken;

use crate::{
    DEVICES, SPAWNING, TOKENS, TRACKER,
    device::device_task,
    mappings::{CandidateDevice, DEVICE_NAMESPACE, Kind, QUERIES},
};

/// How often the watcher re-scans for devices that are connected at the OS level but
/// not currently managed. This is the self-healing path: it recovers devices orphaned
/// by a missed hotplug event or by a panic in the event stream (see `spawn_event_stream`).
const RESCAN_INTERVAL: Duration = Duration::from_secs(3);

/// Floor delay before recreating the event stream after it ends.
const STREAM_RESTART_DELAY: Duration = Duration::from_secs(1);

/// Ceiling for the event-stream restart backoff.
const MAX_RESTART_DELAY: Duration = Duration::from_secs(30);

fn serial_to_id(serial: &String) -> String {
    format!("{}-{}", DEVICE_NAMESPACE, serial)
}

fn device_info_to_candidate(dev: HidDeviceInfo) -> Option<CandidateDevice> {
    let id = serial_to_id(&dev.serial_number.clone()?);
    let kind = Kind::from_vid_pid(dev.vendor_id, dev.product_id)?;

    Some(CandidateDevice { id, dev, kind })
}

/// Returns devices that matches known pid/vid pairs
async fn get_candidates() -> Result<Vec<CandidateDevice>, MirajazzError> {
    log::debug!("Looking for candidate devices");

    let mut candidates: Vec<CandidateDevice> = Vec::new();

    for dev in list_devices(&QUERIES).await? {
        if let Some(candidate) = device_info_to_candidate(dev.clone()) {
            candidates.push(candidate);
        } else {
            continue;
        }
    }

    Ok(candidates)
}

/// RAII guard that releases a device's entry in [`SPAWNING`] when dropped.
///
/// Owned by the managing `device_task`'s task, so the entry is released on *every* exit
/// path: normal return, a panic inside `device_task`, or the future being dropped
/// mid-flight (e.g. runtime shutdown). This is what keeps recovery robust even if
/// `device_task` panics — without it, a panicked task would leak its id in `SPAWNING`
/// forever and the device could never be re-spawned.
struct SpawnGuard {
    id: String,
}

impl Drop for SpawnGuard {
    fn drop(&mut self) {
        SPAWNING.lock().unwrap().remove(&self.id);
    }
}

/// Spawns a [`device_task`] for `candidate` unless one is already managing it.
///
/// Returns `true` if a new task was spawned.
///
/// This is the single idempotent entry point for starting device management. Both the
/// hotplug event stream and the periodic rescan go through here, so they can never
/// double-spawn the same physical device. The `SPAWNING` set is cleared by the task
/// itself when it exits (success or failure), so recovery from a missed event or a
/// watcher panic is automatic.
async fn spawn_device_if_unmanaged(candidate: CandidateDevice) -> bool {
    {
        let mut spawning = SPAWNING.lock().unwrap();
        if spawning.contains(&candidate.id) {
            return false;
        }
        spawning.insert(candidate.id.clone());
    }

    log::info!("Spawning device task for {:?}", candidate);

    let token = CancellationToken::new();
    TOKENS
        .write()
        .await
        .insert(candidate.id.clone(), token.clone());

    let tracker = TRACKER.lock().await.clone();
    tracker.spawn(async move {
        let _guard = SpawnGuard {
            id: candidate.id.clone(),
        };
        device_task(candidate, token).await;
    });

    true
}

/// Re-scans the OS for supported devices and spawns tasks for any present but unmanaged
/// ones. Self-healing path: recovers devices orphaned by a missed hotplug event or an
/// event-stream panic.
async fn rescan_and_spawn() -> Result<(), MirajazzError> {
    for candidate in get_candidates().await? {
        spawn_device_if_unmanaged(candidate).await;
    }

    Ok(())
}

/// Handles a single hotplug lifecycle event from mirajazz's [`DeviceWatcher`].
async fn handle_lifecycle_event(ev: DeviceLifecycleEvent) {
    log::info!("New device event: {:?}", ev);

    match ev {
        DeviceLifecycleEvent::Connected(info) => {
            if let Some(candidate) = device_info_to_candidate(info) {
                spawn_device_if_unmanaged(candidate).await;
            }
        }
        DeviceLifecycleEvent::Disconnected(info) => {
            let id = serial_to_id(&info.serial_number.unwrap());

            if let Some(token) = TOKENS.write().await.remove(&id) {
                log::info!("Sending cancel request for {}", id);
                token.cancel();
            }

            DEVICES.write().await.remove(&id);

            if let Some(outbound) = OUTBOUND_EVENT_MANAGER.lock().await.as_mut() {
                outbound.deregister_device(id.clone()).await.ok();
            }

            log::info!("Disconnected device {}", id);
        }
    }
}

/// Spawns a task that drives the mirajazz hotplug event stream.
///
/// `mirajazz` 0.16.2's `DeviceWatcher::watch` can panic inside its `Connected` handler
/// (`query_devices(...).await.unwrap()` at `device.rs:139`) when a device is reconnected
/// faster than `async-hid` can fully re-query it. That panic ends this task; the caller
/// supervises the returned [`JoinHandle`] and recreates the stream, so hotplug detection
/// recovers instead of going silent (which previously orphaned the device until restart).
fn spawn_event_stream() -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut watcher = DeviceWatcher::new();

        let mut stream = match watcher.watch(&QUERIES).await {
            Ok(stream) => stream,
            Err(err) => {
                log::warn!("DeviceWatcher::watch failed: {err}");
                return;
            }
        };

        log::info!("Device event stream is ready");

        while let Some(ev) = stream.next().await {
            handle_lifecycle_event(ev).await;
        }

        log::info!("Device event stream ended");
    })
}

pub async fn watcher_task(token: CancellationToken) -> Result<(), MirajazzError> {
    log::info!("Watcher is starting");

    // Initial enumeration of already-connected devices. A failure here (e.g. the HID
    // backend not yet ready at plugin start) must not abort the watcher: the periodic
    // rescan below will retry shortly.
    if let Err(err) = rescan_and_spawn().await {
        log::warn!("Initial device rescan failed: {err}");
    }

    let mut poll = tokio::time::interval(RESCAN_INTERVAL);
    poll.set_missed_tick_behavior(MissedTickBehavior::Delay);
    // Discard the immediate first tick; we just enumerated above.
    poll.tick().await;

    // Supervised event stream: recreated if it ends or panics (see `spawn_event_stream`).
    let mut event_task = spawn_event_stream();
    let mut spawned_at = Instant::now();
    let mut backoff = STREAM_RESTART_DELAY;

    loop {
        tokio::select! {
            _ = token.cancelled() => {
                event_task.abort();
                break;
            }
            _ = poll.tick() => {
                if let Err(err) = rescan_and_spawn().await {
                    log::warn!("Periodic device rescan failed: {err}");
                }
            }
            joined = &mut event_task => {
                match joined {
                    Ok(()) => log::info!("Device event stream ended, restarting"),
                    Err(err) if err.is_panic() => {
                        log::error!("Device event stream panicked, restarting: {err}");
                    }
                    Err(err) => log::warn!("Device event stream task failed, restarting: {err}"),
                }

                // If the stream outlived the current backoff, treat it as relatively
                // healthy and reset the delay; otherwise grow it to avoid flooding the
                // log if the watcher keeps failing.
                if spawned_at.elapsed() >= backoff {
                    backoff = STREAM_RESTART_DELAY;
                } else {
                    backoff = (backoff * 2).min(MAX_RESTART_DELAY);
                }

                // Race the restart delay against cancellation so shutdown stays prompt.
                tokio::select! {
                    _ = token.cancelled() => break,
                    _ = tokio::time::sleep(backoff) => {}
                }

                spawned_at = Instant::now();
                event_task = spawn_event_stream();
            }
        }
    }

    log::info!("Watcher is shutting down");

    Ok(())
}
