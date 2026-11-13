use std::time::Duration;

use anyhow::{Context, Result, bail};
use btleplug::{
    api::{Central, CharPropFlags, Manager as _, Peripheral as _, ScanFilter},
    platform::Manager,
};
use futures::StreamExt;
use tokio::time::sleep;
use uuid::Uuid;

/// Smart Band 11の広告名
const TARGET_NAME: &str = "Xiaomi Smart Band 11";

/// Heart Rate Measurement: 0x2A37
const HR_MEASUREMENT_UUID: Uuid = Uuid::from_u128(0x00002a37_0000_1000_8000_00805f9b34fb);

/// Heart Rate Measurementのバイト列をBPMに変換
fn decode_heart_rate(data: &[u8]) -> Option<u16> {
    let flags = *data.first()?;

    // bit 0: 0 = uint8, 1 = uint16
    if flags & 0x01 != 0 {
        let bpm_bytes = data.get(1..3)?;
        Some(u16::from_le_bytes([bpm_bytes[0], bpm_bytes[1]]))
    } else {
        Some(u16::from(*data.get(1)?))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("Smart Band 11を検索中...");

    // 1. Bluetoothアダプターを取得
    let manager = Manager::new().await?;

    let adapters = manager.adapters().await?;

    let Some(adapter) = adapters.into_iter().next() else {
        bail!("Bluetoothアダプターが見つかりません");
    };

    // 2. BLEスキャン開始
    adapter.start_scan(ScanFilter::default()).await?;

    println!("BLEスキャン開始（10秒）...");
    sleep(Duration::from_secs(10)).await;

    // 3. Smart Band 11を探す
    let peripherals = adapter.peripherals().await?;
    let mut target = None;

    for peripheral in peripherals {
        let properties = peripheral.properties().await?;

        let Some(properties) = properties else {
            continue;
        };

        let name = properties.local_name.unwrap_or_default();

        println!(
            "検出: {} ({})",
            if name.is_empty() {
                "(名前なし)"
            } else {
                &name
            },
            peripheral.address()
        );

        if name.contains(TARGET_NAME) {
            target = Some(peripheral);
            break;
        }
    }

    adapter.stop_scan().await?;

    let Some(peripheral) = target else {
        bail!("Smart Band 11が見つかりません");
    };

    println!("Smart Band 11を発見！");
    println!("アドレス: {}", peripheral.address());

    // 4. BLE接続
    println!("接続中...");

    peripheral.connect().await.context("BLE接続失敗")?;

    println!("接続成功: {}", peripheral.is_connected().await?);

    // 5. GATTサービスを探索
    peripheral.discover_services().await?;

    println!("\nGATT Characteristics:");

    let characteristics = peripheral.characteristics();

    for characteristic in &characteristics {
        println!(
            "UUID: {} / Properties: {:?}",
            characteristic.uuid, characteristic.properties
        );
    }

    // 6. Heart Rate Measurementを取得
    let hr_characteristic = characteristics
        .iter()
        .find(|characteristic| {
            characteristic.uuid == HR_MEASUREMENT_UUID
                && characteristic.properties.contains(CharPropFlags::NOTIFY)
        })
        .context("Heart Rate Measurementが見つかりません")?;

    println!("\n心拍数通知を購読します...");

    // 7. 通知ストリームを購読
    peripheral.subscribe(hr_characteristic).await?;

    let mut notifications = peripheral.notifications().await?;

    println!("受信待機中（Ctrl+Cで終了）\n");

    let ctrl_c = tokio::signal::ctrl_c();
    tokio::pin!(ctrl_c);

    loop {
        tokio::select! {
            notification = notifications.next() => {
                let Some(notification) = notification else {
                    println!("通知ストリームが終了しました");
                    break;
                };

                if notification.uuid != HR_MEASUREMENT_UUID {
                    continue;
                }

                match decode_heart_rate(&notification.value) {
                    Some(bpm) => {
                        println!("Heart Rate: {bpm} BPM");
                    }
                    None => {
                        eprintln!(
                            "心拍数データの解析に失敗: {:02X?}",
                            notification.value
                        );
                    }
                }
            }

            result = &mut ctrl_c => {
                result.context("Ctrl+Cの検知に失敗")?;
                println!("\n終了します...");
                break;
            }
        }
    }

    // 8. 切断処理
    peripheral.unsubscribe(hr_characteristic).await?;
    peripheral.disconnect().await?;

    println!("切断しました");

    Ok(())
}
