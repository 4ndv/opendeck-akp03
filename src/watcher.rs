use futures_lite::StreamExt;
use mirajazz::{
    device::{DeviceWatcher, list_devices},
    error::MirajazzError,
    types::{DeviceLifecycleEvent, HidDeviceInfo},
};
use openaction::OUTBOUND_EVENT_MANAGER;
use tokio_util::sync::CancellationToken;

use crate::{
    DEVICES, TOKENS, TRACKER,
    device::device_task,
    mappings::{CandidateDevice, DEVICE_NAMESPACE, Kind, MIRABOX_VID, N3_PID, QUERIES},
};

fn device_id(dev: &HidDeviceInfo) -> Option<String> {
    let serial = dev.serial_number.as_ref()?;

    // OpenDeck profile stores are keyed by device id. If we change device capabilities (e.g. encoder count),
    // an existing profile store can become incompatible and silently break encoder actions.
    //
    // For the Mirabox VID (0x6603) + N3 PID (0x1002) batches, keep the namespace prefix ("n3") but use a
    // distinct id so OpenDeck creates a fresh, correctly sized profile store.
    if dev.vendor_id == MIRABOX_VID && dev.product_id == N3_PID {
        Some(format!("{}-6603-{}", DEVICE_NAMESPACE, serial))
    } else {
        Some(format!("{}-{}", DEVICE_NAMESPACE, serial))
    }
}

fn device_info_to_candidate(dev: HidDeviceInfo) -> Option<CandidateDevice> {
    let id = device_id(&dev)?;
    let kind = Kind::from_vid_pid(dev.vendor_id, dev.product_id)?;

    Some(CandidateDevice { id, dev, kind })
}

/// Returns devices that matches known pid/vid pairs
async fn get_candidates() -> Result<Vec<CandidateDevice>, MirajazzError> {
    log::info!("Looking for candidate devices");

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

pub async fn watcher_task(token: CancellationToken) -> Result<(), MirajazzError> {
    let tracker = TRACKER.lock().await.clone();

    // Scans for connected devices that (possibly) we can use
    let candidates = get_candidates().await?;

    log::info!("Looking for connected devices");

    for candidate in candidates {
        log::info!("New candidate {:#?}", candidate);

        let token = CancellationToken::new();

        TOKENS
            .write()
            .await
            .insert(candidate.id.clone(), token.clone());

        tracker.spawn(device_task(candidate, token));
    }

    let mut watcher = DeviceWatcher::new();
    let mut watcher_stream = watcher.watch(&QUERIES).await?;

    log::info!("Watcher is ready");

    loop {
        let ev = tokio::select! {
            v = watcher_stream.next() => v,
            _ = token.cancelled() => None
        };

        if let Some(ev) = ev {
            log::info!("New device event: {:?}", ev);

            match ev {
                DeviceLifecycleEvent::Connected(info) => {
                    if let Some(candidate) = device_info_to_candidate(info) {
                        // Don't add existing device again
                        if DEVICES.read().await.contains_key(&candidate.id) {
                            continue;
                        }

                        let token = CancellationToken::new();

                        TOKENS
                            .write()
                            .await
                            .insert(candidate.id.clone(), token.clone());

                        log::debug!("Spawning task for new device: {:?}", candidate);
                        tracker.spawn(device_task(candidate, token));
                        log::debug!("Spawned");
                    }
                }
                DeviceLifecycleEvent::Disconnected(info) => {
                    let Some(id) = device_id(&info) else {
                        continue;
                    };

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
        } else {
            log::info!("Watcher is shutting down");

            break Ok(());
        }
    }
}
