import asyncio
from bleak import BleakScanner, BleakClient

HR_SERVICE = "0000180d-0000-1000-8000-00805f9b34fb"
HR_MEASUREMENT = "00002a37-0000-1000-8000-00805f9b34fb"


def on_heart_rate(sender, data: bytearray):
    if len(data) < 2:
        print("不正なデータ:", data.hex())
        return

    flags = data[0]

    # bit 0: 0=uint8, 1=uint16
    if flags & 0x01:
        if len(data) < 3:
            return
        bpm = int.from_bytes(data[1:3], "little")
    else:
        bpm = data[1]

    print(f"Heart Rate: {bpm} BPM")


async def main():
    print("Smart Band 11を検索中...")

    devices = await BleakScanner.discover(
        timeout=15,
        return_adv=True,
    )

    target = None

    for address, (device, adv) in devices.items():
        uuids = [u.lower() for u in adv.service_uuids]
        name = adv.local_name or device.name or ""

        if HR_SERVICE in uuids and "Xiaomi Smart Band 11" in name:
            target = device
            break

    if target is None:
        print("Smart Band 11が見つかりません")
        return

    print(f"発見: {target.name} ({target.address})")
    print("接続中...")

    async with BleakClient(target, timeout=20) as client:
        print("接続:", client.is_connected)

        # GATTサービスを探索
        for service in client.services:
            print(f"Service: {service.uuid}")
            for char in service.characteristics:
                print(f"  Characteristic: {char.uuid}")
                print(f"  Properties: {char.properties}")

        # 心拍数Characteristicを取得
        characteristic = next(
            (
                char
                for service in client.services
                for char in service.characteristics
                if char.uuid.lower() == HR_MEASUREMENT
            ),
            None,
        )

        if characteristic is None:
            print("Heart Rate Measurementが見つかりません")
            return

        print("\n心拍数通知を購読します...")

        await client.start_notify(
            characteristic,
            on_heart_rate,
        )

        print("受信待機中（Ctrl+Cで終了）")

        try:
            while True:
                await asyncio.sleep(1)
        finally:
            await client.stop_notify(characteristic)


if __name__ == "__main__":
    asyncio.run(main())
